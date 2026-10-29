// SPDX-License-Identifier: GPL-2.0-or-later
//! Bounded character previews and verified edits of decrypted Bloodborne save sets.
//! Format locations were checked against Noxde/Bloodborne-save-editor d8d0365 (GPL-3.0).
//! This parser is independently written and validates the structure before exposing fields.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, String>;
const SLOT_BYTES: usize = 0x140000;
const TITLES: &[&str] = &["CUSA00207", "CUSA00900", "CUSA01363", "CUSA03173"];
const INVENTORY_MAGIC: [u8; 4] = [0x40, 0xf0, 0xff, 0xff];

#[derive(Serialize)]
pub struct Character {
    pub title_id: String,
    pub slot: u8,
    pub name: String,
    pub level: u32,
    pub vitality: u32,
    pub endurance: u32,
    pub strength: u32,
    pub skill: u32,
    pub bloodtinge: u32,
    pub arcane: u32,
    pub echoes: u32,
    pub insight: u32,
    pub playtime_ms: u32,
    pub new_game_cycle: u8,
    pub stats_consistent: bool,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct Edit {
    pub title_id: String,
    pub slot: u8,
    pub fingerprint: String,
    pub echoes: u32,
    pub insight: u32,
    pub vitality: u32,
    pub endurance: u32,
    pub strength: u32,
    pub skill: u32,
    pub bloodtinge: u32,
    pub arcane: u32,
}

fn real_dir(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(format!(
            "Save folder is not a regular directory: {}",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn slot_path(user: &Path, title: &str, slot: u8) -> Result<PathBuf> {
    if !TITLES.contains(&title) || slot > 9 {
        return Err("Unsupported Bloodborne save slot.".into());
    }
    let save = user.join("savedata");
    let account = save.join("1");
    let game = account.join(title);
    let set = game.join("SPRJ0005");
    for path in [user, &save, &account, &game, &set] {
        if !real_dir(path)? {
            return Err("The Bloodborne save folder is missing.".into());
        }
    }
    Ok(set.join(format!("userdata{slot:04}")))
}

pub(crate) fn slot_bytes(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() != SLOT_BYTES as u64 {
        return Err("Unsupported Bloodborne character file.".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() != SLOT_BYTES {
        return Err("The character file changed while it was read.".into());
    }
    Ok(bytes)
}

fn number(bytes: &[u8], base: usize, relative: isize) -> Result<u32> {
    let at = base
        .checked_add_signed(relative)
        .ok_or("Invalid character field offset.")?;
    let raw: [u8; 4] = bytes
        .get(at..at + 4)
        .ok_or("Truncated character field.")?
        .try_into()
        .map_err(|_| "Invalid character field.")?;
    Ok(u32::from_le_bytes(raw))
}

pub(crate) fn character_base(bytes: &[u8]) -> Result<Option<usize>> {
    if bytes.len() != SLOT_BYTES {
        return Err("Unsupported Bloodborne character file.".into());
    }
    // Empty slot files have no FACE block. Nonempty ones have the character's, with the
    // inventory marker at its fixed distance before it, once or twice (copies()).
    let faces = faces(bytes);
    if faces.is_empty() {
        return Ok(None);
    }
    let inventory = match copies(bytes, &faces)?.as_slice() {
        [] => return Err("Unrecognized character inventory layout.".into()),
        [one] => *one,
        [first, second] => {
            // Two copies: the current state and the previous one the game keeps. The current one
            // has the longer playtime (the u32 at offset 8 of each copy).
            let delta = second - first;
            if playtime_at(bytes, delta)? > playtime_at(bytes, 0)? { *second } else { *first }
        }
        _ => return Err("Ambiguous character appearance block.".into()),
    };
    let base = inventory
        .checked_sub(469)
        .ok_or("Invalid character layout.")?;
    Ok(Some(base))
}

/// The inventory starts of the character copies: the FACE blocks with the inventory marker at
/// their fixed distance before them, at most two, the second a whole copy of the first part of the
/// file (the game writes it after play).
fn copies(bytes: &[u8], faces: &[usize]) -> Result<Vec<usize>> {
    let anchored: Vec<usize> = faces
        .iter()
        .filter_map(|&face| face.checked_sub(34028))
        .filter(|&inventory| bytes.get(inventory..inventory + 4) == Some(INVENTORY_MAGIC.as_slice()))
        .collect();
    if let [first, second] = anchored.as_slice() {
        if second - first < *first + 34028 {
            return Err("Ambiguous character appearance block.".into());
        }
    }
    Ok(anchored)
}

/// How many copies of the character the file holds (the game sometimes keeps the previous one).
pub(crate) fn copy_count(bytes: &[u8]) -> Result<usize> {
    Ok(copies(bytes, &faces(bytes))?.len())
}

fn playtime_at(bytes: &[u8], copy: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes.get(copy + 8..copy + 12).ok_or("Truncated playtime.")?.try_into().unwrap(),
    ))
}

fn faces(bytes: &[u8]) -> Vec<usize> {
    bytes[0xf000..]
        .windows(4)
        .enumerate()
        .filter_map(|(index, window)| (window == b"FACE").then_some(index + 0xf000))
        .collect()
}

/// After an edit of the current copy: the same bytes in the other copy, when it held the same
/// values at every edited byte (then both copies agree, whichever the game reads). Otherwise the
/// other copy is the game's own previous state and stays as it is.
pub(crate) fn mirror(original: &[u8], edited: &mut [u8]) -> Result<()> {
    let found = copies(original, &faces(original))?;
    let [first, second] = found.as_slice() else {
        return Ok(());
    };
    let delta = second - first;
    let current = character_base(original)?.ok_or("The character slot is empty.")? + 469;
    let changed: Vec<usize> = (0..original.len()).filter(|&i| original[i] != edited[i]).collect();
    let target = |i: usize| if current == *first { i.checked_add(delta) } else { i.checked_sub(delta) };
    let mut writes = Vec::new();
    for &i in &changed {
        let Some(j) = target(i).filter(|&j| j < original.len()) else {
            return Err("An edit fell outside the character's copy.".into());
        };
        if changed.contains(&j) {
            return Err("An edit touched both character copies.".into());
        }
        if original[j] != original[i] {
            return Ok(());
        }
        writes.push((j, edited[i]));
    }
    for (j, value) in writes {
        edited[j] = value;
    }
    Ok(())
}

pub(crate) fn parse(bytes: &[u8], title: &str, slot: u8) -> Result<Option<Character>> {
    let Some(base) = character_base(bytes)? else {
        return Ok(None);
    };
    let name_bytes = bytes
        .get(base + 1..base + 33)
        .ok_or("Truncated character name.")?;
    let units: Vec<u16> = name_bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    let name = String::from_utf16(&units[..end]).map_err(|_| "Invalid character name.")?;
    if name.is_empty()
        || name.chars().any(char::is_control)
        || units[end..].iter().any(|&unit| unit != 0)
    {
        return Err("Invalid character name.".into());
    }
    let vitality = number(bytes, base, -103)?;
    let endurance = number(bytes, base, -95)?;
    let strength = number(bytes, base, -79)?;
    let skill = number(bytes, base, -71)?;
    let bloodtinge = number(bytes, base, -63)?;
    let arcane = number(bytes, base, -55)?;
    let level = number(bytes, base, -23)?;
    let attributes = [vitality, endurance, strength, skill, bloodtinge, arcane];
    if !attributes.iter().all(|&value| (1..=99).contains(&value)) || !(4..=544).contains(&level) {
        return Err("Invalid character attributes.".into());
    }
    let sum: u32 = attributes.iter().sum();
    let stats_consistent = sum.checked_sub(50) == Some(level);
    let new_game_cycle = *bytes
        .get(base + 68831)
        .ok_or("Truncated character progression.")?;
    let copy = copies(bytes, &faces(bytes))?;
    let delta = match copy.as_slice() {
        [first, second] if base + 469 == *second => second - first,
        _ => 0,
    };
    let playtime_ms = playtime_at(bytes, delta)?;
    Ok(Some(Character {
        title_id: title.into(),
        slot,
        name,
        level,
        vitality,
        endurance,
        strength,
        skill,
        bloodtinge,
        arcane,
        echoes: number(bytes, base, -19)?,
        insight: number(bytes, base, -35)?,
        playtime_ms,
        new_game_cycle,
        stats_consistent,
        fingerprint: format!("{:x}", Sha256::digest(bytes)),
    }))
}

/// The fingerprint of a character file in a staged save set (what an edit was opened on).
pub fn fingerprint(savedata: &Path, title: &str, slot: u8) -> Result<String> {
    let user = savedata.parent().ok_or("Invalid save staging folder.")?;
    Ok(format!("{:x}", Sha256::digest(slot_bytes(&slot_path(user, title, slot)?)?)))
}

pub fn rewrite(savedata: &Path, edit: &Edit) -> Result<()> {
    if edit.echoes > 999_999_999 || edit.insight > 99 {
        return Err("Blood Echoes or Insight is outside the game range.".into());
    }
    let user = savedata.parent().ok_or("Invalid save staging folder.")?;
    let path = slot_path(user, &edit.title_id, edit.slot)?;
    let mut bytes = slot_bytes(&path)?;
    let original = bytes.clone();
    let before = parse(&bytes, &edit.title_id, edit.slot)?.ok_or("The character slot is empty.")?;
    if !before.stats_consistent || before.fingerprint != edit.fingerprint {
        return Err("The character changed since it was opened, or its stats are inconsistent. Refresh the save preview.".into());
    }
    let attributes = [
        edit.vitality,
        edit.endurance,
        edit.strength,
        edit.skill,
        edit.bloodtinge,
        edit.arcane,
    ];
    if !attributes.iter().all(|value| (1..=99).contains(value)) {
        return Err("Character attributes must be between 1 and 99.".into());
    }
    if edit.vitality != before.vitality && crate::character_growth::hp(edit.vitality).is_none() {
        return Err("Vitality must be between 7 and 99 to update health.".into());
    }
    if edit.endurance != before.endurance
        && crate::character_growth::stamina(edit.endurance).is_none()
    {
        return Err("Endurance must be between 8 and 99 to update stamina.".into());
    }
    let level = attributes
        .iter()
        .sum::<u32>()
        .checked_sub(50)
        .ok_or("The resulting character level is outside the game range.")?;
    if !(4..=544).contains(&level) {
        return Err("The resulting character level is outside the game range.".into());
    }
    if before.echoes == edit.echoes
        && before.insight == edit.insight
        && [
            before.vitality,
            before.endurance,
            before.strength,
            before.skill,
            before.bloodtinge,
            before.arcane,
        ] == attributes
    {
        return Err("No character values changed.".into());
    }
    let base = character_base(&bytes)?.ok_or("The character slot is empty.")?;
    for (offset, value) in [
        (-19, edit.echoes),
        (-35, edit.insight),
        (-103, edit.vitality),
        (-95, edit.endurance),
        (-79, edit.strength),
        (-71, edit.skill),
        (-63, edit.bloodtinge),
        (-55, edit.arcane),
        (-23, level),
    ] {
        let at = base
            .checked_add_signed(offset)
            .ok_or("Invalid character field offset.")?;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (changed, start, value) in [
        (
            edit.vitality != before.vitality,
            -147,
            crate::character_growth::hp(edit.vitality),
        ),
        (
            edit.endurance != before.endurance,
            -119,
            crate::character_growth::stamina(edit.endurance),
        ),
    ] {
        if changed {
            let value = value.ok_or("Unsupported health or stamina progression value.")?;
            // Three saved copies represent the character's current and maximum base value.
            for index in 0..3 {
                let at = base
                    .checked_add_signed(start + index * 4)
                    .ok_or("Invalid derived field offset.")?;
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
    }
    let after = parse(&bytes, &edit.title_id, edit.slot)?
        .ok_or("Character validation failed after editing.")?;
    if after.echoes != edit.echoes
        || after.insight != edit.insight
        || after.name != before.name
        || after.level != level
        || !after.stats_consistent
        || [
            after.vitality,
            after.endurance,
            after.strength,
            after.skill,
            after.bloodtinge,
            after.arcane,
        ] != attributes
    {
        return Err("Character validation failed after editing.".into());
    }
    for (changed, start, expected) in [
        (edit.vitality != before.vitality, -147, crate::character_growth::hp(edit.vitality)),
        (edit.endurance != before.endurance, -119, crate::character_growth::stamina(edit.endurance)),
    ] {
        if changed {
            let expected = expected.ok_or("Unsupported health or stamina progression value.")?;
            for index in 0..3 {
                if number(&bytes, base, start + index * 4)? != expected {
                    return Err("Character health or stamina validation failed after editing.".into());
                }
            }
        }
    }
    mirror(&original, &mut bytes)?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn list(user: &Path) -> Result<Vec<Character>> {
    let mut found = Vec::new();
    if !real_dir(&user.join("savedata"))? || !real_dir(&user.join("savedata/1"))? {
        return Ok(found);
    }
    for title in TITLES {
        let set = user.join("savedata/1").join(title).join("SPRJ0005");
        if !real_dir(&user.join("savedata/1").join(title))? || !real_dir(&set)? {
            continue;
        }
        for slot in 0..10 {
            let path = slot_path(user, title, slot)?;
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.to_string()),
                Ok(_) => {}
            }
            if let Some(character) = parse(&slot_bytes(&path)?, title, slot)? {
                found.push(character);
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut bytes = vec![0; SLOT_BYTES];
        let base: usize = 0xc000;
        let inventory = base + 469;
        let face = inventory + 34028;
        bytes[inventory..inventory + 4].copy_from_slice(&INVENTORY_MAGIC);
        bytes[face..face + 4].copy_from_slice(b"FACE");
        for (i, unit) in "Hunter".encode_utf16().enumerate() {
            bytes[base + 1 + i * 2..base + 3 + i * 2].copy_from_slice(&unit.to_le_bytes());
        }
        for (relative, value) in [
            (-147, 616_u32),
            (-143, 616),
            (-139, 616),
            (-119, 93),
            (-115, 93),
            (-111, 93),
            (-103, 12_u32),
            (-95, 11),
            (-79, 22),
            (-71, 9),
            (-63, 6),
            (-55, 7),
            (-23, 17),
            (-19, 100),
            (-35, 3),
        ] {
            let at = base.checked_add_signed(relative).unwrap();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn parses_only_a_bounded_character_slot() {
        let mut bytes = sample();
        let face = 0xc000 + 469 + 34028;
        let character = parse(&bytes, "CUSA03173", 0).unwrap().unwrap();
        assert_eq!(character.name, "Hunter");
        assert_eq!(character.level, 17);
        assert!(character.stats_consistent);
        // Another appearance record later in the file (the game writes one) is not the anchor.
        bytes[face + 0x1000..face + 0x1004].copy_from_slice(b"FACE");
        assert_eq!(parse(&bytes, "CUSA03173", 0).unwrap().unwrap().name, "Hunter");
        // Two blocks that both have the inventory marker before them are ambiguous.
        let other = face + 0x2000;
        bytes[other..other + 4].copy_from_slice(b"FACE");
        bytes[other - 34028..other - 34024].copy_from_slice(&INVENTORY_MAGIC);
        assert!(parse(&bytes, "CUSA03173", 0).is_err());
        assert!(parse(&bytes[..100], "CUSA03173", 0).is_err());
    }

    #[test]
    fn edits_reach_both_copies_when_they_agree() {
        const DELTA: usize = 0xd2090;
        let mut two = sample();
        let (first, rest) = two.split_at_mut(DELTA);
        rest[..0x20000].copy_from_slice(&first[..0x20000]);
        two[8..12].copy_from_slice(&2000u32.to_le_bytes()); // the first copy is the current one
        two[DELTA + 8..DELTA + 12].copy_from_slice(&1000u32.to_le_bytes());
        let character = parse(&two, "CUSA03173", 0).unwrap().unwrap();
        assert_eq!(character.playtime_ms, 2000);
        let base = 0xc000;
        let mut edited = two.clone();
        edited[base - 19..base - 15].copy_from_slice(&500u32.to_le_bytes());
        mirror(&two, &mut edited).unwrap();
        assert_eq!(number(&edited, base + DELTA, -19).unwrap(), 500);
        // A previous copy that differs where the edit is stays as the game left it.
        let mut diverged = two.clone();
        diverged[base + DELTA - 19] = 7;
        let mut edited = diverged.clone();
        edited[base - 19..base - 15].copy_from_slice(&500u32.to_le_bytes());
        mirror(&diverged, &mut edited).unwrap();
        assert_eq!(edited[base + DELTA - 19], 7);
        // When the second copy is the current one, it is the one read.
        two[DELTA + 8..DELTA + 12].copy_from_slice(&3000u32.to_le_bytes());
        two[base + DELTA - 19..base + DELTA - 15].copy_from_slice(&42u32.to_le_bytes());
        let character = parse(&two, "CUSA03173", 0).unwrap().unwrap();
        assert_eq!((character.playtime_ms, character.echoes), (3000, 42));
    }

    #[test]
    fn edit_keeps_clean_backup_and_rejects_stale_preview() {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "bb-editor-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let original = root.join("user");
        let set = original.join("savedata/1/CUSA03173/SPRJ0005");
        fs::create_dir_all(&set).unwrap();
        fs::write(set.join("userdata0000"), sample()).unwrap();
        let before = list(&original).unwrap().remove(0);
        let edit = Edit {
            title_id: before.title_id.clone(),
            slot: before.slot,
            fingerprint: before.fingerprint.clone(),
            echoes: 999_999_999,
            insight: 99,
            vitality: 99,
            endurance: 99,
            strength: 99,
            skill: 99,
            bloodtinge: 99,
            arcane: 99,
        };
        crate::saves::edit(&root, &original, "original", |savedata| {
            rewrite(savedata, &edit)
        })
        .unwrap();
        let after = list(&original).unwrap().remove(0);
        assert_eq!((after.echoes, after.insight), (999_999_999, 99));
        assert_eq!(
            (after.vitality, after.endurance, after.strength, after.level),
            (99, 99, 99, 544)
        );
        let edited = fs::read(set.join("userdata0000")).unwrap();
        let base = character_base(&edited).unwrap().unwrap();
        for offset in [-147, -143, -139] {
            assert_eq!(number(&edited, base, offset).unwrap(), 1900);
        }
        for offset in [-119, -115, -111] {
            assert_eq!(number(&edited, base, offset).unwrap(), 170);
        }
        assert!(original.join("savedata/.bb-edited").is_file());
        assert!(!crate::trophies::eligible(&original));
        let backups = crate::saves::list(&root, &original)
            .unwrap()
            .remove(0)
            .backups;
        assert_eq!(backups.len(), 1);
        let backup_set = root
            .join("save-backups/original")
            .join(&backups[0].id)
            .join("savedata");
        assert!(!backup_set.join(".bb-edited").exists());
        assert_eq!(
            fs::read(backup_set.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap(),
            sample()
        );
        assert!(
            crate::saves::edit(&root, &original, "original", |savedata| rewrite(
                savedata, &edit
            ))
            .is_err()
        );
        assert_eq!(
            crate::saves::list(&root, &original)
                .unwrap()
                .remove(0)
                .backups
                .len(),
            1
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn copied_save_preview_when_provided() {
        let Ok(user) = std::env::var("BB_TEST_SAVE_USER") else {
            return;
        };
        let characters = list(Path::new(&user)).unwrap();
        assert!(!characters.is_empty());
        assert!(characters
            .iter()
            .all(|character| character.stats_consistent));
    }
}
