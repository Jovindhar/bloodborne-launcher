import { useTranslation } from "react-i18next";
import { ChevronsUpDown, UserRound } from "lucide-react";
import { TrophyCount } from "@/components/trophies";
import { useProfiles } from "@/hooks";
import { useUiStore } from "@/stores";
import { Menu, MenuContent, MenuItem, MenuLabel, MenuRadioGroup, MenuRadioItem, MenuSeparator, MenuTrigger } from "@/components/ui";

/** The active profile as a small widget (its name and recorded trophies) that opens the profile
 * menu: switch profile, and go to its saves, its trophies or all profiles. `compact`: the
 * narrow-screen bar. */
export function ActiveProfile({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation();
  const { data, change, busy, running } = useProfiles();
  const setTab = useUiStore((state) => state.setTab);
  const active = data?.profiles.find((profile) => profile.id === data.active);
  const locked = busy || running || !data || data.setup_required;
  // Trophies off for this save set, or their import failed: a slashed trophy, the reason on hover.
  return (
    <div className={compact ? undefined : "mb-3"}>
      <Menu>
        <MenuTrigger disabled={!active} aria-label={t("profiles.active")}
          className="flex w-full items-center gap-2.5 rounded-xl surface-raised px-2.5 py-2 text-left outline-none transition-colors hover:bg-accent/60 focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:opacity-50">
          {/* The phone bar shares its row with Play: the name keeps the room, the avatar goes. */}
          <span className={`relative shrink-0 items-center justify-center rounded-lg bg-blood-700/20 text-blood-400 ${compact ? "hidden" : "flex size-8"}`}>
            <UserRound className="size-4" aria-hidden="true" />
          </span>
          {/* One row, all centred: the avatar, the name, the trophies and the menu's chevron. */}
          <span className="min-w-0 flex-1 truncate text-sm font-medium">{active?.name ?? "—"}</span>
          {active && <TrophyCount profile={active} keepSlot={false} className="text-sm text-parchment/70" iconClassName="size-4" />}
          <ChevronsUpDown className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        </MenuTrigger>
        <MenuContent className="w-64">
          <MenuLabel>{t("profiles.active")}</MenuLabel>
          <MenuRadioGroup value={data?.active ?? ""} onValueChange={(id: string) => id !== data?.active && change({ action: "select", id }).catch(() => {})}>
            {data?.profiles.map((profile) => <MenuRadioItem key={profile.id} value={profile.id} disabled={locked}>
              <span className="min-w-0 flex-1 truncate">{profile.name}</span>
              <TrophyCount profile={profile} className="text-xs text-muted-foreground" iconClassName="size-4" />
            </MenuRadioItem>)}
          </MenuRadioGroup>
          <MenuSeparator />
          <MenuItem onClick={() => setTab("saves")}>{t("nav.saves")}</MenuItem>
          <MenuItem onClick={() => setTab("trophies")}>{t("profiles.trophies")}</MenuItem>
          <MenuItem onClick={() => setTab("profiles")}>{t("nav.profiles")}</MenuItem>
        </MenuContent>
      </Menu>
    </div>
  );
}
