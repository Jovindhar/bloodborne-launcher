// Calls into the Tauri backend (src-tauri/src/main.rs). Components use them through the
// TanStack Query hooks (@/hooks) or the game store (@/stores/game).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { profilesService } from "@/services";
import { mockApi } from "@/lib/mockApi";
import type { ReleaseInfo, Config, Display, Fsr411Support, GameCheck, Gamepad, GameState, Mod, Patch, Settings, SaveList, SaveChange, Character, CharacterEdit } from "@/lib/types";

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
  editCharacter: (id: string, edit: CharacterEdit) => invoke<void>("edit_character", { id, edit }),
  changeSave: (change: SaveChange) => invoke<void>("change_save", change),
  loadConfig: () => invoke<Config>("load_config"),
  saveSettings: (settings: Partial<Settings>) => invoke<void>("save_settings", { settings }),
  saveIni: (values: Record<string, string | null>) => invoke<void>("save_ini", { values }),
  checkGame: (dir: string) => invoke<GameCheck>("check_game", { dir }),
  listMods: () => invoke<{ mods: Mod[]; dir: string }>("list_mods"),
  saveMods: (order: string[], disabled: string[]) => invoke<void>("save_mods", { order, disabled }),
  listPatches: () => invoke<{ patches: Patch[]; dir: string }>("list_patches"),
  savePatches: (shown: string[], enabled: string[], disabled: string[]) =>
    invoke<void>("save_patches", { shown, enabled, disabled }),
  listGamepads: () => invoke<Gamepad[]>("list_gamepads"),
  listDisplays: () => invoke<Display[]>("list_displays"),
  fsr411Support: () => invoke<Fsr411Support>("fsr411_support"),
  addFsr411Files: (dll: string) => invoke<void>("add_fsr411_files", { dll }),
  onlinePasswordSaved: (npid: string) => invoke<boolean>("online_password_saved", { npid }),
  setOnlinePassword: (npid: string, password: string) => invoke<void>("set_online_password", { npid, password }),
  readInput: (kind: "key" | "pad") => invoke<string | null>("read_input", { kind }),
  openLauncherData: () => invoke<void>("open_launcher_data"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  launch: () => invoke<void>("launch"),
  stop: () => invoke<void>("stop"),
  onOutput: (handler: (line: string) => void) => listen<string>("game-output", (e) => handler(e.payload)),
  onState: (handler: (state: GameState) => void) => listen<GameState>("game-state", (e) => handler(e.payload)),
};

export const api: typeof tauriApi = inTauri ? tauriApi : mockApi;
