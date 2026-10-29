// SPDX-License-Identifier: GPL-2.0-or-later
//! Whether a folder holds the game the runtime runs: Bloodborne with the 1.09 update merged in.
//! The same check as bloodborne-runtime's scripts/game_check.py (with prepare.py's param.sfo and
//! SELF readers), so the launcher needs no Python for it (iOS has none): the edition from
//! sce_sys/param.sfo, the SHA-256 of eboot.bin's loaded image (the same whatever tool dumped it),
//! and the shaders in dvdroot_ps4/shader unpacking (a broken extraction makes the game hang while
//! it loads them). BB_SKIP_GAME_CHECK=1 skips the last two.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

/// The retail releases: Bloodborne (US EU UK JP AS), then the editions with The Old Hunters
/// (US EU JP AS). The executable decides in the end: SUPPORTED_IMAGE.
const SUPPORTED_TITLES: [&str; 9] = [
    "CUSA00900", "CUSA00207", "CUSA00208", "CUSA00299", "CUSA01363",
    "CUSA03179", "CUSA03173", "CUSA03014", "CUSA03023",
];
const OLD_HUNTERS_TITLES: [&str; 4] = ["CUSA03179", "CUSA03173", "CUSA03014", "CUSA03023"];
const SUPPORTED_VERSION: &str = "01.09";
const SUPPORTED_IMAGE: &str = "071df19c8880086d97182dbc057bc8cb37badaca57d9112683836b24a0444c0a";
/// The 1.09 executable with Lance McDonald's 60 fps patch baked in: the gestures menu crashes it.
const LANCE_60FPS_IMAGE: &str = "bbc91f4dff6bc3118b039464289e39878a8fdfc802a07fab9d5fe84d935a676f";
/// The game loads these first; a broken extraction damages most files, these among them.
const CHECKED_FOLDER: &str = "dvdroot_ps4/shader";

type Error = String;

fn span(data: &[u8], offset: usize, size: usize) -> Result<&[u8], Error> {
    offset
        .checked_add(size)
        .filter(|&end| end <= data.len())
        .map(|end| &data[offset..end])
        .ok_or_else(|| format!("out of bounds: {offset:#x}+{size:#x} / {:#x}", data.len()))
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(span(data, offset, 2)?.try_into().unwrap()))
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(span(data, offset, 4)?.try_into().unwrap()))
}
fn u64_at(data: &[u8], offset: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(span(data, offset, 8)?.try_into().unwrap()))
}

/// param.sfo's entries: numbers (format 0x404) and texts.
fn sfo(data: &[u8]) -> Result<HashMap<String, Value>, Error> {
    if span(data, 0, 4)? != b"\0PSF" {
        return Err("bad SFO signature".into());
    }
    let (keys, values, count) = (u32_at(data, 8)? as usize, u32_at(data, 12)? as usize, u32_at(data, 16)?);
    let mut result = HashMap::new();
    for i in 0..count as usize {
        let entry = 20 + i * 16;
        let (key, format) = (u16_at(data, entry)? as usize, u16_at(data, entry + 2)?);
        let (size, offset) = (u32_at(data, entry + 4)? as usize, u32_at(data, entry + 12)? as usize);
        let start = keys + key;
        let name = span(data, start, data.len().saturating_sub(start))?;
        let name = &name[..name.iter().position(|&b| b == 0).ok_or("unterminated SFO key")?];
        let value = span(data, values + offset, size)?;
        let value = if format == 0x404 {
            json!(u32::from_le_bytes(span(value, 0, 4)?.try_into().unwrap()))
        } else {
            let end = value.iter().rposition(|&b| b != 0).map_or(0, |p| p + 1);
            json!(String::from_utf8_lossy(&value[..end]))
        };
        result.insert(String::from_utf8_lossy(name).into_owned(), value);
    }
    Ok(result)
}

struct Segment {
    kind: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
}

