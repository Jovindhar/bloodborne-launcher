# Credits and license

[← Documentation](README.md)

The launcher was developed in [bloodborne-mac](https://github.com/bloodborne-native/bloodborne-mac)
and moved into this repository with its history. It is licensed under GPL-2.0-or-later; see
[LICENSE](../LICENSE).

The runtime it launches is built on [bbport](https://github.com/deadinside28/bloodborne_pc) by
deadinside28 and [shadPS4](https://github.com/shadps4-emu/shadPS4). Their credits and the other
runtime contributors are in
[bloodborne-runtime's credits](https://github.com/bloodborne-native/bloodborne-runtime/blob/main/docs/CREDITS.md).

## Dependencies

The interface uses Tauri, React, shadcn/ui, Base UI, Tailwind CSS, TanStack Query, Zustand,
i18next, Sonner, lucide, [react-markdown](https://github.com/remarkjs/react-markdown) (release
notes) and the Cinzel and Inter fonts; the backend also uses reqwest, rustls, semver and Tauri's
opener plugin. They are package dependencies; their licenses remain with their packages.

## Adapted code

- **Leari** ([leari-mail/leari `5876482`](https://github.com/leari-mail/leari/tree/5876482), MIT,
  Pablo Souza): the Drizzle-to-Tauri SQL adapter in `src/db/client.ts`, and the window restoration
  and Dock/taskbar handling in `src-tauri/src/tray.rs`. Its service/hook organization, shared
  SQLite access and tray setup informed the launcher's design. The
  [MIT notice](licenses/leari-MIT.txt) is kept here. No Leari icon or artwork was copied; the
  hunter's sigil in the tray is original.
- **shadcn/ui** ([shadcn-ui/ui `30a58d4`](https://github.com/shadcn-ui/ui/tree/30a58d4498f270af1c1e981a7d78a15cf5740841/apps/v4/registry/bases/base/ui),
  MIT): the components in `src/components/ui/` were generated with the shadcn CLI from the
  base-nova registry (Base UI templates). The
  [MIT notice](licenses/shadcn-ui-MIT.txt) is kept here.

No code from an external save editor or update checker was copied.

## Save editor

Field locations in the character save were checked against
[Noxde/Bloodborne-save-editor `d8d0365`](https://github.com/Noxde/Bloodborne-save-editor/tree/d8d03658f288fe747f819d0dc9a09c5f9d2e1668)
(GPL-3.0). Its source is not copied: the parser, validation, staged whole-set edit, backup and
edited-save handling are the launcher's own. The game-file formats it reads (DCX, BND4, FMG,
PARAM, TPF) are the ones documented by the FromSoftware modding community (SoulsFormats); the
readers in `game_data.rs` are the launcher's own.

The base HP and stamina progression used when Vitality or Endurance changes was checked against
the public [Bloodborne Wiki Vitality](https://www.bloodborne-wiki.com/2015/05/vitality.html) and
[Endurance](https://www.bloodborne-wiki.com/2015/05/endurance.html) tables
([CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)). No wiki text is included.

Item, weapon, armor, gem and rune names, descriptions, numbers and icons are read from the
player's own game files while the launcher runs. None of them is in this repository or shipped
with the app.

## Trophies

Public names and grades follow [PSN100](https://psn100.net/game/3609-bloodborne); the trophy IDs
were cross-checked against
[Supermedo/bloodborne_pc `79d44f0`](https://github.com/Supermedo/bloodborne_pc/commit/79d44f0eecc2eed45c1e33b48996e53626adca48)
by Ambrose (GPL-2.0-or-later), whose event recording the runtime's follows. Localized names come
from [Exophase's public lists](https://www.exophase.com/game/bloodborne-ps4/trophies/);
`trophy_catalog.json` records each locale's source. The English lists differ in one UK/US spelling
(ID 2); the launcher follows PSN100. Requirement descriptions are original summaries of the public
requirements in [Gamepressure](https://www.gamepressure.com/bloodborne/list-of-all-trophies/z7aad1),
PSN100 and [a Brazilian Portuguese guide](https://magnaway.com.br/guias/38082/guia-de-trofeus-bloodborne/).
The importer, eligibility checks and trophy pages are project code. No extracted game metadata,
key, game description text or artwork is included; grade emblems are lucide's trophy in original colours.

## Images

`src/assets/home-hero.webp` (the night city behind Home's title) and
`src/assets/page-background.webp` (a night street, darkened behind the whole window, menu and page,
into which Home's picture fades) are AI-generated images supplied by the maintainer: generic gothic
scenes with no character, weapon or place from the game. Home's game tile is read from the player's
dump (`sce_sys/icon0.png`) when shown and never stored. `scripts/make_icon.swift` draws the app
icon; the tray's hunter's sigil is original.

## LLVM toolchain

The Recompiled code page downloads clang and `ld64.lld` from [LLVM](https://llvm.org) 23.1.4
(Apache-2.0 with LLVM exception), trimmed from LLVM's official release by bloodborne-recomp's
`tools/recomp/toolchain.sh` and published with LLVM's license as that repository's
`toolchain-llvm-23.1.4` release. The launcher does not ship it and deletes it once the library is
built.
