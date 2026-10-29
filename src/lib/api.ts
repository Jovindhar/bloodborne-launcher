// Calls into the Tauri backend (src-tauri/src/main.rs). Components use them through the
// TanStack Query hooks (@/hooks) or the game store (@/stores/game).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { profilesService } from "@/services";
import { mockApi } from "@/lib/mockApi";
import type { ReleaseInfo, Config, Display, Fsr411Support, GameCheck, Gamepad, GameState, Mod, Patch, Settings, SaveList, SaveChange, Character, Inventory, SaveEdit, CatalogItem, RecompStatus, RecompProgress } from "@/lib/types";

/** Running inside the Tauri app (else a browser preview with mock data). */
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const tauriApi = {
  checkRelease: () => invoke<ReleaseInfo | null>("check_release"),
  openRelease: (tag: string) => invoke<void>("open_release", { tag }),
  setTrayLabels: (labels: { show: string; stop: string }) => invoke<void>("set_tray_labels", { labels }),
  listProfiles: profilesService.list,
  changeProfile: profilesService.change,
  listSaves: (id?: string) => invoke<SaveList>("list_saves", { id }),
  listCharacters: (id: string) => invoke<Character[]>("list_characters", { id }),
  characterInventory: (id: string, titleId: string, slot: number, language: string) =>
    invoke<Inventory>("character_inventory", { id, titleId, slot, language }),
  editSave: (id: string, edit: SaveEdit) => invoke<void>("edit_save", { id, edit }),
  itemCatalog: (language: string) => invoke<CatalogItem[]>("item_catalog", { language }),
  itemIcon: (icon: number) => invoke<string>("item_icon", { icon }),
  changeSave: (change: SaveChange) => invoke<void>("change_save", change),
  loadConfig: () => invoke<Config>("load_config"),
  saveSettings: (settings: Partial<Settings>) => invoke<void>("save_settings", { settings }),
  saveIni: (values: Record<string, string | null>) => invoke<void>("save_ini", { values }),
  checkGame: (dir: string) => invoke<GameCheck>("check_game", { dir }),
  /** The game's tile from the dump (PNG bytes; empty when there is none). */
  gameIcon: (dir: string) => invoke<ArrayBuffer>("game_icon", { dir }),
  listMods: () => invoke<{ mods: Mod[]; dir: string }>("list_mods"),
  saveMods: (order: string[], disabled: string[]) => invoke<void>("save_mods", { order, disabled }),
  listPatches: () => invoke<{ patches: Patch[]; dir: string }>("list_patches"),
  savePatches: (shown: string[], enabled: string[], disabled: string[]) =>
    invoke<void>("save_patches", { shown, enabled, disabled }),
  listGamepads: () => invoke<Gamepad[]>("list_gamepads"),
  listDisplays: () => invoke<Display[]>("list_displays"),
  fsr411Support: () => invoke<Fsr411Support>("fsr411_support"),
  addFsr411Files: (dll: string) => invoke<void>("add_fsr411_files", { dll }),
  readInput: (kind: "key" | "pad") => invoke<string | null>("read_input", { kind }),
  openLauncherData: () => invoke<void>("open_launcher_data"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  launch: (password?: string) => invoke<void>("launch", { password }),
  saveOnlinePassword: (password: string) => invoke<void>("save_online_password", { password }),
  stop: () => invoke<void>("stop"),
  recompStatus: () => invoke<RecompStatus>("recomp_status"),
  /** Resolves when the build ends; its progress comes as onRecompProgress events. */
  recompBuild: () => invoke<void>("recomp_build"),
  recompCancel: () => invoke<void>("recomp_cancel"),
  recompDelete: () => invoke<void>("recomp_delete"),
  onRecompProgress: (handler: (progress: RecompProgress) => void) => listen<RecompProgress>("recomp-progress", (e) => handler(e.payload)),
  onOutput: (handler: (line: string) => void) => listen<string>("game-output", (e) => handler(e.payload)),
  onState: (handler: (state: GameState) => void) => listen<GameState>("game-state", (e) => handler(e.payload)),
};

export const api: typeof tauriApi = inTauri ? tauriApi : mockApi;
