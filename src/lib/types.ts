// Shapes the Tauri backend (src-tauri/src/main.rs) sends and receives.

/** launcher.json: start-up options passed to run.sh as environment variables. */
export type Settings = {
  game_dir: string;
  user_dir: string;
  save_profile: string;
  mods_dir: string;
  mods_enabled: boolean;
  patches_dir: string;
  language: string;
  fullscreen: boolean;
  /** The game's monitor by name (BB_DISPLAY); "" = the main display. */
  display: string;
  hdr: boolean;
  present_mode: string;
  gamepad: string;
  gamepad_name: string;
  fps_mode: string;
  fps_limit: number;
  draw_pipe: string;
  readbacks: string;
  preupload: string;
  frame_stats: boolean;
  save_log: boolean;
  /** Automatically hide the launcher during gameplay; does not disable logging. */
  hide_launcher: boolean;
  trophy_notifications: boolean;
  /** macOS: Apple's Metal Performance HUD over the game (MTL_HUD_ENABLED). */
  metal_hud: boolean;
  /** macOS: "moltenvk" (default) or "metal" (the Metal backend's frame path). */
  renderer: string;
  /** How the game's code runs: "translated" (bbcpu's translator) or "recompiled" (the library built
   * on the Recompiled code page, used only while it matches the game and the recompiler). */
  cpu: "translated" | "recompiled";
  online: boolean;
  online_pool: "seamless" | "huntersdream" | "yharnamtogether";
  online_server: string;
  online_webapi: string;
  online_community: string;
  online_npid: string;
  /** The player accepted saving the password unencrypted on this computer; else asked at each launch. */
  online_save_password: boolean;
  online_upnp: boolean;
  /** BloodOnline: BBSC seamless co-op (BB_BBSC), Seamless Hunt only. */
  online_coop: boolean;
  /** BloodOnline: BBSC's game server on this computer (BB_BBSC_LOCAL_SERVER). */
  online_local_server: "auto" | "on" | "off";
  thd_server: string;
  thd_online_id: string;
  thd_token: string;
  /** BloodOnline: the Yharnam Together host's address (online_pool "yharnamtogether"). */
  yt_host: string;
  check_updates: boolean;
  crash_diag: boolean;
  gpu_profile: boolean;
  extra_env: string;
  /** Launcher language ("" = the system's). */
  ui_language: string;
};

/** The game folder's edition, from its sce_sys/param.sfo (scripts/game_check.py describe). */
export type GameEdition = {
  title_id: string;
  title: string;
  version: string;
  region: "america" | "europe" | "japan" | "asia" | "unknown";
  /** A Game of the Year edition: The Old Hunters is part of the game. */
  old_hunters: boolean;
};

/** The game check: why the folder cannot start (null when it can) and its edition. */
export type GameCheck = { problem: string | null; game: GameEdition | null };

/** bbport.ini: the port's settings, shared with the in-game menu. */
export type Ini = Record<string, string>;

export type Config = {
  settings: Settings;
  ini: Ini;
  paths: { data: string; mods: string; patches: string; logs: string; saves: string };
  bundled: boolean;
  version: string;
  /** The launcher's OS (Rust's std::env::consts::OS: "macos", "linux", ...). */
  platform?: string;
  /** The computer's memory in MiB (0 when unknown). */
  memory_mb?: number;
  running: boolean;
  /** A password is saved (online_save_password on and the file present). */
  online_password_saved?: boolean;
};

export type Mod = { name: string; enabled: boolean };

export type Patch = {
  key: string;
  name: string;
  file: string;
  author: string;
  note: string;
  default: boolean;
  enabled: boolean;
};

export type Gamepad = { guid: string; name: string };
/** A monitor as the game sees it (bb-gpu-capabilities --displays). */
export type Display = { name: string; size: string; primary: boolean };
/** Whether FSR 4.1.1 on Metal can run here (bb-gpu-capabilities --fsr411), and why not. */
export type Fsr411Support = { supported: boolean; reason: string };

export type GameState = { running: boolean; code?: number | null };

/** An i18n key (src/i18n/locales/en.json). */
export type TKey = import("i18next").ParseKeys;

/** A choice: label and hint are i18n keys, or literal text when `literal` (e.g. language names). */
export type Option<T extends string | number> =
  | { value: T; label: TKey; hint?: TKey; literal?: false }
  | { value: T; label: string; hint?: undefined; literal: true };

/** Complete runtime save sets, kept separate from shader caches. Times are Unix seconds. */
export type SaveBackup = { id: string; created: number; bytes: number; /** Its save set was edited (.bb-edited). */ edited: boolean };
export type TrophyUnlock = { translations: Record<string, { name: string; description?: string | null }>; name: string | null; description: string | null; grade: "platinum" | "gold" | "silver" | "bronze" | "unknown"; group: "base" | "old_hunters" | null; title_id: string; trophy_id: number; unlocked_at: number };
export type SaveProfile = { trophies_eligible: boolean; trophy_unlocks: TrophyUnlock[]; trophy_error: string | null; id: string; name: string; path: string; bytes: number; modified: number | null; backups: SaveBackup[] };
export type SaveList = { active: string; profiles: SaveProfile[] };
export type SaveChange = { action: "backup" | "restore"; id: string; backup?: string };
export type Character = {
  title_id: string; slot: number; name: string; level: number;
  vitality: number; endurance: number; strength: number; skill: number; bloodtinge: number; arcane: number;
  echoes: number; insight: number; playtime_ms: number; new_game_cycle: number;
  stats_consistent: boolean; fingerprint: string;
};
export type CharacterEdit = { title_id: string; slot: number; fingerprint: string; echoes: number; insight: number;
  vitality: number; endurance: number; strength: number; skill: number; bloodtinge: number; arcane: number };

