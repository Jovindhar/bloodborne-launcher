// SPDX-License-Identifier: GPL-2.0-or-later
//! Drizzle-generated schema, native transactions. Blocking entry points are called only on
//! Tauri blocking workers, setup, or synchronous tests, never on async executor workers.
use crate::saves;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, Row, SqliteConnection};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub type Result<T> = std::result::Result<T, String>;
pub fn path(data: &Path) -> PathBuf {
    data.join("launcher.db")
}
/// Blocking wait on one query: call it from a blocking thread only (a command's `blocking(...)`),
/// never on the async runtime itself, where tokio panics and the invoke never answers.
pub fn block<T>(future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    tauri::async_runtime::block_on(future)
}
pub async fn connect(data: &Path) -> Result<SqliteConnection> {
    saves::ensure_directory(data)?;
    let file = path(data);
    if file
        .symlink_metadata()
        .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
    {
        return Err("Invalid launcher database file.".into());
    }
    let options = SqliteConnectOptions::new()
        .filename(file)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(10));
    let mut connection = open(&options).await?;
    migrate(&mut connection).await?;
    Ok(connection)
}
async fn open(options: &SqliteConnectOptions) -> Result<SqliteConnection> {
    // SQLx documents that switching journal mode needs an exclusive lock which
    // sqlite3_busy_timeout cannot wait for. Retry this initialization step only;
    // migrations keep their own atomic transactions and normal busy timeout.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut delay = Duration::from_millis(10);
    loop {
        match SqliteConnection::connect_with(options).await {
            Ok(connection) => return Ok(connection),
            Err(error) if initialization_locked(&error) && Instant::now() < deadline => {
                tokio::time::sleep(delay.min(deadline.saturating_duration_since(Instant::now())))
                    .await;
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            Err(error) => return Err(err(error)),
        }
    }
}
fn initialization_locked(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| matches!(code & 0xff, 5 | 6)) // SQLITE_BUSY / SQLITE_LOCKED, including extended codes.
}
pub fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0000_launcher_storage",
        include_str!("../../src/db/migrations/0000_launcher_storage.sql"),
    ),
    (
        "0001_profile_deletion",
        include_str!("../../src/db/migrations/0001_profile_deletion.sql"),
    ),
    (
        "0002_settings",
        include_str!("../../src/db/migrations/0002_settings.sql"),
    ),
];
// This first migration briefly shipped under the original Drizzle-generated name.
// Both identities describe exactly the same SQL; never rerun its DDL to repair history.
const INITIAL_ALIAS: &str = "0000_giant_rage";

