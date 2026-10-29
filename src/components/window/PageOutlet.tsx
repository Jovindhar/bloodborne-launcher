import { AdvancedPage, ControlsPage, GamePage, GraphicsPage, HomePage, LogPage, ModsPage, OnlinePage, PatchesPage, ProfilesPage, RecompPage, SavesPage, TrophiesPage } from "@/pages";
import { useUiStore } from "@/stores";

/** The page of the chosen tab. */
export function PageOutlet() {
  const tab = useUiStore((s) => s.tab);
  switch (tab) {
    case "home":
      return <HomePage />;
    case "graphics":
      return <GraphicsPage />;
    case "controls":
      return <ControlsPage />;
    case "game":
      return <GamePage />;
    case "online":
      return <OnlinePage />;
    case "saves":
      return <SavesPage />;
    case "trophies":
      return <TrophiesPage />;
    case "profiles":
      return <ProfilesPage />;
    case "mods":
      return <ModsPage />;
    case "recomp":
      return <RecompPage />;
    case "patches":
      return <PatchesPage />;
    case "advanced":
      return <AdvancedPage />;
    case "log":
      return <LogPage />;
  }
}
