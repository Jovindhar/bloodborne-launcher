# bloodborne-launcher

[![Discord](https://img.shields.io/badge/Discord-join%20the%20server-5865F2?logo=discord&logoColor=white)](https://discord.gg/QS6vHz45xA)

The independent launcher for [bloodborne-native](https://github.com/bloodborne-native).
Its source is included as the `launcher/` submodule of
[bloodborne-mac](https://github.com/bloodborne-native/bloodborne-mac), which packages it as the
macOS app around the game (`packaging/macos.sh`). Player guides are in the app's
[docs](https://github.com/bloodborne-native/bloodborne-mac/tree/main/docs/guide); the launcher's
own design is in [docs](docs/README.md).

The launcher has a game folder check, graphics, controls (controller choice and button mapping),
game options, online play, user profiles, saves with backups, a save editor, trophies, mods,
patches, the recompiled library ([Recompiled code](docs/RECOMPILED_CODE.md)), advanced options and
the game's log ([Pages](docs/PAGES.md)). It starts the game with the app's own bash and Python.

- **Interface:** React 19, TypeScript, Tailwind CSS 4, [shadcn/ui](https://ui.shadcn.com)
  (Base UI), lucide icons, react-i18next.
- **Backend:** [Tauri 2](https://tauri.app). The launcher is a library (`src-tauri/src/lib.rs`,
  `run()`); `main.rs` only starts it on desktops. It keeps the settings in its SQLite database
  (`settings_store.rs`; `launcher.json` and `bbport.ini` in the data folder are imported once and
  still written for the scripts) and in sync with the in-game menu while the game runs, checks the
  game folder (`game_check.rs`), lists mods and patches through `runtime/scripts/launcher_api.py`
  and controllers through `bb-gpu-capabilities`, and runs `run.sh`.

```bash
npm ci
npm run dev          # the interface in a browser, with mock data (src/lib/mockApi.ts); #graphics etc. opens a tab
BB_REPO=/path/to/bloodborne-runtime npm run tauri dev  # standalone: runtime build required
npm run check        # TypeScript, ESLint and the locales' keys
npx tauri build      # Bloodborne.app (packaging/macos.sh adds the game to it)
```

## Status

The launcher has: profiles and saves with backups and restore; the save editor (attributes, Blood
Echoes, Insight, items, weapons, armor, gems and runes; an edited save earns no trophies and plays
offline only); trophies with notices drawn by the game; settings shared with the in-game menu;
online play (Seamless Hunt and The Hunter's Dream); the Recompiled code page; mods and patches;
new-version notices; 20 languages; layouts from phone width up; one backend for every system
([Platforms](#platforms-and-screen-sizes)).

Not done yet:

- Import and export of complete save sets; managing the backup history (backups are not pruned).
- Game integration on Windows, Linux and Android.

The whole project's status and next steps:
[STATUS](https://github.com/bloodborne-native/bloodborne-mac/blob/main/docs/STATUS.md) and
[ROADMAP](https://github.com/bloodborne-native/bloodborne-mac/blob/main/docs/ROADMAP.md) in
bloodborne-mac; the save feature's stages:
[Profiles, saves and the database](docs/SAVE_MANAGER.md#stages).

## Platforms and screen sizes

The launcher is meant for macOS, Windows and Linux, and later Android and iOS. The game integration
exists on macOS only; the launcher itself also builds and runs on the iPhone (below).

- **System differences** live in `src-tauri/src/platform.rs`: the data folder (macOS
  `~/Library/Application Support/bloodborne_mac`; Linux `$XDG_DATA_HOME/bloodborne-native`, by
  default `~/.local/share/bloodborne-native`; Windows `%APPDATA%\bloodborne-native`; iOS the app's
  `Documents`, shown in the Files app), the
  package's layout, program names (`.exe`), the online module's file, opening a folder, finding
  and stopping the game, and the system language. The rest of the backend is the same everywhere;
  CI checks it on macOS at every push, and also on Ubuntu and Windows when run by hand
  (`gh workflow run checks.yml`; releases are macOS-only for now).
- **Texts** never name a system or a kind of machine ("this device", "Open folder"). Options that exist on one
  system only (Metal, MetalFX, the Metal HUD, MoltenVK) appear only there (`config.platform`).
- **Layout:** from Tailwind's `md` width (768 px) up, the sidebar with the tabs and Play; below it
  (phones, small windows) a tab strip at the top (`components/mobile/MobileHeader`) and the profile
  and Play in a bar at the bottom (`MobileBar`), inside the screen's safe areas. Setting rows put
  their control under the text below `sm` (640 px). The macOS title-bar strip is drawn only on
  macOS. Check every layout change at phone width (390×844, touch, in the browser preview) and
  on the desktop; `npm run dev` shows both layouts when the browser window is resized.
- **Theme:** colours are the CSS variables in `src/index.css`. The game's in-game menus use the
  same colours (`Theme` in the runtime's `gpu/shim/bbport_overlay.cpp`): mirror every change there.

## Conventions

These are enforced by ESLint (`eslint.config.js`) where possible.

- **One component per file** (`react/no-multi-comp`). shadcn's generated `src/components/ui` is
  exempt.
- **Imports go through the `@/` alias** (`src/`), never relative paths (`no-restricted-imports`).
- **Every folder has an `index.ts`** that exports its contents. Code outside a folder imports
  from the folder (`@/components/settings`, `@/hooks`, `@/lib/data`). Files inside a folder
  import their siblings by full path (`@/pages/controls/BindingRow`), so a folder's index is
  never imported from inside it.
- **A folder holds a feature and its parts.** For example, `components/sidebar` has the
  sidebar, its nav item and the Play button, and `pages/controls` has the page with its rows and
  pickers.
- **State:**
  - Backend data goes through **TanStack Query** (`src/hooks`): the config, the game check,
    displays, controllers, mods, patches, profiles (with their trophies), saves, the recompiled
    library, the FSR 4.1.1 check and the latest release; the save editor's characters, inventory,
    item catalog and icons. Settings use optimistic mutations; file operations wait for native
    confirmation.
  - Client state goes in **Zustand** stores (`src/stores`): the game process and its log
    (`game.ts`), the current tab and the character open in the editor (`ui.ts`), and the
    recompiled library's build progress (`recomp.ts`).
  - Keep component-local state in `useState`.
- **All text comes from i18n** (`src/i18n/locales/*.json`). `en.json` is the reference: its keys
  are typed (`i18next.d.ts`), so a missing key is a type error. Option lists store keys
  (`src/lib/data`), with `literal: true` for names shown as they are, such as language names.
- **UI components come from shadcn** (`npx shadcn@latest add <name>`, then export them from
  `src/components/ui/index.ts`).

### iOS

The iOS project (`src-tauri/gen/apple`, generated by `tauri ios init` and kept in Git) starts the
launcher's library (`lib.rs`, `run()`), as `main.rs` does on desktops. On iOS the
tray and the single-instance plugin are left out (`tray_mobile.rs` stands in for `tray.rs`), the
identifier and minimum version come from `tauri.ios.conf.json`, and Play does not start a program:
it calls `bb_ios_start_game`, which the app around the launcher provides
([bloodborne-ios](https://github.com/bloodborne-native/bloodborne-ios), private), on the main
thread (`platform::start_in_app`). Without it Play says the game is not in this build.

The project's Rust build phase is `scripts/ios-rust.sh` (the front end, then the library for
arm64 iPhones), not `tauri ios xcode-script`, which needs Xcode's iOS simulator platform and the
Tauri CLI running. Needs Xcode, `rustup target add aarch64-apple-ios` and, to regenerate the
project, `xcodegen` and CocoaPods.

```bash
cd src-tauri/gen/apple
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project bloodborne-launcher.xcodeproj \
    -scheme bloodborne-launcher_iOS -configuration debug -destination "id=<udid>" \
    -derivedDataPath ../../target/ios-xcode -allowProvisioningUpdates DEVELOPMENT_TEAM=<team> build
xcrun devicectl device install app --device <udid> ../../target/ios-xcode/Build/Products/debug-iphoneos/Bloodborne.app
```

## Layout

| Path | Contents |
|---|---|
| `src/pages/<tab>` | One folder per tab with the page and its parts ([Pages](docs/PAGES.md)); `profiles/` also has the Saves and Trophies tabs, `saves/` the character cards and `saves/editor/` the save editor |
| `src/components/ui` | shadcn/ui components |
| `src/components/{brand,common,mobile,profiles,settings,sidebar,trophies,trophy-emblem,updates,window}` | Shared parts: the moon and title, small common pieces, the phone header and bar, profile setup, recovery and the active-profile widget, setting rows, the sidebar with its tabs (`navTabs.tsx`) and Play, trophy lists and counts, the grade emblem, update notices, the window shell and backgrounds |
| `src/hooks` | TanStack Query hooks and small effects |
| `src/stores` | Zustand stores |
| `src/lib` | Backend calls (`api.ts`, `mockApi.ts` for the browser preview), types, query client, `data/` (options, controls, the memory estimate), `trophy-text/` (the trophy catalog) |
| `src/i18n` | i18next setup and locales: the game's 20 languages |
| `src/db`, `src/services` | Drizzle schema and migrations; typed reads |
| `src-tauri/src/lib.rs` | The backend: Tauri commands, settings files, the game's environment and launch |
| `src-tauri/src/{settings_store,database,profiles,saves}.rs` | Settings in SQLite; migrations; profile ownership and the file-operation journal; save sets, backups and restore |
| `src-tauri/src/{save_editor,inventory,character_growth,game_data}.rs` | Character preview and edits; inventory, weapons, armor and gems; HP and stamina progression; reading the player's game text, parameters and icons |
| `src-tauri/src/{trophies,trophy_catalog}.rs` | Importing unlocks and eligibility; the catalog |
| `src-tauri/src/game_check.rs` | Whether a folder holds the game the runtime runs |
| `src-tauri/src/recomp.rs` | Building and checking the recompiled library |
| `src-tauri/src/updates.rs` | The release check |
| `src-tauri/src/online_password.rs` | The saved online password |
| `src-tauri/src/{tray,tray_mobile,game_lifecycle}.rs` | The menu-bar icon and window while the game runs; process polling |
| `src-tauri/src/platform.rs` | What differs between systems |
| `src-tauri` | Tauri config, icons, `examples/tray_lifecycle.rs`, the iOS project (`gen/apple`) |
| `scripts/check_locales.py` | Checks every locale's keys against `en.json` |
| `scripts/ios-rust.sh` | The iOS project's Rust build phase |
| `scripts/make_icon.swift` | Draws the picture the app's icons are made from |

## Checks

```bash
npm run check && npm run build
cd src-tauri && cargo test && cargo clippy -- -D warnings
```

Native tests use synthetic data in temporary folders, never player saves. A packaged launcher
uses another data folder with `BB_LAUNCHER_DATA_DIR=/absolute/synthetic/dir` (empty or relative
values are refused before start-up): keep the player's data out of every test.

The browser preview (`npm run dev`) uses an in-memory backend. `#graphics` and the like open a tab;
query flags set the fake data: `?profile=Name` skips setup, `?more` adds two profiles,
`?backups` adds backups, `?edited` marks the save set edited, `?fresh` gives an empty one,
`?nogame` no game folder, `?lang=en` the launcher's language, `?update` a newer release
(`&empty-notes` without notes), `?recomp=ready` or `?recomp=stale` a built library, `?keys` the
keyboard bindings.

Database schema changes: `npm run db:generate`, then register the migration in
`src-tauri/src/database.rs` ([Profiles, saves and the database](docs/SAVE_MANAGER.md)).

## Documentation

[Pages](docs/PAGES.md) · [Settings](docs/SETTINGS.md) ·
[Profiles, saves and the database](docs/SAVE_MANAGER.md) · [Save editor](docs/SAVE_EDITOR.md) ·
[Trophies](docs/TROPHIES.md) · [Recompiled code](docs/RECOMPILED_CODE.md) ·
[While the game runs](docs/LAUNCHER_LIFECYCLE.md) · [Updates](docs/UPDATES.md) ·
[Credits and license](docs/CREDITS.md) ([index](docs/README.md))

## Credits

The project is built on [bbport](https://github.com/deadinside28/bloodborne_pc) and
[shadPS4](https://github.com/shadps4-emu/shadPS4). The SQL adapter and window handling are
adapted from [Leari](https://github.com/leari-mail/leari/tree/5876482) (MIT). See
[Credits and license](docs/CREDITS.md).
