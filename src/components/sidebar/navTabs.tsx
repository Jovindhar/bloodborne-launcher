import type { ReactNode } from "react";
import { Users, Cpu, Gamepad2, House, MonitorCog, Package, Save, ScrollText, Settings2, Swords, Trophy, Wrench, Globe2 } from "lucide-react";
import type { Tab } from "@/stores";

/** Online play is shown once the runtime's online module ships (bloodborne-mac ROADMAP, "Online"). */
export const ONLINE_READY = true;

type NavTab = { id: Tab; icon: ReactNode };

/** The tabs in groups: playing (no title), settings, and developer tools. The sidebar shows the
 * groups' titles; the narrow-screen strip shows the same tabs in the same order. */
export const NAV_GROUPS: { title: "nav.groupSettings" | "nav.groupDeveloper" | null; tabs: NavTab[] }[] = [
  { title: null, tabs: [
    { id: "home", icon: <House /> },
    { id: "saves", icon: <Save /> },
    { id: "trophies", icon: <Trophy /> },
    { id: "profiles", icon: <Users /> },
    { id: "online", icon: <Globe2 /> },
  ] },
  { title: "nav.groupSettings", tabs: [
    { id: "graphics", icon: <MonitorCog /> },
    { id: "controls", icon: <Gamepad2 /> },
    { id: "game", icon: <Swords /> },
    { id: "mods", icon: <Package /> },
    { id: "recomp", icon: <Cpu /> },
  ] },
  { title: "nav.groupDeveloper", tabs: [
    { id: "patches", icon: <Wrench /> },
    { id: "advanced", icon: <Settings2 /> },
    { id: "log", icon: <ScrollText /> },
  ] },
].map((group) => ({ ...group, tabs: group.tabs.filter((tab) => ONLINE_READY || tab.id !== "online") })) as { title: "nav.groupSettings" | "nav.groupDeveloper" | null; tabs: NavTab[] }[];

/** All tabs, in order. */
export const NAV_TABS: NavTab[] = NAV_GROUPS.flatMap((group) => group.tabs);
