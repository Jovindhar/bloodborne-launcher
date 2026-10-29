# Pages

[← Documentation](README.md)

The launcher's tabs, in the order of `NAV_GROUPS` (`src/components/sidebar/navTabs.tsx`). From
Tailwind's `md` width (768 px) up they are in the sidebar under their group's title; below it they
are a strip at the top, in the same order. Each tab is a folder in `src/pages/` (`src/stores/ui.ts`
holds the current tab; `#graphics` and the like in the URL open one).

The active profile is a widget above Play (on phones, in the bottom bar with Play): its name and
recorded trophies (`components/trophies/TrophyCount`, a slashed trophy when they are off or failed
to import), and a menu to switch profile or open Saves, Trophies or Profiles
(`components/profiles/ActiveProfile`).

Moving a control between pages never changes its stored key, default or effect; its translations
move with it in all 20 locales.

## Play (no title)

| Tab | Folder | What it holds |
|---|---|---|
| Home | `home/` | A night-city picture with the title and Play; a row of buttons with the upscaler and render size, frame-rate mode, controller and mods, each opening its page; the last run's exit; the game card: the game's own tile from the player's dump (`sce_sys/icon0.png`, read when shown, never stored), title, region, version, whether The Old Hunters is included, whether the game can start (`game_check.rs`), and its folder |
| Saves | `profiles/SavesPage`, `saves/` | The active profile's save set: its characters (name, level, attributes, Insight, Blood Echoes, playtime, journey), each with **Edit character** opening the save editor in place of the page ([Save editor](SAVE_EDITOR.md)); **Create backup**, the folder, and the backups (each marked edited or original) with Restore ([Profiles and saves](SAVE_MANAGER.md)) |
| Trophies | `profiles/TrophiesPage` | The active profile's trophies by game, filtered and sorted ([Trophies](TROPHIES.md)) |
| Profiles | `profiles/` | Every profile: use, rename, delete (with confirmation), and create a new one |
| Online | `online/` | Online play on or off; the pool (Seamless Hunt or The Hunter's Dream); that pool's account (the Seamless Hunt password saved only after a warning, below), servers and UPnP; a note that seamless co-op (BBSC) is not supported. A warning when the active profile's save set is edited: it cannot go online |

The Seamless Hunt password is saved only after the player accepts that it is stored unencrypted
on this device, in a file of its own readable by their account alone (`online_password.rs`;
`<data>/online-password`, not in the database, its backups or `launcher.json`). Otherwise Play asks
for it each time and it goes to that launch only.

## Settings

| Tab | Folder | What it holds |
|---|---|---|
| Graphics | `graphics/` | Aspect ratio (Auto, 4:3, 16:10, 16:9, 21:9, 32:9) and the output resolutions of that ratio, with the memory estimate; monitor, fullscreen, presentation; frame-rate mode and limit, FPS counter (size, position), the Metal HUD (macOS); upscaling (below), sharpening and its strength, object motion vectors; model detail; effects (all but SSR switch while the game runs) |
| Controls | `controls/` | The controller, and controller and keyboard bindings with a reset |
| Game | `game/` | The game's language, skipping the intro videos, the active profile's save folder |
| Mods | `mods/` | Mods on or off, their folder, the list with each mod's switch and order |
| Recompiled code | `recomp/` | Translated or recompiled game code, and building the library ([Recompiled code](RECOMPILED_CODE.md)) |

**Upscaling.** Off by default. With the Metal renderer (the default on macOS) the choices are
MetalFX and off; a saved FSR choice shows and runs as MetalFX there. With the MoltenVK renderer the
choices are FSR 3.1, MetalFX (macOS), FSR 4.1.1 and off. FSR 4.1.1 is offered on macOS when
`bb-gpu-capabilities --fsr411` says the device's GPU can run it (or while it is the saved choice),
with a row that copies AMD's two files from the player's own DLL into the data folder's
`fsr4_411/` when they are missing (`Fsr411FilesRow`).

## Developer

| Tab | Folder | What it holds |
|---|---|---|
| Patches | `patches/` | Third-party patches, their folder and switches |
| Advanced | `advanced/` | Launcher: its language, hiding during gameplay ([lifecycle](LAUNCHER_LIFECYCLE.md)), trophy notifications ([Trophies](TROPHIES.md)); updates ([Updates](UPDATES.md)); performance: the renderer (macOS: native Metal by default, MoltenVK as a fallback), draw pipeline, readbacks, preupload, live resolution; diagnostics: free camera, debug menu, frame statistics, saving the log, crash diagnostics, GPU profiling, extra environment variables; the data and backup folders |
| Log | `log/` | The game's output, following it, copy, clear, and the logs folder |
