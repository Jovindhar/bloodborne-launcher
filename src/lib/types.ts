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
  check_updates: boolean;
  crash_diag: boolean;
  gpu_profile: boolean;
  extra_env: string;
  /** Launcher language ("" = the system's). */
  ui_language: string;
  /** Online play (BB_ONLINE): Seamless Hunt's servers, with the Online ID below. */
  online: boolean;
  /** The shadNet server ("" = Seamless Hunt's, play.seamlesshunt.com:31313). */
  online_server: string;
  /** The Online ID (BB_SHADNET_NPID); its password is in the macOS Keychain. */
  online_npid: string;
  /** BBSC seamless co-op (BB_BBSC). */
  online_coop: boolean;
  online_upnp: boolean;
  /** BBSC's game server on this computer (BB_BBSC_LOCAL_SERVER). */
  online_local_server: "auto" | "on" | "off";
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
export type SaveBackup = { id: string; created: number; bytes: number };
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

export type ProfileList = SaveList & { setup_required: boolean; has_existing_saves: boolean };
export type ProfileChange = { action: "initialize" | "create" | "select" | "rename" | "delete"; id: string; name?: string };

export type ReleaseInfo = { version: string; tag: string; newer: boolean; notes: string };
