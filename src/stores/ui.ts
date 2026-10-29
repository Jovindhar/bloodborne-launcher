import { create } from "zustand";

export type Tab = "home" | "saves" | "trophies" | "graphics" | "controls" | "game" | "online" | "profiles" | "mods" | "recomp" | "patches" | "advanced" | "log";

type UiStore = {
  tab: Tab;
  setTab: (tab: Tab) => void;
  /** The character open in the save editor on the Saves tab ("title:slot"), or none. */
  editing: string | null;
  setEditing: (character: string | null) => void;
};

const TABS: Tab[] = ["home", "saves", "trophies", "graphics", "controls", "game", "online", "profiles", "mods", "recomp", "patches", "advanced", "log"];

/** The first tab: the URL's hash when it names one (#graphics; previews and development). */
function initialTab(): Tab {
  const hash = typeof location !== "undefined" ? location.hash.slice(1) : "";
  return (TABS as string[]).includes(hash) ? (hash as Tab) : "home";
}

export const useUiStore = create<UiStore>((set) => ({
  tab: initialTab(),
  setTab: (tab) => set({ tab, editing: null }),
  editing: null,
  setEditing: (editing) => set({ editing }),
}));
