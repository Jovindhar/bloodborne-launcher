# Settings

[← Documentation](README.md)

## Where they live

All settings are rows of the `settings` table in `launcher.db` (scopes `launcher` and `game`),
shared by all profiles (`src-tauri/src/settings_store.rs`). `bbport.ini` (game settings, for
`run.sh` and the runtime) and `launcher.json` are written from them; on first start they are
imported from those files once. Launcher options reach the game as environment variables
(`main.rs`: for example `renderer` → `BB_METAL_FRAME`, `fps_limit` → `BB_FPS_LIMIT`).

**While the game runs** the launcher and the in-game menu exchange changes as lines on the game's
standard streams: `BBSET key=value` for game settings and `BBSET launcher:key=value` for the
launcher options the menu shows (fullscreen, display, presentation, frame-rate mode and limit,
Metal HUD, draw pipeline, readbacks, preupload, renderer: `GAME_SYNCED` in
`settings_store.rs`). Each side applies what it can at once; the menu offers a restart for the
rest.

## Tabs

Settings are grouped by what the player controls. Moving a control never changes its stored key,
default or effect; translations move with it in all 20 locales.

| Tab | Contents |
|---|---|
| Home | Active profile and trophies, Profiles/Saves shortcuts, game folder and edition, a summary of the main settings |
| Graphics | Aspect ratio (Auto, 4:3, 16:10, 16:9, 21:9, 32:9) and the output resolutions of that ratio, with the memory estimate; monitor, fullscreen, presentation, frame-rate mode and limit, FPS counter (size, position), Metal HUD; upscaling (off by default; FSR 3.1; on macOS MetalFX, and FSR 4.1.1 on Metal when `bb-gpu-capabilities --fsr411` says this Mac can run it, with a button that takes AMD's two files from the player's own DLL into the data folder's `fsr4_411`; with the Metal renderer only MetalFX and off, and a saved FSR choice shows and runs as MetalFX), sharpening, motion vectors, model detail, effects (all but SSR switch while the game runs) |
| Controls | Controller choice and controller/keyboard bindings |
| Game | Game language, skipping intro videos, the active profile's save folder |
| Online | Online play through Seamless Hunt (bbport 0.6's online module): the switch (`BB_ONLINE`), the Online ID (`BB_SHADNET_NPID`) and its password, kept in the macOS Keychain (service "bloodborne_mac online", account the Online ID; never in the database; `BB_SHADNET_PASSWORD` only in the game's environment), seamless co-op (`BB_BBSC`), UPnP (`BB_UPNP`), BBSC's game server on this Mac (`BB_BBSC_LOCAL_SERVER`: auto, on, off) and the shadNet server (`BB_SHADNET_SERVER`, empty for Seamless Hunt's) (`src-tauri/src/online.rs`) |
| Profiles | Profiles, each with Saves · Backups and Trophies |
| Mods, Patches | Mods and their order; third-party patches |
| Advanced | Launcher language and hiding during gameplay, updates, trophy notifications; the renderer (Metal, the default, or MoltenVK; the launcher sets `BB_METAL_FRAME` either way) and other performance options, live resolution; free camera, debug menu, logs, profiling, extra environment variables; data folders |
| Log | The game's output |

The memory estimate (`src/lib/data/memory.ts`) computes what the game will use as `run.sh` sizes
it, plus the renderer's measured share, against the Mac's memory; the in-game menu computes the
same (`MemoryEstimate` in the runtime's `bbport_overlay.cpp`). Keep the three in step.
