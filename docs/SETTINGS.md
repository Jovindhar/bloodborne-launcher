# Settings

[← Documentation](README.md)

Where the settings are stored, how they reach the game and how they stay in step with the in-game
menu. Which page shows which setting: [Pages](PAGES.md).

## Where they live

All settings are rows of the `settings` table in `launcher.db`, shared by all profiles
(`src-tauri/src/settings_store.rs`). The scope `launcher` holds the launcher's options (JSON
values); the scope `game` holds the game's (`bbport.ini`'s keys and text values). `bbport.ini`
(read by `run.sh` and the runtime) and `launcher.json` are written from the table; on the first
start of a launcher with the table they are imported from those files once.

## How they reach the game

Game settings reach the game through `bbport.ini`. Launcher options reach it as environment
variables set in `game_environment` (`src-tauri/src/lib.rs`), for example `renderer` →
`BB_METAL_FRAME` and `BB_METAL_NATIVE_BOOT` (both set either way on macOS), `fps_limit` →
`BB_FPS_LIMIT`, `trophy_notifications` → `BB_TROPHY_OVERLAY`, and the active profile's folders
and name → `BB_USER_DIR`, `BB_GPU_USER_DIR` and `BB_USER_NAME`. **Extra environment variables**
(Advanced) are added after the options and before the profile's variables, which they cannot
override.

## Sync with the game

While the game runs, the launcher and the in-game menu exchange changes as lines on the game's
standard streams: `BBSET key=value` for game settings and `BBSET launcher:key=value` for the
launcher options the menu shows: fullscreen, display, presentation, frame-rate mode and limit,
the Metal HUD, draw pipeline, readbacks, preupload and renderer (`GAME_SYNCED` in
`settings_store.rs`). The backend stores what the menu sends before the interface hears of it
(`src/stores/game.ts`). Each side applies what it can at once; the menu offers a restart for the
rest.

## The memory estimate

`src/lib/data/memory.ts` computes what the game will use at the chosen output resolution, as
`run.sh` sizes it, plus the renderer's measured share, against the device's memory; Graphics shows
it under the resolution. The in-game menu computes the same (`MemoryEstimate` in the runtime's
`gpu/shim/bbport_overlay.cpp`). Keep the three in step.
