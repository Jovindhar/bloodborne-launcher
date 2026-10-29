import { MobileBar, MobileHeader } from "@/components/mobile";
import { UpdateNotice } from "@/components/updates";
import { Sidebar } from "@/components/sidebar";
import { PageOutlet } from "@/components/window/PageOutlet";
import { TitleBar } from "@/components/window/TitleBar";
import { PageBackground } from "@/components/window/PageBackground";
import { useConfig } from "@/hooks";

/** The launcher window: sidebar with the tabs and Play, the chosen page on the right. Narrow
 *  screens (phones, small windows): the tabs in a strip at the top and Play in a bar at the bottom. */
export function AppWindow() {
  const { data: config } = useConfig();
  return (
    // The window: its background sits behind the menu and the page alike (Home's picture over it).
    <div className="relative isolate flex h-full flex-col md:flex-row">
      <PageBackground />
      <Sidebar />
      <MobileHeader />
      <main className="relative isolate flex flex-1 flex-col overflow-y-auto bg-[radial-gradient(ellipse_at_top_right,rgba(201,162,101,0.05),transparent_55%)]">
        {config?.platform === "macos" ? <TitleBar /> : <div className="hidden h-6 shrink-0 md:block" />}
        <div className="mx-auto w-full max-w-4xl px-4 pb-28 pt-4 sm:px-6 md:px-10 md:pb-12 md:pt-0">
          <UpdateNotice />
          <PageOutlet />
        </div>
      </main>
      <MobileBar />
    </div>
  );
}
