// SPDX-License-Identifier: GPL-2.0-or-later
//! Save sets are opaque directories. No character data is parsed or rewritten here.
//! Publish copies only after completion; reject links and special files throughout a save tree.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, String>;

#[derive(Serialize, Deserialize)]
struct Metadata {
    name: String,
    created: u64,
}
#[derive(Serialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub modified: Option<u64>,
    pub backups: Vec<Backup>,
    pub trophies_eligible: bool,
    pub trophy_unlocks: Vec<crate::trophies::Unlock>,
    pub trophy_error: Option<String>,
}
#[derive(Serialize)]
pub struct Backup {
    pub id: String,
    pub created: u64,
    pub bytes: u64,
    /// Whether the save set in it was edited (its .bb-edited marker).
    pub edited: bool,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn validate(id: &str) -> Result<()> {
    if id == "original"
        || (id.starts_with("p-") && id.len() > 2 && id[2..].bytes().all(|c| c.is_ascii_hexdigit()))
    {
        Ok(())
    } else {
        Err("Invalid save profile.".into())
    }
}
fn plain_directory(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(fail)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Save folders must be real directories, not links.".into());
    }
    Ok(())
}
fn check_ancestors(path: &Path) -> Result<()> {
    // Check every existing ancestor before creating anything inside it.
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => plain_directory(ancestor)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(fail(e)),
        }
    }
    Ok(())
}
pub(crate) fn ensure_directory(path: &Path) -> Result<()> {
    check_ancestors(path)?;
    fs::create_dir_all(path).map_err(fail)
}
fn copy_tree(source: &Path, dest: &Path) -> Result<()> {
    check_ancestors(source)?;
    plain_directory(source)?;
    fs::create_dir(dest).map_err(fail)?;
    for entry in fs::read_dir(source).map_err(fail)? {
        let entry = entry.map_err(fail)?;
        let kind = entry.file_type().map_err(fail)?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &dest.join(entry.file_name()))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), dest.join(entry.file_name())).map_err(fail)?;
        } else {
            return Err("Save sets cannot contain links or special files.".into());
        }
    }
    Ok(())
}
fn stats(path: &Path) -> Result<(u64, Option<u64>)> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((0, None)),
        Err(e) => return Err(fail(e)),
        Ok(_) => plain_directory(path)?,
    }
    let mut bytes = 0;
    let mut modified = None;
    for entry in fs::read_dir(path).map_err(fail)? {
        let entry = entry.map_err(fail)?;
        let meta = fs::symlink_metadata(entry.path()).map_err(fail)?;
        let (size, time) = if meta.is_dir() {
            stats(&entry.path())?
        } else if meta.is_file() {
            (
                meta.len(),
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|t| t.as_secs()),
            )
        } else {
            return Err("Save sets cannot contain links or special files.".into());
        };
        bytes += size;
        modified = modified.max(time);
    }
    Ok((bytes, modified))
}
fn allocate(root: &Path, prefix: &str) -> Result<(String, PathBuf)> {
    ensure_directory(root)?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(fail)?
        .as_nanos();
    for index in 0..100 {
        let id = format!("{prefix}{:x}", seed + index);
        let path = root.join(format!(".{id}.pending"));
        if root.join(&id).exists() {
            continue;
        }
        match fs::create_dir(&path) {
            Ok(()) => return Ok((id, path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail(e)),
        }
    }
    Err("Could not allocate a save folder.".into())
}
fn metadata(path: &Path) -> Result<Metadata> {
    let file = path.join("profile.json");
    let meta = fs::symlink_metadata(&file).map_err(fail)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Invalid save profile metadata.".into());
    }
    serde_json::from_slice(&fs::read(file).map_err(fail)?).map_err(fail)
}
fn publish(root: &Path, name: &str, source: Option<&Path>, prefix: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Use a profile name between 1 and 80 characters.".into());
    }
    let (id, stage) = allocate(root, prefix)?;
    let result = (|| {
        let target = if prefix == "p-" {
            fs::create_dir(stage.join("user")).map_err(fail)?;
            stage.join("user/savedata")
        } else {
            stage.join("savedata")
        };
        if let Some(source) = source {
            match fs::symlink_metadata(source) {
                Ok(_) => copy_tree(source, &target)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&target).map_err(fail)?;
                }
                Err(e) => return Err(fail(e)),
            }
        } else {
            fs::create_dir(&target).map_err(fail)?;
        }
        fs::write(
            stage.join("profile.json"),
            serde_json::to_vec(&Metadata {
                name: name.into(),
                created: now(),
            })
            .map_err(fail)?,
        )
        .map_err(fail)?;
        fs::rename(&stage, root.join(&id)).map_err(fail)?;
        Ok(id)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

