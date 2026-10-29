# Trophies

[← Documentation](README.md)

The runtime records unlocks and draws their notices in the game's own ImGui overlay; the launcher
imports the unlocks into each profile's history and shows the list. How the runtime records and
delivers them (the event file, the notice queue and timings, the overlay):
[bloodborne-runtime's Local trophies](https://github.com/bloodborne-native/bloodborne-runtime/blob/main/docs/TROPHIES.md).
The launcher draws no notifications and starts no other window.

## Import

The runtime appends unlocks to `trophies-v1.log` in the profile's folder. `trophies::refresh`
(`src-tauri/src/trophies.rs`) imports them into the database's `trophy_unlocks`, keeping the
earliest time of each trophy, every time the profile list is read (`list_profiles`):

- at startup and after any profile or save change;
- every 5 seconds while the game runs (`useProfiles`; the active-profile widget uses it, so this
  happens on any page);
- at once when the game prints `Runtime: trophy N unlocked` (`src/stores/game.ts`), also while the
  launcher is hidden and timed refetches pause.

A file the launcher cannot read sets the profile's `trophy_error`; its history and the other
profiles stay usable.

## Catalog and locales

`src/lib/trophy-text/trophy_catalog.json` lists 40 trophies: Bloodborne 34, The Old Hunters 6. Names
are in 15 locales; requirement summaries in English, Brazilian Portuguese and French. Anything
missing falls back to English. The file records each locale's source; the Rust side
(`trophy_catalog.rs`) embeds the same file. Names and grades are public; descriptions are original
summaries ([Credits](CREDITS.md#trophies)). Bloodborne and The Old Hunters are separate groups; only
the base game's trophies count toward platinum. Unlocks the catalog does not know are shown in a
group of their own.

## Eligibility

**An edited save never unlocks trophies.** The marker `savedata/.bb-edited` travels with the save
through copies, backups and restores. While it is present the runtime does not record, and the
launcher does not import (`trophies::eligible`; a `savedata` that is a link or not a folder is
ineligible too). The editor writes the marker before publishing an edited save
([Save editor](SAVE_EDITOR.md#how-an-edit-is-applied)) and nothing removes it; a clean pre-edit
backup keeps its own eligibility. A restore never removes earned history.
A marked save set also plays offline only: the launcher refuses online play for it and the
runtime's online module checks the marker again.

## The Trophies page

`src/pages/profiles/TrophiesPage.tsx` shows the active profile's trophies: a note that they are tracked
on this device only and earlier progress is not imported, a warning when the save set is edited or the import failed
(`TrophyStatus`), then one panel per game with its count (earned/total). The filter (All, Unlocked,
Locked) and the order (as in the game, most recent, locked first, highest or lowest grade; ties
keep the game's order) apply to every panel (`components/trophies/TrophyGroup`). The
active-profile widget and the profile menu show each profile's count (`TrophyCount`), the trophy
slashed when trophies are off or failed to import.

## The notification setting

**Advanced → Launcher → Trophy notifications** (`trophy_notifications`, on by default). Off, the
launcher passes `BB_TROPHY_OVERLAY=0` at the next launch: the game shows no notices but still
records unlocks. For a test on copied saves, `BB_TROPHY_OVERLAY_PREVIEW=1` (Extra environment
variables) shows labelled preview notices without recording anything.
