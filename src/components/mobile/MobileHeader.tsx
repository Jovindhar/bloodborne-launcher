import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Brand } from "@/components/brand";
import { NAV_TABS, RunningDot, SidebarNavItem } from "@/components/sidebar";
import { useGameStore, useUiStore } from "@/stores";

/** Narrow screens (phones, small windows): the name and the tabs in a strip that scrolls sideways. */
export function MobileHeader() {
  const { t } = useTranslation();
  const { tab, setTab } = useUiStore();
  const running = useGameStore((s) => s.running);
  // The chosen tab stays in sight, also when it is chosen from elsewhere (Home's buttons).
  const nav = useRef<HTMLElement>(null);
  useEffect(() => {
    nav.current?.querySelector("[aria-current=page]")?.scrollIntoView({ block: "nearest", inline: "center" });
  }, [tab]);
  return (
    <header className="sticky top-0 z-20 border-b border-sidebar-border bg-sidebar/95 pt-[env(safe-area-inset-top)] backdrop-blur md:hidden">
      <div className="px-4 pt-3"><Brand inline /></div>
      <nav ref={nav} className="flex gap-1 overflow-x-auto px-3 py-2.5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        {NAV_TABS.map((item) => (
          <SidebarNavItem key={item.id} pill label={t(`nav.${item.id}`)} icon={item.icon} active={tab === item.id}
            onClick={() => setTab(item.id)} badge={item.id === "log" && running ? <RunningDot /> : undefined} />
        ))}
      </nav>
    </header>
  );
}
