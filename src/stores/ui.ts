import { create } from "zustand";

export type Tab = "home" | "graphics" | "controls" | "game" | "online" | "profiles" | "mods" | "patches" | "advanced" | "log";

type UiStore = {
  tab: Tab;
  setTab: (tab: Tab) => void;
  viewedProfile: string | null;
  profileSection: "saves" | "trophies";
  openProfile: (id: string, section?: "saves" | "trophies") => void;
};

const TABS: Tab[] = ["home", "graphics", "controls", "game", "online", "profiles", "mods", "patches", "advanced", "log"];

/** The first tab: the URL's hash when it names one (#graphics; previews and development). */
function initialTab(): Tab {
  const hash = typeof location !== "undefined" ? location.hash.slice(1) : "";
  if (hash === "saves") return "profiles";
  return (TABS as string[]).includes(hash) ? (hash as Tab) : "home";
}

export const useUiStore = create<UiStore>((set) => ({
  tab: initialTab(),
  viewedProfile: null,
  profileSection: "saves",
  setTab: (tab) => set({ tab, viewedProfile: null }),
  openProfile: (id, section = "saves") => set({ tab: "profiles", viewedProfile: id, profileSection: section }),
}));