fn check_restore(user: &Path) -> Result<()> {
    if !user.try_exists().map_err(fail)? {
        return Ok(());
    }
    plain_directory(user)?;
    for entry in fs::read_dir(user).map_err(fail)? {
        let entry = entry.map_err(fail)?;
        if (entry.file_name().to_string_lossy().starts_with(".restore-")
            || entry.file_name().to_string_lossy().starts_with(".edit-"))
            && entry.path().join("previous").try_exists().map_err(fail)?
        {
            return Err(format!("An unfinished save change needs recovery before this save can be used. Previous files: {}", entry.path().join("previous").display()));
        }
    }
    Ok(())
}

pub fn user_dir(data: &Path, original: &Path, id: &str) -> Result<PathBuf> {
    validate(id)?;
    if id == "original" {
        check_ancestors(original)?;
        check_restore(original)?;
        return Ok(original.into());
    }
    let root = data.join("save-profiles");
    check_ancestors(&root)?;
    plain_directory(&root)?;
    let profile = root.join(id);
    plain_directory(&profile)?;
    metadata(&profile)?;
    let user = profile.join("user");
    plain_directory(&user)?;
    check_restore(&user)?;
    Ok(user)
}
pub fn list(data: &Path, original: &Path) -> Result<Vec<Profile>> {
    list_excluding(data, original, &[])
}
pub fn list_excluding(data: &Path, original: &Path, excluded: &[String]) -> Result<Vec<Profile>> {
    let mut ids = vec![("original".to_string(), String::new())];
    let root = data.join("save-profiles");
    if root.try_exists().map_err(fail)? {
        plain_directory(&root)?;
        for entry in fs::read_dir(&root).map_err(fail)? {
            let entry = entry.map_err(fail)?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') || excluded.contains(&id) {
                continue;
            }
            validate(&id)?;
            plain_directory(&entry.path())?;
            ids.push((id, metadata(&entry.path())?.name));
        }
    }
    ids[1..].sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    ids.into_iter()
        .filter(|(id, _)| !excluded.contains(id))
        .map(|(id, name)| {
            let path = user_dir(data, original, &id)?;
            let (bytes, modified) = stats(&path.join("savedata"))?;
            let backup_root = data.join("save-backups").join(&id);
            let mut backups = Vec::new();
            if backup_root.try_exists().map_err(fail)? {
                plain_directory(&backup_root)?;
                for entry in fs::read_dir(&backup_root).map_err(fail)? {
                    let entry = entry.map_err(fail)?;
                    let bid = entry.file_name().to_string_lossy().into_owned();
                    if bid.starts_with('.') {
                        continue;
                    }
                    backup_id(&bid)?;
                    plain_directory(&entry.path())?;
                    backups.push(Backup {
                        id: bid,
                        created: metadata(&entry.path())?.created,
                        bytes: stats(&entry.path().join("savedata"))?.0,
                        edited: fs::symlink_metadata(entry.path().join("savedata/.bb-edited")).is_ok_and(|meta| meta.is_file()),
                    });
                }
            }
            backups.sort_by(|a, b| b.id.cmp(&a.id));
            Ok(Profile {
                trophies_eligible: crate::trophies::eligible(&path),
                trophy_unlocks: Vec::new(),
                trophy_error: None,
                id,
                name,
                path,
                bytes,
                modified,
                backups,
            })
        })
        .collect()
}
// Only profile-owned files are removed; the original user root also holds shared GPU caches.
pub fn delete_profile_files(data: &Path, original: &Path, id: &str) -> Result<()> {
    validate(id)?;
    let saves = if id == "original" {
        original.join("savedata")
    } else {
        data.join("save-profiles").join(id)
    };
    if id == "original" {
        let events=original.join("trophies-v1.log");
        check_ancestors(original)?;
        match fs::symlink_metadata(&events) {
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => return Err("Invalid trophy event file.".into()),
            Ok(_) => fs::remove_file(events).map_err(fail)?,
            Err(e) if e.kind()==std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(fail(e)),
        }
    }
    for folder in [saves, data.join("save-backups").join(id)] {
        check_ancestors(&folder)?;
        match fs::symlink_metadata(&folder) {
            Ok(_) => {
                plain_directory(&folder)?;
                fs::remove_dir_all(&folder).map_err(fail)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(fail(e)),
        }
    }
    Ok(())
}
pub fn create(data: &Path, original: &Path, name: &str, source: Option<&str>) -> Result<String> {
    let source = source.map(|id| user_dir(data, original, id)).transpose()?;
    let root = data.join("save-profiles");
    // Publish profiles with the same user directory layout as the original runtime path.
    publish(
        &root,
        name,
        source.as_ref().map(|p| p.join("savedata")).as_deref(),
        "p-",
    )
}
pub fn backup(data: &Path, original: &Path, id: &str) -> Result<String> {
    let user = user_dir(data, original, id)?;
    publish(
        &data.join("save-backups").join(id),
        "Backup",
        Some(&user.join("savedata")),
        "b-",
    )
}
fn backup_id(id: &str) -> Result<()> {
    if id.starts_with("b-") && id.len() > 2 && id[2..].bytes().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("Invalid backup.".into())
    }
}
pub fn restore(data: &Path, original: &Path, id: &str, bid: &str) -> Result<()> {
    backup_id(bid)?;
    let user = user_dir(data, original, id)?;
    let source = data.join("save-backups").join(id).join(bid);
    check_ancestors(&source)?;
    metadata(&source)?;
    plain_directory(&source)?;
    ensure_directory(&user)?;
    // Keep a recovery copy before replacing any live save files.
    backup(data, original, id)?;
    let (_, stage) = allocate(&user, "restore-")?;
    let live = user.join("savedata");
    let result = (|| {
        copy_tree(&source.join("savedata"), &stage.join("savedata"))?;
        let exists = live.try_exists().map_err(fail)?;
        if exists {
            plain_directory(&live)?;
            fs::rename(&live, stage.join("previous")).map_err(fail)?;
        }
        if let Err(e) = fs::rename(stage.join("savedata"), &live) {
            if exists {
                fs::rename(stage.join("previous"), &live).map_err(|rollback| format!("Restore failed: {e}; rollback failed: {rollback}. Previous files retained at {}", stage.display()))?;
            }
            return Err(fail(e));
        }
        Ok(())
    })();
    // If rollback failed, never delete the retained previous files.
    if !stage.join("previous").exists() || result.is_ok() {
        fs::remove_dir_all(&stage).map_err(|e| {
            format!(
                "Could not remove restore staging folder {}: {e}",
                stage.display()
            )
        })?;
    }
    result
}