fn program_headers(data: &[u8], base: usize) -> Result<(usize, Vec<Segment>), Error> {
    if span(data, base, 7)? != b"\x7fELF\x02\x01\x01" || u16_at(data, base + 18)? != 62 {
        return Err("expected little-endian x86-64 ELF".into());
    }
    let (phoff, phentsize, phnum) = (u64_at(data, base + 32)? as usize, u16_at(data, base + 54)?, u16_at(data, base + 56)?);
    if phentsize != 56 || phnum == 0 || phnum >= 256 {
        return Err("unsupported program headers".into());
    }
    let headers = (0..phnum as usize)
        .map(|i| {
            let at = base + phoff + i * 56;
            Ok(Segment {
                kind: u32_at(data, at)?,
                offset: u64_at(data, at + 8)?,
                vaddr: u64_at(data, at + 16)?,
                filesz: u64_at(data, at + 32)?,
                memsz: u64_at(data, at + 40)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok((phoff + phnum as usize * 56, headers))
}

/// eboot.bin (a PS4 SELF, or a plain ELF) as its ELF file and program headers.
fn parse_self(data: &[u8]) -> Result<(Vec<u8>, Vec<Segment>), Error> {
    const LIMIT: u64 = 512 * 1024 * 1024;
    if span(data, 0, 4)? == b"\x7fELF" {
        let (headers_end, ph) = program_headers(data, 0)?;
        let end = ph.iter().map(|p| p.offset + p.filesz).max().unwrap_or(0).max(headers_end as u64);
        if end > LIMIT {
            return Err("probe image exceeds 512 MiB limit".into());
        }
        return Ok((span(data, 0, end as usize)?.to_vec(), ph));
    }
    if span(data, 0, 4)? != b"O\x15=\x1d" {
        return Err("expected PS4 SELF or ELF (eboot.bin is neither: an encrypted or damaged dump?)".into());
    }
    let count = u16_at(data, 24)? as usize;
    let base = 32 + count * 32;
    let (headers_end, ph) = program_headers(data, base)?;
    let end = ph.iter().map(|p| p.offset + p.filesz).max().unwrap_or(0);
    if end > LIMIT {
        return Err("probe image exceeds 512 MiB limit".into());
    }
    let mut elf = vec![0u8; end as usize];
    let first = headers_end.min(elf.len());
    elf[..first].copy_from_slice(span(data, base, first)?);
    let mut covered = vec![(0u64, headers_end as u64)];
    for i in 0..count {
        let at = 32 + i * 32;
        let (flags, offset, size, mem) = (u64_at(data, at)?, u64_at(data, at + 8)?, u64_at(data, at + 16)?, u64_at(data, at + 24)?);
        if flags & 0x800 == 0 {
            continue;
        }
        if flags & 10 != 0 {
            return Err("encrypted/compressed SELF segment is unsupported".into());
        }
        let p = ph.get(((flags >> 20) & 4095) as usize).ok_or("invalid segment index")?;
        if size != p.filesz || size != mem {
            return Err("unsupported blocked segment layout".into());
        }
        let target = span(&elf, p.offset as usize, size as usize)?.len();
        let from = span(data, offset as usize, size as usize)?;
        elf[p.offset as usize..p.offset as usize + target].copy_from_slice(from);
        covered.push((p.offset, p.offset + size));
    }
    for (i, p) in ph.iter().enumerate() {
        let inside = covered.iter().any(|&(a, z)| a <= p.offset && p.offset + p.filesz <= z);
        if p.filesz != 0 && !inside && matches!(p.kind, 1 | 2 | 0x6100_0000 | 0x6100_0010) {
            return Err(format!("required segment {i} is unavailable"));
        }
    }
    Ok((elf, ph))
}

/// The SHA-256 of eboot.bin's loaded image (its loadable segments at their addresses).
fn image_sha256(game: &Path) -> Result<String, Error> {
    let data = fs::read(game.join("eboot.bin")).map_err(|e| e.to_string())?;
    let (elf, ph) = parse_self(&data)?;
    let loads: Vec<&Segment> = ph.iter().filter(|p| p.kind == 1 || p.kind == 0x6100_0010).collect();
    let size = loads.iter().map(|p| p.vaddr + p.memsz).max().ok_or("no loadable segments")?;
    let mut image = vec![0u8; usize::try_from(size).map_err(|e| e.to_string())?];
    for p in loads {
        let from = span(&elf, p.offset as usize, p.filesz as usize)?;
        let to = image.get_mut(p.vaddr as usize..(p.vaddr + p.filesz) as usize).ok_or("segment outside the image")?;
        to.copy_from_slice(from);
    }
    Ok(Sha256::digest(&image).iter().map(|b| format!("{b:02x}")).collect())
}

/// True when a DFLT .dcx file does not unpack to its stated size or fails zlib's checksum. Other
/// compressions are not checked.
fn dcx_broken(path: &Path) -> Result<bool, Error> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    if data.get(..4) != Some(b"DCX\0") || data.get(0x18..0x1C) != Some(b"DCS\0") || data.get(0x28..0x2C) != Some(b"DFLT") {
        return Ok(false);
    }
    let size = u32::from_be_bytes(span(&data, 0x1C, 4)?.try_into().unwrap()) as usize;
    let packed = u32::from_be_bytes(span(&data, 0x20, 4)?.try_into().unwrap()) as usize;
    let end = (0x4C + packed).min(data.len());
    let mut out = Vec::with_capacity(size);
    match flate2::read::ZlibDecoder::new(&data[0x4C..end]).read_to_end(&mut out) {
        Ok(_) => Ok(out.len() != size),
        Err(_) => Ok(true),
    }
}

fn dcx_files(dir: &Path, found: &mut Vec<std::path::PathBuf>) -> Result<(), Error> {
    let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            dcx_files(&path, found)?;
        } else if path.extension().is_some_and(|e| e == "dcx") {
            found.push(path);
        }
    }
    Ok(())
}

fn has_broken_files(game: &Path) -> Result<bool, Error> {
    let mut files = Vec::new();
    dcx_files(&game.join(CHECKED_FOLDER), &mut files)?;
    for file in files {
        if dcx_broken(&file)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn text(info: &HashMap<String, Value>, key: &str) -> String {
    match info.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// The dump's edition: title ID, title, version, region and whether The Old Hunters is included.
fn describe(game: &Path) -> Value {
    let Ok(info) = fs::read(game.join("sce_sys/param.sfo")).map_err(|e| e.to_string()).and_then(|d| sfo(&d)) else {
        return Value::Null;
    };
    let title = text(&info, "TITLE_ID");
    let region = match text(&info, "CONTENT_ID").get(..2) {
        Some("UP") => "america",
        Some("EP") => "europe",
        Some("JP") => "japan",
        Some("HP") => "asia",
        _ => "unknown",
    };
    json!({
        "title_id": title,
        "title": text(&info, "TITLE"),
        "version": text(&info, "APP_VER"),
        "region": region,
        "old_hunters": OLD_HUNTERS_TITLES.contains(&title.as_str()),
    })
}

/// None for the supported game, else what is wrong and what to do (English, as the runtime's log).
fn problem(game: &Path) -> Option<String> {
    if std::env::var("BB_SKIP_GAME_CHECK").as_deref() == Ok("1") {
        return None;
    }
    let info = fs::read(game.join("sce_sys/param.sfo")).map_err(|e| e.to_string()).and_then(|d| sfo(&d)).unwrap_or_default();
    let or_unknown = |key| Some(text(&info, key)).filter(|s| !s.is_empty()).unwrap_or_else(|| "?".into());
    let (title, version) = (or_unknown("TITLE_ID"), or_unknown("APP_VER"));
    let found = format!("Found {title} version {version} (sce_sys/param.sfo).");
    let kind = match image_sha256(game) {
        Ok(hash) if hash == SUPPORTED_IMAGE => match has_broken_files(game) {
            Ok(false) => return None,
            Ok(true) => "damaged_files",
            Err(_) => "unreadable",
        },
        Err(_) => "unreadable",
        Ok(hash) if hash == LANCE_60FPS_IMAGE => "patched_eboot",
        Ok(_) if !SUPPORTED_TITLES.contains(&title.as_str()) => "other_title",
        Ok(_) if version != SUPPORTED_VERSION => "missing_update",
        Ok(_) => "wrong_eboot",
    };
    Some(match kind {
        "missing_update" => format!("{found} bbport needs the 1.09 update merged into the game folder: copy everything from the dumped 1.09 update (e.g. CUSA03173-patch) into the game folder, replacing files (eboot.bin and sce_sys too)."),
        "wrong_eboot" => format!("{found} param.sfo is from 1.09 but eboot.bin is not: copy eboot.bin from the dumped 1.09 update into the game folder, replacing the old one."),
        "patched_eboot" => format!("{found} This eboot.bin has the 60 fps patch by Lance McDonald applied to it, which crashes the game when the gestures menu opens (bbport has its own 60 fps): copy eboot.bin from a clean dump of the 1.09 update into the game folder, replacing this one."),
        "other_title" => format!("{found} This is not a retail release of Bloodborne: bbport runs Bloodborne (any region, The Old Hunters editions too) with update 1.09."),
        "unreadable" => format!("{found} eboot.bin could not be read as a decrypted PS4 executable: dump the game and the 1.09 update again."),
        _ => format!("{found} The game files are damaged: the shaders in {CHECKED_FOLDER} do not unpack, and the game would hang while loading them. The extraction tool left parts of compressed sectors stale (LibOrbisPkg PkgTool on .NET 6 or newer, issue #81): extract the game and the 1.09 update again with a fixed tool. scripts/game_check.py GAME_DIR checks every file."),
    })
}

/// The launcher's answer for a game folder: {"problem": text or null, "game": edition or null}.
pub fn check(game: &Path) -> Value {
    let edition = describe(game);
    if !game.join("eboot.bin").is_file() {
        return json!({"problem": "No eboot.bin in this folder: choose the folder of your Bloodborne dump.", "game": edition});
    }
    json!({"problem": problem(game), "game": edition})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sfo_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
        let (mut keys, mut values, mut table) = (Vec::new(), Vec::new(), Vec::new());
        for (key, value) in entries {
            let (key_at, value_at) = (keys.len() as u16, values.len() as u32);
            keys.extend_from_slice(key.as_bytes());
            keys.push(0);
            let mut v = value.as_bytes().to_vec();
            v.push(0);
            table.extend_from_slice(&key_at.to_le_bytes());
            table.extend_from_slice(&0x204u16.to_le_bytes());
            table.extend_from_slice(&(v.len() as u32).to_le_bytes());
            table.extend_from_slice(&(v.len() as u32).to_le_bytes());
            table.extend_from_slice(&value_at.to_le_bytes());
            values.extend_from_slice(&v);
        }
        let keys_at = 20 + table.len() as u32;
        let values_at = keys_at + keys.len() as u32;
        let mut out = b"\0PSF".to_vec();
        out.extend_from_slice(&0x101u32.to_le_bytes());
        out.extend_from_slice(&keys_at.to_le_bytes());
        out.extend_from_slice(&values_at.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        out.extend(table);
        out.extend(keys);
        out.extend(values);
        out
    }

    #[test]
    fn edition_and_problems_of_synthetic_folders() {
        let dir = std::env::temp_dir().join(format!("bb-game-check-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join("sce_sys")).unwrap();
        let empty = check(&dir);
        assert!(empty["problem"].as_str().unwrap().starts_with("No eboot.bin"));
        fs::write(
            dir.join("sce_sys/param.sfo"),
            sfo_bytes(&[("TITLE_ID", "CUSA03173"), ("APP_VER", "01.09"), ("TITLE", "Bloodborne"), ("CONTENT_ID", "EP9000-CUSA03173_00-X")]),
        )
        .unwrap();
        fs::write(dir.join("eboot.bin"), b"not an executable").unwrap();
        let answer = check(&dir);
        assert_eq!(answer["game"]["title_id"], "CUSA03173");
        assert_eq!(answer["game"]["region"], "europe");
        assert_eq!(answer["game"]["old_hunters"], true);
        assert!(answer["problem"].as_str().unwrap().contains("could not be read as a decrypted PS4 executable"));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The player's real dump against scripts/game_check.py: BB_CHECK_GAME=<dump> cargo test
    /// real_dump -- --ignored --nocapture (read-only).
    #[test]
    #[ignore]
    fn real_dump() {
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        println!("{}", check(Path::new(&game)));
    }

    #[test]
    fn dcx_with_bad_deflate_is_broken() {
        let dir = std::env::temp_dir().join(format!("bb-dcx-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let mut dcx = vec![0u8; 0x4C];
        dcx[..4].copy_from_slice(b"DCX\0");
        dcx[0x18..0x1C].copy_from_slice(b"DCS\0");
        dcx[0x28..0x2C].copy_from_slice(b"DFLT");
        dcx[0x1C..0x20].copy_from_slice(&16u32.to_be_bytes());
        dcx[0x20..0x24].copy_from_slice(&4u32.to_be_bytes());
        dcx.extend_from_slice(b"junk");
        let path = dir.join("a.dcx");
        fs::write(&path, &dcx).unwrap();
        assert!(dcx_broken(&path).unwrap());
        fs::remove_dir_all(&dir).unwrap();
    }
}
