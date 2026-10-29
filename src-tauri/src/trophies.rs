// SPDX-License-Identifier: GPL-2.0-or-later
//! Profile-owned local unlock events. Save contents and trophy catalogs are never parsed here.
use crate::{database as db, saves};
use serde::Serialize;
use sqlx::{Connection, Row, SqliteConnection};
use std::{fs, io::Read, path::Path};

#[derive(Serialize)]
pub struct Unlock {
    pub title_id: String,
    pub trophy_id: i32,
    pub translations: std::collections::BTreeMap<String, crate::trophy_catalog::Text>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub grade: crate::trophy_catalog::Grade,
    pub group: Option<String>,
    pub unlocked_at: i64, // Unix seconds, matching the runtime event protocol.
}
pub fn eligible(user: &Path) -> bool {
    let save = user.join("savedata");
    match fs::symlink_metadata(&save) {
        Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => return false,
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return false,
        _ => {}
    }
    matches!(fs::symlink_metadata(save.join(".bb-edited")), Err(e) if e.kind()==std::io::ErrorKind::NotFound)
}
struct Event {
    title_id: String,
    trophy_id: i32,
    unlocked_at: i64,
}
fn events(user: &Path) -> db::Result<Vec<Event>> {
    let path = user.join("trophies-v1.log");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(db::err(e)),
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err("Invalid trophy event file.".into())
        }
        _ => {}
    }
    // A concurrent append can leave an incomplete final record; retry it on the next refresh.
    // Bound both allocation and read size; never reset or truncate a user's event file.
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(db::err)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(db::err)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Trophy event file exceeds 32 MiB.".into());
    }
    let end = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    let text = std::str::from_utf8(&bytes[..end]).map_err(db::err)?;
    let mut result = std::collections::BTreeMap::<(String, i32), i64>::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 4 || fields[0] != "v1" {
            continue;
        }
        let (Ok(timestamp), Ok(id)) = (fields[1].parse::<i64>(), fields[3].parse::<i32>()) else {
            continue;
        };
        let title = fields[2];
        if !(0..128).contains(&id)
            || timestamp <= 0
            || timestamp > db::now() + 300
            || title.is_empty()
            || title.len() > 16
            || !title
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            continue;
        }
        result
            .entry((title.into(), id))
            .and_modify(|at| *at = (*at).min(timestamp))
            .or_insert(timestamp);
    }
    Ok(result
        .into_iter()
        .map(|((title_id, trophy_id), unlocked_at)| Event {
            title_id,
            trophy_id,
            unlocked_at,
        })
        .collect())
}
pub async fn refresh(
    connection: &mut SqliteConnection,
    profile: &mut saves::Profile,
) -> db::Result<()> {
    profile.trophies_eligible = eligible(&profile.path);
    // The runtime also gates generation. Never import from an edited save's current session.
    if profile.trophies_eligible {
        match events(&profile.path) {
            Ok(events) => {
                let mut tx = connection
                    .begin_with("BEGIN IMMEDIATE")
                    .await
                    .map_err(db::err)?;
                for event in events {
                    sqlx::query("INSERT INTO trophy_unlocks (profile_id,title_id,trophy_id,unlocked_at) VALUES (?,?,?,?) ON CONFLICT(profile_id,title_id,trophy_id) DO UPDATE SET unlocked_at=MIN(unlocked_at,excluded.unlocked_at)")
                        .bind(&profile.id).bind(event.title_id).bind(event.trophy_id).bind(event.unlocked_at)
                        .execute(&mut *tx).await.map_err(db::err)?;
                }
                tx.commit().await.map_err(db::err)?;
            }
            Err(error) => profile.trophy_error = Some(error), // History and other profiles remain usable.
        }
    }
    let rows=sqlx::query("SELECT title_id,trophy_id,unlocked_at FROM trophy_unlocks WHERE profile_id=? ORDER BY unlocked_at DESC,title_id,trophy_id")
        .bind(&profile.id).fetch_all(connection).await.map_err(db::err)?;
    profile.trophy_unlocks = rows
        .into_iter()
        .map(|row| {
            let title: String = row.try_get("title_id").map_err(db::err)?;
            let id: i32 = row.try_get("trophy_id").map_err(db::err)?;
            let trophy = u8::try_from(id)
                .ok()
                .and_then(|id| crate::trophy_catalog::find(&title, id));
            Ok(Unlock {
                translations: trophy.map(|t| t.translations.clone()).unwrap_or_default(),
                name: trophy.map(|t| t.text("en").name.clone()),
                description: trophy.and_then(|t| t.text("en").description.clone()),
                grade: trophy.map_or(crate::trophy_catalog::Grade::Unknown, |t| t.grade),
                group: trophy.map(|t| t.group.clone()),
                title_id: title,
                trophy_id: id,
                unlocked_at: row.try_get("unlocked_at").map_err(db::err)?,
            })
        })
        .collect::<db::Result<_>>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles;
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = fs::canonicalize(std::env::temp_dir())
                .unwrap()
                .join(format!("bb-trophies-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("user")).unwrap();
            profiles::initialize(&root, &root.join("user"), "original", "First").unwrap();
            Self(root)
        }
        fn user(&self) -> std::path::PathBuf {
            self.0.join("user")
        }
        fn get(&self) -> saves::Profile {
            profiles::selected(&self.0).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Windows keeps the database's files locked while a pooled connection lingers.
            let result = fs::remove_dir_all(&self.0);
            if !cfg!(windows) {
                result.unwrap();
            }
        }
    }
    #[test]
    fn import_is_idempotent_earliest_wins_and_incomplete_records_retry() {
        let f = Fixture::new();
        fs::write(f.user().join("trophies-v1.log"),"v1 123 SYNTH0001 7\nv1 125 SYNTH0001 7\nv1 121 SYNTH0001 7\ninvalid\nv1 123 SYNTH0001 128\nv1 123 SYNTH0001 -1\nv1 0 SYNTH0001 3\nv1 999999999999 SYNTH0001 3\nv1 123 SYNTH0001 8").unwrap();
        for _ in 0..2 {
            let p = f.get();
            assert_eq!(p.trophy_unlocks.len(), 1);
            assert_eq!(p.trophy_unlocks[0].unlocked_at, 121);
        }
        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(f.user().join("trophies-v1.log"))
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        assert_eq!(f.get().trophy_unlocks.len(), 2);
    }
    #[test]
    fn edited_backups_block_import_restoring_clean_save_preserves_history() {
        let f = Fixture::new();
        fs::create_dir(f.user().join("savedata")).unwrap();
        fs::write(f.user().join("savedata/data"), b"synthetic").unwrap();
        fs::write(f.user().join("trophies-v1.log"), "v1 123 SYNTH0001 1\n").unwrap();
        assert_eq!(f.get().trophy_unlocks.len(), 1);
        let clean = saves::backup(&f.0, &f.user(), "original").unwrap();
        fs::write(f.user().join("savedata/.bb-edited"), b"edited-v1\n").unwrap();
        let edited = saves::backup(&f.0, &f.user(), "original").unwrap();
        let p = f.get();
        assert!(!p.trophies_eligible);
        assert_eq!(p.trophy_unlocks.len(), 1);
        saves::restore(&f.0, &f.user(), "original", &clean).unwrap();
        assert!(f.get().trophies_eligible);
        saves::restore(&f.0, &f.user(), "original", &edited).unwrap();
        assert!(!f.get().trophies_eligible);
        assert_eq!(f.get().trophy_unlocks.len(), 1);
    }
    #[test]
    fn profiles_are_isolated_and_deleted_events_do_not_return() {
        let f = Fixture::new();
        fs::write(f.user().join("trophies-v1.log"), "v1 123 SYNTH0001 1\n").unwrap();
        assert_eq!(f.get().trophy_unlocks.len(), 1);
        profiles::change(&f.0, "create", "", Some("Second"), None).unwrap();
        let id = profiles::list(&f.0, &f.user())
            .unwrap()
            .profiles
            .into_iter()
            .find(|p| p.name == "Second")
            .unwrap()
            .id;
        profiles::change(&f.0, "select", &id, None, None).unwrap();
        assert!(f.get().trophy_unlocks.is_empty());
        profiles::change(&f.0, "delete", "original", None, None).unwrap();
        assert!(!f.user().join("trophies-v1.log").exists());
        assert!(f.get().trophy_unlocks.is_empty());
    }
    #[test]
    fn unreadable_events_do_not_reset_history_or_block_profiles() {
        let f = Fixture::new();
        let log = f.user().join("trophies-v1.log");
        fs::write(&log, "v1 123 SYNTH0001 1\n").unwrap();
        assert_eq!(f.get().trophy_unlocks.len(), 1);
        fs::remove_file(&log).unwrap();
        fs::create_dir(&log).unwrap();
        let p = f.get();
        assert!(p.trophy_error.is_some());
        assert_eq!(p.trophy_unlocks.len(), 1);
    }
}
