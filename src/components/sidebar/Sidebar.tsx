import { useTranslation } from "react-i18next";
import { Brand } from "@/components/brand";
import { ActiveProfile } from "@/components/profiles";
import { NAV_GROUPS } from "@/components/sidebar/navTabs";
import { RunningDot } from "@/components/sidebar/RunningDot";
import { SidebarNavItem } from "@/components/sidebar/SidebarNavItem";
import { useConfig } from "@/hooks";
import { cn } from "@/lib";
import { useGameStore, useUiStore } from "@/stores";

export function Sidebar() {
  const { t } = useTranslation();
  const { tab, setTab } = useUiStore();
  const running = useGameStore((s) => s.running);
  const { data: config } = useConfig();
  return (
    <aside
      // Wide windows only (narrow ones: MobileHeader). macOS draws its window buttons over the top.
      className={cn("drag hidden w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar/45 px-3 backdrop-blur-sm pb-4 md:flex",
        config?.platform === "macos" ? "pt-11" : "pt-5")}
      data-tauri-drag-region
    >
      <div className="mb-7 px-2"><Brand /></div>
      <nav className="no-drag flex flex-col gap-0.5">
        {NAV_GROUPS.map((group) => <div key={group.title ?? "main"} className="flex flex-col gap-0.5 [&+&]:mt-4">
          {group.title && <p className="px-3 pb-1 text-[11px] text-muted-foreground">{t(group.title)}</p>}
          {group.tabs.map((item) => (
            <SidebarNavItem
              key={item.id}
              label={t(`nav.${item.id}`)}
              icon={item.icon}
              active={tab === item.id}
              onClick={() => setTab(item.id)}
              badge={item.id === "log" && running ? <RunningDot /> : undefined}
            />
          ))}
        </div>)}
      </nav>
      <div className="no-drag mt-auto">
        <ActiveProfile />
        <div className="mt-3 text-center text-[11px] text-muted-foreground">{t("app.version", { version: config?.version ?? "" })}</div>
      </div>
    </aside>
  );
}