/** A character's inventory and storage (inventory.rs); names come from the player's game files. */
export type InventoryLocation = "inventory" | "storage";
export type InventoryEntry = {
  location: InventoryLocation; index: number; kind: "item" | "weapon" | "armor" | "upgrade";
  id: number; amount: number; name: string | null; description: string | null;
  /** The game's long description (lore) of the item, weapon or armor piece. */
  caption: string | null;
  level: number; imprint: number; levels: number[]; imprints: number[]; imprint_names: string[];
  /** Items: the most this section holds. */
  max: number;
  /** The game's icon number (api.itemIcon). */
  icon: number | null;
  /** The hover card's numbers, from the game's parameter tables. */
  stats: InventoryStats | null;
  /** Weapons: the hover card's numbers at each level and imprint, by weapon ID. */
  variants: Record<string, InventoryStats>;
  /** Weapons: their open gem slots (shape bits: 1 radial, 2 triangle, 4 waning, 8 circle), with
   * the gem in each (an index in Inventory.upgrades). */
  slots: WeaponSlot[];
  /** Weapons: the slots the game gives them, for each imprint: each slot's shape and the level it
   * opens at (empty when the game's tables do not list the weapon). */
  slot_plan: { imprint: number; slot: number; opens: number; shape: number }[];
};
export type WeaponSlot = { slot: number; shape: number; gem: number | null };
export type InventoryStats = {
  attack?: Record<string, number>; scaling?: Record<string, number>; requirements?: Record<string, number>;
  defense?: Record<string, number>; max?: number;
};
export type CatalogItem = { id: number; name: string; category: "consumable" | "material" | "chalice" | "weapon" | "armor" | "gem"; max: number; icon: number | null; description: string | null;
  /** The game's long description, and the hover card's numbers. */
  caption: string | null; stats: InventoryStats | null };
export type Inventory = { title_id: string; slot: number; fingerprint: string; entries: InventoryEntry[]; upgrades: Gem[] };
/** A gem or rune (inventory.rs): its shape, effects with their alternatives, and where it is. */
export type Gem = {
  index: number; handle: number; rune: boolean;
  /** Gems: 1 radial, 2 triangle, 4 waning, 8 circle, 63 droplet; runes: 1, 2 (Oath) or 4. */
  shape: number;
  effects: { slot: number; id: number; text: string | null; options: { id: number; text: string }[] }[];
  /** The game's icon number (api.itemIcon), from its shape and first effect. */
  icon: number | null;
  location: InventoryLocation | null; equipped: boolean; weapon: string | null;
};
export type InventoryChange =
  | { type: "amount"; location: InventoryLocation; index: number; amount: number }
  | { type: "weapon"; location: InventoryLocation; index: number; level: number; imprint: number }
  | { type: "add"; location: InventoryLocation; id: number; amount: number }
  | { type: "add_equipment"; location: InventoryLocation; id: number }
  | { type: "effect"; upgrade: number; slot: number; effect: number }
  | { type: "equip"; upgrade: number; location: InventoryLocation; index: number; slot: number }
  | { type: "unequip"; upgrade: number }
  | { type: "add_gem"; shape: number; effects: number[] };
export type InventoryEdit = { title_id: string; slot: number; fingerprint: string; changes: InventoryChange[] };
/** One edit of a character: attributes and Blood Echoes, inventory changes, or both (one backup). */
export type SaveEdit = { character: CharacterEdit | null; inventory: InventoryEdit | null };

export type ProfileList = SaveList & { setup_required: boolean; has_existing_saves: boolean };
export type ProfileChange = { action: "initialize" | "create" | "select" | "rename" | "delete"; id: string; name?: string };

export type ReleaseInfo = { version: string; tag: string; newer: boolean; notes: string };

/** The recompiled library (recomp.rs): its state and record, and the disk it takes. */
export type RecompStatus = {
  status: {
    state: "none" | "ready" | "stale" | "building";
    /** stale: built from another copy of the game, or by another recompiler version. */
    reason: "game" | "recompiler" | null;
    library: { built: string; seconds: number; functions: number; size: number; compiler: string } | null;
    /** Whether this build of the app has the recompiler. */
    available: boolean;
  };
  disk_used: number;
  disk_needed: number;
};
/** A build's progress (the "recomp-progress" event). */
export type RecompProgress = {
  step: "download" | "unpack" | "check" | "extract" | "scan" | "generate" | "compile" | "link" | "done" | "error" | "cancelled";
  done?: number;
  total?: number;
  functions?: number;
  message?: string;
};