async fn schema(connection: &mut SqliteConnection) -> Result<Vec<(String, String, String)>> {
    sqlx::query_as("SELECT type, name, sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' AND name != '__launcher_migrations' AND sql IS NOT NULL ORDER BY type, name")
        .fetch_all(connection).await.map_err(err)
}
async fn execute_sql(connection: &mut SqliteConnection, sql: &str) -> Result<()> {
    for statement in sql
        .split("--> statement-breakpoint")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(statement)
            .execute(&mut *connection)
            .await
            .map_err(err)?;
    }
    Ok(())
}
async fn migrate(connection: &mut SqliteConnection) -> Result<()> {
    migrate_with(connection, MIGRATIONS).await
}
async fn migrate_with(
    connection: &mut SqliteConnection,
    migrations: &[(&str, &str)],
) -> Result<()> {
    // Validate and repair history atomically before attempting any schema change.
    // A newer launcher's history must never be interpreted as an empty database.
    let mut tx = connection
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(err)?;
    sqlx::query("CREATE TABLE IF NOT EXISTS __launcher_migrations (tag TEXT PRIMARY KEY, applied_at INTEGER NOT NULL, migration_sql TEXT)")
        .execute(&mut *tx).await.map_err(err)?;
    let columns = sqlx::query("PRAGMA table_info(__launcher_migrations)")
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
    if !columns
        .iter()
        .any(|r| r.get::<String, _>("name") == "migration_sql")
    {
        sqlx::query("ALTER TABLE __launcher_migrations ADD COLUMN migration_sql TEXT")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
    }
    let history: Vec<(String, i64, Option<String>)> =
        sqlx::query_as("SELECT tag, applied_at, migration_sql FROM __launcher_migrations")
            .fetch_all(&mut *tx)
            .await
            .map_err(err)?;
    let mut applied = vec![false; migrations.len()];
    for (tag, _, recorded_sql) in &history {
        let canonical = if tag == INITIAL_ALIAS {
            migrations[0].0
        } else {
            tag
        };
        let Some(index) = migrations.iter().position(|(known, _)| *known == canonical) else {
            return Err(format!("This database belongs to a newer or unsupported launcher ({tag}). Install the latest launcher. Your data has not been reset."));
        };
        if let Some(recorded) = recorded_sql {
            if recorded != migrations[index].1 {
                return Err(format!("Migration {tag} changed after it was applied. Reinstall the launcher or report this error. Your data has not been reset."));
            }
        }
        applied[index] = true;
    }
    let prefix = applied.iter().take_while(|v| **v).count();
    if applied[prefix..].iter().any(|v| *v) {
        return Err("Launcher migration history is incomplete. Your data has not been reset. Please report this error.".into());
    }
    let actual = schema(&mut tx).await?;
    let mut reference = SqliteConnection::connect("sqlite::memory:")
        .await
        .map_err(err)?;
    for (_, sql) in &migrations[..prefix] {
        execute_sql(&mut reference, sql).await?;
    }
    let expected = schema(&mut reference).await?;
    let adopt_initial = prefix == 0 && !actual.is_empty();
    if adopt_initial {
        execute_sql(&mut reference, migrations[0].1).await?;
        if actual != schema(&mut reference).await? {
            return Err("Launcher database schema does not match a supported upgrade. Your data has not been reset. Please report this error.".into());
        }
        sqlx::query(
            "INSERT INTO __launcher_migrations (tag, applied_at, migration_sql) VALUES (?, ?, ?)",
        )
        .bind(migrations[0].0)
        .bind(now())
        .bind(migrations[0].1)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
    } else if actual != expected {
        return Err("Launcher database schema does not match its migration history. Your data has not been reset. Please report this error.".into());
    }
    for (index, (tag, sql)) in migrations.iter().enumerate().take(prefix) {
        let applied_at = history
            .iter()
            .filter(|(old, _, _)| old == tag || (index == 0 && old == INITIAL_ALIAS))
            .map(|(_, time, _)| *time)
            .min()
            .unwrap();
        let alias_present = index == 0 && history.iter().any(|(old, _, _)| old == INITIAL_ALIAS);
        let already_recorded = history.iter().any(|(old, time, recorded)| {
            old == tag && *time == applied_at && recorded.as_deref() == Some(*sql)
        });
        if already_recorded && !alias_present {
            continue;
        }
        if index == 0 {
            sqlx::query("DELETE FROM __launcher_migrations WHERE tag = ?")
                .bind(INITIAL_ALIAS)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
        }
        sqlx::query("INSERT INTO __launcher_migrations (tag, applied_at, migration_sql) VALUES (?, ?, ?) ON CONFLICT(tag) DO UPDATE SET applied_at=excluded.applied_at, migration_sql=excluded.migration_sql")
            .bind(tag).bind(applied_at).bind(sql).execute(&mut *tx).await.map_err(err)?;
    }
    let invalid = sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
    if invalid.is_some() {
        return Err("Launcher database has inconsistent ownership records. Your data has not been reset. Please report this error.".into());
    }
    tx.commit().await.map_err(err)?;
    // Existing data gets a consistent SQLite snapshot before any pending schema DDL.
    // VACUUM INTO includes committed WAL data; copying launcher.db alone would not.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM __launcher_migrations")
        .fetch_one(&mut *connection)
        .await
        .map_err(err)?;
    if count > 0 && (count as usize) < migrations.len() {
        snapshot(connection).await?;
    }
    apply_migrations(connection, migrations).await
}
async fn snapshot(connection: &mut SqliteConnection) -> Result<PathBuf> {
    let databases = sqlx::query("PRAGMA database_list")
        .fetch_all(&mut *connection)
        .await
        .map_err(err)?;
    let file: String = databases
        .iter()
        .find(|r| r.get::<String, _>("name") == "main")
        .ok_or("Missing launcher database path")?
        .get("file");
    let directory = Path::new(&file)
        .parent()
        .ok_or("Missing launcher database directory")?
        .join("database-backups");
    saves::ensure_directory(&directory)?;
    let backup = directory.join(format!("launcher-{}.db", operation_id()));
    sqlx::query("VACUUM INTO ?").bind(backup.to_string_lossy().as_ref()).execute(connection).await.map_err(|e| format!("Cannot back up launcher database before upgrading: {e}. Free disk space or check folder permissions, then retry."))?;
    Ok(backup)
}
async fn apply_migrations(
    connection: &mut SqliteConnection,
    migrations: &[(&str, &str)],
) -> Result<()> {
    // BEGIN IMMEDIATE serializes first-launch migration by multiple launcher instances.
    for (tag, sql) in migrations {
        let mut tx = connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(err)?;
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM __launcher_migrations WHERE tag = ?")
                .bind(tag)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        if exists == 0 {
            execute_sql(&mut tx, sql).await?;
            sqlx::query("INSERT INTO __launcher_migrations (tag, applied_at, migration_sql) VALUES (?, ?, ?)")
                .bind(tag).bind(now()).bind(sql).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
    }
    Ok(())
}
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs() as i64)
        .unwrap_or_default()
}
pub fn operation_id() -> String {
    format!("op-{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Removes a test's folder. On Windows the database's files stay locked while a pooled
    /// connection lingers, so the folder is left to the system's temporary-file cleanup there.
    fn remove_test_dir(root: impl AsRef<Path>) {
        let result = std::fs::remove_dir_all(root);
        if !cfg!(windows) {
            result.unwrap();
        }
    }
    fn fixture() -> PathBuf {
        std::fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(operation_id())
    }
    async fn initial_database(root: &Path) -> Result<SqliteConnection> {
        saves::ensure_directory(root)?;
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(path(root))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .map_err(err)?;
        execute_sql(&mut db, MIGRATIONS[0].1).await?;
        Ok(db)
    }
    async fn seed(db: &mut SqliteConnection) -> Result<()> {
        execute_sql(db, "INSERT INTO profiles (id,name,name_key,created_at) VALUES ('owner','Hunter','hunter',123); --> statement-breakpoint INSERT INTO save_sets VALUES ('set','owner','/synthetic'); --> statement-breakpoint INSERT INTO backups VALUES ('backup','set','/synthetic-backup',124,456); --> statement-breakpoint INSERT INTO trophy_unlocks VALUES ('owner','synthetic-title',1,125)").await
    }
    #[test]
    fn renamed_migration_preserves_records_and_original_timestamp() {
        let root = fixture();
        block(async {
            let mut db = initial_database(&root).await?;
            seed(&mut db).await?;
            execute_sql(&mut db, "CREATE TABLE __launcher_migrations (tag TEXT PRIMARY KEY, applied_at INTEGER NOT NULL); --> statement-breakpoint INSERT INTO __launcher_migrations VALUES ('0000_giant_rage',123)").await?;
            drop(db);
            for _ in 0..2 {
                let mut db = connect(&root).await?;
                let history: Vec<(String, i64, String)> = sqlx::query_as("SELECT tag, applied_at, migration_sql FROM __launcher_migrations").fetch_all(&mut db).await.map_err(err)?;
                assert_eq!(history.len(), MIGRATIONS.len());
                assert!(history.contains(&(MIGRATIONS[0].0.into(),123,MIGRATIONS[0].1.into())));
                for table in ["profiles", "save_sets", "backups", "trophy_unlocks"] {
                    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}")).fetch_one(&mut db).await.map_err(err)?;
                    assert_eq!(count,1);
                }
            }
            Ok(())
        }).unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn exact_untracked_schema_is_adopted_but_partial_schema_is_rejected() {
        let root = fixture();
        block(async {
            let mut db = initial_database(&root).await?;
            seed(&mut db).await?;
            // Check adoption of the exact initial schema without applying later migrations yet.
            migrate_with(&mut db, &MIGRATIONS[..1]).await?;
            sqlx::query("DROP TABLE __launcher_migrations")
                .execute(&mut db)
                .await
                .map_err(err)?;
            sqlx::query("DROP INDEX backups_path_unique")
                .execute(&mut db)
                .await
                .map_err(err)?;
            assert!(migrate(&mut db)
                .await
                .unwrap_err()
                .contains("does not match"));
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backups")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            // Even history-table creation was rolled back on validation failure.
            let history: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='__launcher_migrations'",
            )
            .fetch_one(&mut db)
            .await
            .map_err(err)?;
            assert_eq!(history, 0);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn newer_history_and_changed_migration_are_rejected_without_reset() {
        let root = fixture();
        block(async {
            let mut db = connect(&root).await?;
            seed(&mut db).await?;
            sqlx::query(
                "INSERT INTO __launcher_migrations VALUES ('future_version',1,'future sql')",
            )
            .execute(&mut db)
            .await
            .map_err(err)?;
            assert!(migrate(&mut db)
                .await
                .unwrap_err()
                .contains("latest launcher"));
            sqlx::query("DELETE FROM __launcher_migrations WHERE tag='future_version'")
                .execute(&mut db)
                .await
                .map_err(err)?;
            sqlx::query("UPDATE __launcher_migrations SET migration_sql='changed'")
                .execute(&mut db)
                .await
                .map_err(err)?;
            assert!(migrate(&mut db)
                .await
                .unwrap_err()
                .contains("changed after"));
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn upgrade_snapshot_includes_wal_data_and_failed_upgrade_can_be_retried() {
        let root = fixture();
        block(async {
            let mut db = connect(&root).await?;
            seed(&mut db).await?;
            // Every current migration, then a synthetic upgrade.
            let mut bad = MIGRATIONS.to_vec();
            bad.push((
                "synthetic_upgrade",
                "CREATE TABLE upgrade_data(id INTEGER); --> statement-breakpoint INVALID SQL",
            ));
            assert!(migrate_with(&mut db, &bad).await.is_err());
            let backup = std::fs::read_dir(root.join("database-backups"))
                .map_err(err)?
                .next()
                .ok_or("Missing upgrade backup")?
                .map_err(err)?
                .path();
            let mut saved = SqliteConnection::connect_with(
                &SqliteConnectOptions::new().filename(backup).read_only(true),
            )
            .await
            .map_err(err)?;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backups")
                .fetch_one(&mut saved)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            let mut good = MIGRATIONS.to_vec();
            good.push(("synthetic_upgrade", "CREATE TABLE upgrade_data(id INTEGER)"));
            migrate_with(&mut db, &good).await?;
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM __launcher_migrations WHERE tag='synthetic_upgrade'",
            )
            .fetch_one(&mut db)
            .await
            .map_err(err)?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn failed_upgrade_backup_blocks_schema_changes() {
        let root = fixture();
        block(async {
            let mut db = connect(&root).await?;
            seed(&mut db).await?;
            std::fs::write(
                root.join("database-backups"),
                b"synthetic blocked directory",
            )
            .map_err(err)?;
            assert!(migrate_with(
                &mut db,
                &[
                    MIGRATIONS[0],
                    MIGRATIONS[1],
                    ("synthetic_upgrade", "CREATE TABLE upgrade_data(id INTEGER)")
                ]
            )
            .await
            .is_err());
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE name='upgrade_data'")
                    .fetch_one(&mut db)
                    .await
                    .map_err(err)?;
            assert_eq!(count, 0);
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM __launcher_migrations")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, MIGRATIONS.len() as i64);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn broken_ownership_is_reported_without_removing_records() {
        let root = fixture();
        block(async {
            let mut db = connect(&root).await?;
            sqlx::query("PRAGMA foreign_keys=OFF")
                .execute(&mut db)
                .await
                .map_err(err)?;
            sqlx::query("INSERT INTO save_sets VALUES ('orphan','missing','/synthetic')")
                .execute(&mut db)
                .await
                .map_err(err)?;
            assert!(migrate(&mut db)
                .await
                .unwrap_err()
                .contains("inconsistent ownership"));
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM save_sets")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn wal_initialization_waits_for_an_existing_reader_without_losing_data() {
        let root = fixture();
        block(async {
            let mut reader = initial_database(&root).await?;
            seed(&mut reader).await?;
            let mut held = reader.begin().await.map_err(err)?;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
                .fetch_one(&mut *held)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            let copy = root.clone();
            let opening = tokio::spawn(async move { connect(&copy).await });
            tokio::time::sleep(Duration::from_millis(150)).await;
            assert!(!opening.is_finished());
            held.commit().await.map_err(err)?;
            let mut db = opening.await.map_err(err)??;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backups")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn simultaneous_first_open_and_reopen_apply_one_migration() {
        let root = fixture();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let root = root.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    block(async {
                        connect(&root).await?;
                        connect(&root).await?;
                        Ok(())
                    })
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        remove_test_dir(root);
    }
    #[test]
    fn failed_migration_rolls_back_schema_and_history() {
        let root = std::fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(operation_id());
        block(async {
            let mut connection = connect(&root).await?;
            let failure = apply_migrations(&mut connection, &[("synthetic_failure", "CREATE TABLE synthetic_atomic (id INTEGER); --> statement-breakpoint THIS IS NOT SQL")]).await;
            assert!(failure.is_err());
            let tables: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name='synthetic_atomic'").fetch_one(&mut connection).await.map_err(err)?;
            let history: i64 = sqlx::query_scalar("SELECT count(*) FROM __launcher_migrations WHERE tag='synthetic_failure'").fetch_one(&mut connection).await.map_err(err)?;
            assert_eq!((tables, history), (0,0));
            Ok(())
        }).unwrap();
        remove_test_dir(root);
    }
    #[test]
    fn generated_migrations_list_is_complete_and_foreign_keys_are_enforced() {
        let journal: serde_json::Value =
            serde_json::from_str(include_str!("../../src/db/migrations/meta/_journal.json"))
                .unwrap();
        assert_eq!(
            journal["entries"].as_array().unwrap().len(),
            MIGRATIONS.len()
        );
        for (entry, (tag, _)) in journal["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(MIGRATIONS)
        {
            assert_eq!(entry["tag"].as_str(), Some(*tag));
        }
        let root = std::fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(operation_id());
        block(async {
            let mut db = connect(&root).await?;
            assert!(
                sqlx::query("INSERT INTO save_sets VALUES ('orphan','missing','/synthetic')")
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            drop(db);
            let mut db = connect(&root).await?;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM __launcher_migrations")
                .fetch_one(&mut db)
                .await
                .map_err(err)?;
            assert_eq!(count, MIGRATIONS.len() as i64);
            Ok(())
        })
        .unwrap();
        remove_test_dir(root);
    }
}
