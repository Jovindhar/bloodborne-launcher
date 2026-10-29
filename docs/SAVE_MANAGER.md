# Profiles, saves and the database

[← Documentation](README.md)

How the launcher owns profiles, saves, backups and trophy history, and how it keeps them safe.
The editor is in [Save editor](SAVE_EDITOR.md), trophies in [Trophies](TROPHIES.md). The
player's view is the app's
[Profiles and saves](https://github.com/bloodborne-native/bloodborne-mac/blob/main/docs/guide/saves.md)
guide.

## Stages

The save feature's status. Done: **3 of 4 stages (75%)** `███████████████░░░░░`

| Stage | Scope | Status |
|---|---|---|
| S0 | Profile ownership, first-profile setup, complete save sets, active selection, backup and restore, deletion, local trophies | Done |
| S1 | Read-only character preview: name, level, attributes, Insight, Blood Echoes, playtime, journey | Done |
| S2 | Import and export of complete save sets; backup history management (pruning old backups) | Not started |
| S3 | Editor for verified fields, with edited-save lineage | Done: attributes, Blood Echoes, Insight; item amounts and additions; weapon levels, imprints and gem slots; adding weapons and armor; gem and rune effects; adding gems ([Save editor](SAVE_EDITOR.md)) |

**Not yet:** S2 above (also in the app's
[ROADMAP](https://github.com/bloodborne-native/bloodborne-mac/blob/main/docs/ROADMAP.md), Launcher).
Backups accumulate until removed by hand. Additions to a file that holds two copies of the
character are refused. The launcher does not detect saves edited by other tools, and nothing
synchronizes with PSN.

## Profiles

A profile is an identity with exactly one complete save set (which can hold several characters),
its backups and its trophy history. Profiles are created, renamed, selected and deleted on the
Profiles tab; the active one is the widget above Play, and its saves and trophies are the Saves
and Trophies tabs ([Pages](PAGES.md)). Names are trimmed and compared in Unicode lowercase for
duplicates; ids and paths never change on rename. New profiles start empty: saves are never
copied between profiles.

**First setup.** A new installation, or an upgrade from a launcher without ownership records,
shows a required first-profile window; Play (and `--play`) is refused until it is done. The
entered name takes ownership of the existing user folder without moving its files; earlier
development profile folders are adopted too. Setup cannot be repeated over established
ownership. A damaged database reports an error; it is never treated as a fresh installation.

**Deletion.** Only an inactive profile, never the last one, after a confirmation dialog. The
database marks the profile deleted and records a pending cleanup in one transaction, then removes
its folder and backups (for the original profile only `savedata/` and its backups, keeping the
shared shader caches). Folder ancestors are checked and links refused. An interrupted cleanup is
retried at startup. The profile row stays as a tombstone so leftover folders are never adopted
again.

## Storage

In the data folder (see the README's [Platforms](../README.md#platforms-and-screen-sizes);
`BB_LAUNCHER_DATA_DIR` replaces it for tests):

| Where | What |
|---|---|
| `launcher.db` | SQLite: profiles, save sets, the active profile, backups, trophy unlocks, settings, the file-operation journal |
| `user/` | The first profile's saves, and the shared shader and pipeline caches |
| `save-profiles/<id>/user/` | Other profiles' saves, with a metadata manifest for recovery |
| `save-backups/<id>/` | Each profile's backups |
| `database-backups/` | A snapshot of the database before each schema upgrade |

At launch the launcher sets `BB_USER_DIR` to the active profile's folder, `BB_USER_NAME` to its
name (the game sees at most 15 bytes) and `BB_GPU_USER_DIR` to the original folder for the shared
caches. The game always sees user id 1; profiles are separated by folder.

## The database

The schema is Drizzle's (`src/db/schema.ts`; `npm run db:generate`). Rust applies the generated SQL
files (registered in `src-tauri/src/database.rs`), each migration with its history entry in one
transaction, after a `VACUUM INTO` snapshot. Startup checks the history and the actual tables. An
older launcher refuses a newer database. Migrations are immutable once applied: add a new one,
never rename or rewrite one. Native connections use WAL, foreign keys and a busy timeout. The
frontend reads through generated SELECT queries only (`src/services/`); every write is a native
command. The structure follows [Leari](https://github.com/leari-mail/leari/tree/5876482).

**Recovery.** Startup and configuration failures show a recovery screen with the error, **Retry**
and **Open folder** (`components/profiles/ProfileRecovery`). Never delete `launcher.db` to recover:
it owns identity and trophy history. Partial or unknown schemas are kept and reported, never
reset.

## Backups and restore

- A backup copies the whole `savedata/` subtree to a staging folder and publishes it when
  complete. One is made before every launch, every restore and every edit, and with **Create
  backup** on the Saves tab. Each backup is marked edited or original.
- A restore prepares the replacement beside the live folder, keeps the previous one, and rolls
  back if the swap fails. An interrupted restore blocks the profile and names its recovery path.
  Restores keep the profile's identity and trophy history.
- Every file operation is journaled; unfinished ones are marked interrupted at startup after
  reconciling folders with the database.
- Nothing changes while a game runs (the game mutex and `pgrep -x bb-probe`). The launcher is a
  single instance.
- Database transactions do not make folder copies atomic: this is not a power-loss guarantee.

## Character preview

The Saves tab lists the readable characters of the save set (`save_editor.rs`, `list_characters`):
name, level, attributes, Blood Echoes, Insight, playtime and journey. Ambiguous or malformed
character layouts are refused, not shown.

## Checks

`cd src-tauri && cargo test` (synthetic data in temporary folders: migrations, upgrades and
snapshots, recovery, ownership, copies, restores, deletion, trophy import and eligibility, the
editor) and `npm run check && npm run build`. The browser preview (`npm run dev`) uses an
in-memory backend: `?profile=Name` skips setup, `?more` adds two profiles, `?backups` adds an
edited and an original backup, `?edited` marks the save set edited, `?fresh` gives an empty one.
No test touches the player's saves.