/// Edit a staged copy of the whole set, then publish it with the trophy marker.
/// The pre-edit backup remains separate from the newly edited lineage.
pub fn edit(data: &Path, original: &Path, id: &str, change: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let user = user_dir(data, original, id)?;
    let live = user.join("savedata");
    plain_directory(&live)?;
    let (_, stage) = allocate(&user, "edit-")?;
    let result = (|| {
        copy_tree(&live, &stage.join("savedata"))?;
        change(&stage.join("savedata"))?;
        let marker = stage.join("savedata/.bb-edited");
        match fs::symlink_metadata(&marker) {
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => return Err("Invalid edited-save marker.".into()),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::write(&marker, b"edited by bloodborne-mac\n").map_err(fail)?,
            Err(e) => return Err(fail(e)),
        }
        backup(data, original, id)?;
        fs::rename(&live, stage.join("previous")).map_err(fail)?;
        if let Err(e) = fs::rename(stage.join("savedata"), &live) {
            fs::rename(stage.join("previous"), &live).map_err(|rollback| format!("Edit failed: {e}; rollback failed: {rollback}. Previous files retained at {}", stage.display()))?;
            return Err(fail(e));
        }
        Ok(())
    })();
    if !stage.join("previous").exists() || result.is_ok() {
        fs::remove_dir_all(&stage).map_err(|e| format!("Could not remove edit staging folder {}: {e}", stage.display()))?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let (_, root) = allocate(
                &fs::canonicalize(std::env::temp_dir()).unwrap(),
                "bb-save-test-",
            )
            .unwrap();
            fs::create_dir(root.join("original")).unwrap();
            Self(root)
        }
        fn original(&self) -> PathBuf {
            self.0.join("original")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn profile_backup_restore_preserves_complete_sets() {
        let f = Fixture::new();
        let original = f.original();
        fs::create_dir_all(original.join("savedata/1/TITLE/save.sce_sys")).unwrap();
        fs::create_dir_all(original.join("savedata/1/TITLE.memory")).unwrap();
        fs::write(
            original.join("savedata/1/TITLE/save.sce_sys/meta"),
            b"synthetic metadata",
        )
        .unwrap();
        fs::write(
            original.join("savedata/1/TITLE.memory/memory"),
            b"synthetic memory",
        )
        .unwrap();
        let id = create(&f.0, &original, "Copy", Some("original")).unwrap();
        let user = user_dir(&f.0, &original, &id).unwrap();
        let bid = backup(&f.0, &original, &id).unwrap();
        fs::write(user.join("savedata/extra"), b"changed").unwrap();
        restore(&f.0, &original, &id, &bid).unwrap();
        assert!(!user.join("savedata/extra").exists());
        assert_eq!(
            fs::read(user.join("savedata/1/TITLE.memory/memory")).unwrap(),
            b"synthetic memory"
        );
        assert_eq!(
            list(&f.0, &original)
                .unwrap()
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .backups
                .len(),
            2
        );
        assert!(!original.join("savedata/extra").exists());
        let fresh = create(&f.0, &original, "Fresh", None).unwrap();
        assert_eq!(
            list(&f.0, &original)
                .unwrap()
                .iter()
                .find(|p| p.id == fresh)
                .unwrap()
                .bytes,
            0
        );
        assert!(user_dir(&f.0, &original, "../escape").is_err());
        assert!(restore(&f.0, &original, &id, "../escape").is_err());
    }
    #[test]
    fn interrupted_restore_blocks_launch_and_preserves_previous_files() {
        let f = Fixture::new();
        let original = f.original();
        fs::create_dir_all(original.join(".restore-a.pending/previous")).unwrap();
        fs::write(
            original.join(".restore-a.pending/previous/data"),
            b"synthetic previous save",
        )
        .unwrap();
        assert!(user_dir(&f.0, &original, "original")
            .unwrap_err()
            .contains("unfinished save change"));
        assert_eq!(
            fs::read(original.join(".restore-a.pending/previous/data")).unwrap(),
            b"synthetic previous save"
        );
    }
    #[test]
    fn interrupted_edit_blocks_launch_and_preserves_previous_files() {
        let f = Fixture::new();
        let original = f.original();
        fs::create_dir_all(original.join(".edit-a.pending/previous")).unwrap();
        fs::write(original.join(".edit-a.pending/previous/data"), b"synthetic previous save").unwrap();
        assert!(user_dir(&f.0, &original, "original").unwrap_err().contains("unfinished save change"));
        assert_eq!(fs::read(original.join(".edit-a.pending/previous/data")).unwrap(), b"synthetic previous save");
    }
    #[cfg(unix)]
    #[test]
    fn corrupt_backup_does_not_replace_live_save() {
        let f = Fixture::new();
        let original = f.original();
        fs::create_dir(original.join("savedata")).unwrap();
        fs::write(original.join("savedata/data"), b"synthetic current save").unwrap();
        let bid = backup(&f.0, &original, "original").unwrap();
        let savedata =
            f.0.join("save-backups/original")
                .join(&bid)
                .join("savedata");
        std::os::unix::fs::symlink(&f.0, savedata.join("bad-link")).unwrap();
        assert!(restore(&f.0, &original, "original", &bid).is_err());
        assert_eq!(
            fs::read(original.join("savedata/data")).unwrap(),
            b"synthetic current save"
        );
        assert!(user_dir(&f.0, &original, "original").is_ok());
        assert!(!fs::read_dir(&original).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".restore-")));
    }
    #[cfg(unix)]
    #[test]
    fn reject_links_without_publishing_or_replacing() {
        let f = Fixture::new();
        let original = f.original();
        fs::create_dir(original.join("savedata")).unwrap();
        std::os::unix::fs::symlink(&f.0, original.join("savedata/link")).unwrap();
        assert!(create(&f.0, &original, "Bad", Some("original")).is_err());
        assert!(backup(&f.0, &original, "original").is_err());
        assert!(fs::read_dir(f.0.join("save-profiles"))
            .unwrap()
            .next()
            .is_none());
        assert!(original.join("savedata/link").symlink_metadata().is_ok());
    }
}
