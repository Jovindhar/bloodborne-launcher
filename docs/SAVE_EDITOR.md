# Save editor

[← Documentation](README.md)

The editor changes one character of the active profile's save set. It opens from **Edit
character** on a character card (Saves) and replaces the page until it is closed. Profiles,
storage, backups and recovery: [Profiles, saves and the database](SAVE_MANAGER.md); its stage in
the save feature: that page's [Stages](SAVE_MANAGER.md#stages). The player's view is the app's
[Profiles and saves](https://github.com/bloodborne-native/bloodborne-mac/blob/main/docs/guide/saves.md)
guide.

## What it edits

| Section | What can change | Limits |
|---|---|---|
| Character | Vitality (7–99), Endurance (8–99), Strength, Skill, Bloodtinge, Arcane (1–99), Blood Echoes (0–999,999,999), Insight (0–99); **Max all** sets each to its maximum (level 544) | Level is recomputed from the six attributes; the three saved HP and stamina values follow Vitality and Endurance from the game's base progression (`character_growth.rs`), so runes and equipment may change what the game then shows |
| Items, Storage | Amounts; adding consumables, materials and chalices; Storage also takes weapons and armor | Amounts up to the game's `maxNum` in the inventory and 99 in storage; key items cannot be added |
| Weapons | Level and imprint; gems in its slots; adding weapons | A gem in a slot that closes or no longer fits its shape must come out first |
| Armor | Adding armor | — |
| Gems | Each gem's or rune's effects, among the strengths the game has for that kind of effect (gems and runes never mix); adding gems of a chosen shape | Rune shapes 1, 2, 4 and 16 are accepted (16 is written by the game) |

The editor refuses what it does not recognize: ambiguous or malformed character layouts are not
previewed, and nothing can be added to a file that holds two copies of the character.

## How an edit is applied

One edit carries every change on the page, character and inventory together (`edit_save` in
`lib.rs`; `saves::edit`). The game must be stopped and the preview must still match the character
file (its fingerprint). The editor warns before applying, then:

1. **Staged:** the whole `savedata/` is copied to a staging folder beside it.
2. **Edited and verified:** the character file is changed there (`save_editor.rs`,
   `inventory.rs`); the edited bytes are parsed again and every changed field, entry and gem effect
   compared with what was asked before the file is written.
3. **Marked:** `savedata/.bb-edited` is written into the staged set. From then on it earns no
   trophies and plays offline only ([Trophies](TROPHIES.md#eligibility)); nothing removes the marker.
4. **Backed up:** the live set is backed up (a clean set's backup stays eligible).
5. **Swapped:** the live folder moves aside, the staged one takes its place, and the old one comes
   back if the swap fails. An interrupted swap blocks the profile and names the recovery folder.

## Save-format facts

Field locations were checked against
[Noxde/Bloodborne-save-editor `d8d0365`](https://github.com/Noxde/Bloodborne-save-editor/tree/d8d03658f288fe747f819d0dc9a09c5f9d2e1668);
the parser is the launcher's own and validates the structure before exposing any field.

- **Entries:** the inventory (1,983 entries) and storage (1,984) follow the character name. An entry
  is 16 bytes: an ordering number, a handle (its high byte the kind: `0x80` weapon, `0x90` armor,
  `0xB0` item, `0xC0` gem or rune), the ID (a weapon's carries its level and imprint) and the
  amount. A free entry has the ID `0xFFFFFFFF`.
- **The table before the character:** 4,096 entries from byte 84, each a gem or rune record (40
  bytes), a weapon's or armor piece's block (60 bytes, with its five gem slots) or a free entry (8
  bytes). The character starts 620 bytes after the table, so it moves when the table grows.
- **Weapons:** a level or imprint change rewrites the ID in the entry and in the block, and sets the
  block's slots from the game's `WeaponGenParam` (one row per weapon ID): each slot's shape where
  it first opens.
- **Adding:** an item takes a free entry. A weapon or armor piece reuses a free block of its kind
  that no entry or other byte refers to. A gem takes a free 8-byte table entry: everything after it
  moves 32 bytes on and the fixed-size file (1,310,720 bytes) drops its last 32; gems are applied
  last in an edit, and the checked offsets move with them. Its source (the record's second word and
  the entry's ID) is `0x80000000` with the `GemGenParam` row of a gem with that one fixed effect.
  The game loads such a save and writes it back with the gem.

## Game data

Names, descriptions, numbers and icons come from the player's own game files while the launcher
runs; nothing of them is stored in the repository or shipped. Without a readable game folder the
editor stays closed and says why.

- **Text** (`game_data.rs`): `dvdroot_ps4/msg/<language>/item.msgbnd.dcx` (DCX around a BND4 of
  FMG tables) in the launcher's language.
- **Parameters:** the item, weapon, armor, gem-effect, weapon-slot and gem-source tables
  (`EquipParamGoods`, `EquipParamWeapon`, `EquipParamProtector`, `GemeffectParam`, `WeaponGenParam`,
  `GemGenParam` from `param/gameparam/gameparam.parambnd.dcx`), for amounts, stats, slots and
  effect choices.
- **Icons:** `dvdroot_ps4/menu/menu_knowledge_NNNNN.tpf.dcx`, untiled and decoded to PNG by
  `game_data::icon_png`; `item_icon` returns a data URL, cached for the launch. A gem's or rune's
  icon is its shape's first icon in the menu set (3000 radial, 3200 triangle, 3400 waning, 3600
  circle, 4000 droplet; runes from 3000) plus its first effect's `gemIconIdOffset`.

## The interface (`src/pages/saves/editor/`)

`SaveEditor` has the sections Character, Items, Weapons, Armor, Gems and Storage, saved together
with one confirmation and one backup (`useSaveEdits` keeps the pending changes). Weapons and gems
are a list with the chosen one in full beside it (below it on narrow windows: `WeaponDetail`,
`GemDetail`, `GemSlots`). Each row's details (`ItemStats`, `StatsTable`) are a hover card with a
mouse and a window on touch. **Add** opens `AddItemDialog` (`AddItemPanel`): it stays open to add
more, and what was added waits at the top of the list (`PendingAdditions`) until the edit is saved.
`useItemCatalog` reads the catalog once per game folder and language.

## Checks

`cd src-tauri && cargo test` covers the parser, edits, verification, additions and gem moves on
synthetic files in temporary folders, never the player's saves.
