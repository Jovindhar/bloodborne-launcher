# bloodborne-launcher

The launcher for bloodborne-native. Tauri (Rust), React and TypeScript, with 20 languages.
What it has and what is not done yet: README's "Status"; the whole project's status and plan:
bloodborne-mac's `docs/STATUS.md` and `docs/ROADMAP.md`. Keep feature status in those docs, not here.
Keep profile/save logic independent of the platform and renderer; system differences go in
`src-tauri/src/platform.rs`, texts never name a system or a kind of machine ("this device", not
"this computer"). Do not claim Windows/Linux game integration before it is implemented and tested.

## Interface rules

- Layouts work from phone width up: check every layout change at phone width (390x844, touch, in
  the browser preview `npm run dev`) and on the desktop.
- Theme and colour changes are mirrored in the runtime's ImGui overlays (`Theme` in
  bloodborne-runtime's `gpu/shim/bbport_overlay.cpp`), so the in-game menus match the launcher.
- The memory estimate (`src/lib/data/memory.ts`) stays in step with `run.sh` and the in-game
  menu's `MemoryEstimate` (docs/SETTINGS.md).

## Layout and checks

- `src/`: frontend. Follow `README.md` conventions: one component per file, `@/` imports,
  folder exports, TanStack Query for backend data, Zustand for local state, all text in i18n.
- `src-tauri/src/`: Rust backend; synthetic save tests use temporary folders, never player data.
- `src/db/`: Drizzle schema and generated migrations; reads through `src/services/`. Rust owns
  transactions, migration application and file operations. Register new migrations in
  `src-tauri/src/database.rs`; `npm run db:generate` generates SQL. No component queries SQL directly.
  Applied migration tags and SQL are immutable: add a migration, never rename or rewrite one.
  Test upgrades from the previous schema, rollback/retry, WAL-aware snapshots and data preservation.
  Never reset a user's database to bypass an initialization failure; keep the recovery UI usable.
- `docs/`: developer docs (index: `docs/README.md`). Player guides live only in bloodborne-mac's
  `docs/guide/`. The wiki mirrors the docs.
- `npm ci && npm run check && npm run build`.
- `cd src-tauri && cargo test && cargo clippy -- -D warnings`.
- Packaged QA: `BB_LAUNCHER_DATA_DIR` selects an absolute synthetic data directory; empty/relative
  overrides fail before initialization. Keep player data outside every test.
- Standalone development: set `BB_REPO` to a bloodborne-runtime checkout for `npm run tauri dev`.
  Browser preview (`npm run dev`) uses an in-memory backend and does not need the game.

## Repositories and ownership

The launcher is independent, included as `launcher/` in bloodborne-mac. That app also owns
`runtime/` (bloodborne-runtime), which owns `recomp/` (bloodborne-recomp). Before editing this
submodule, checkout `main` and pull. Commit and push in the repository owning the files, then
bump the submodule pointer in the parent app and push it. Before every push, run
`git remote get-url origin`. Ask before force-pushing or tagging a release. The app's and the
launcher's versions stay equal: change them only with bloodborne-mac's `packaging/version.sh`.
Commit subjects are short; bodies explain what changed, why and measured results, with the session's co-author trailer.

## Data and game testing

Never write the player's saves/settings while testing. File tests use synthetic temporary data.
Game tests use bloodborne-runtime's `tools/test_run.sh NAME [VAR=...]` only, which copies saves.
Every test run says what it tests on screen: `TEST_TITLE="..." TEST_DETAIL="..."` set before it
put a banner at the top of the game's window (the title names the test and the build, the detail
the settings and what is checked).
Check `pgrep -x bb-probe` first, one game at a time. Tell the user before batches; stop when asked.
All tests use English (`BB_LANGUAGE=1 BB_UI_LANGUAGE=en`, and English launcher/helper UI).
Never capture the user's screen. Only the runtime's `tools/grab.sh` may capture game frames.
No game code, generated/decompiled game code, game files or game-memory records in Git or wikis.
The recompiled library and its generated C are game code: they stay in the player's data folder
on the device and never enter Git, releases or wikis. The save editor's names, numbers and icons
are read from the player's game files at run time and are never stored or shipped.
No `rm -rf` on variable shell paths; no `xargs ... sh -c`.

## Credits and docs

Keep bbport and shadPS4 credited in README, docs/CREDITS.md and the launcher wiki. Any adopted
third-party code needs credits in those docs, the wiki, the commit message and a short code
comment naming the source repository and commit. Docs use plain, precise English and measured numbers. Status
progress bars have 20 blocks, one per 5%. Docs say what is true now; investigation logs do not
go in docs (conclusions do).

## Save editor and trophies

The editor warns before applying changes, edits a staged copy, verifies it and marks it edited
(`savedata/.bb-edited`) before swapping it in (docs/SAVE_EDITOR.md). Playing an edited save must not
unlock trophies: the runtime does not record and the launcher does not import while the marker is
present. Copies and backups keep the marker; nothing removes it; existing profile trophies remain
(docs/TROPHIES.md). Edited saves play offline only: the launcher refuses online play for a marked
save set and the runtime's online module checks the marker again. Do not claim external-edit
detection or PSN synchronization.

Public trophy names/grades may be used, per the maintainer. Keep their source per locale in
`trophy_catalog.json`; use English fallback until a translation is verified. Descriptions are
original public-requirement summaries. Never extract trophy files/artwork or ship keys.
Keep Bloodborne and The Old Hunters separate; only base-game trophies determine platinum.
