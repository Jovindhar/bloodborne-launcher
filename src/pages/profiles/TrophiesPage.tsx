import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Choice } from "@/components/common";
import { PageHeader, SettingsSection } from "@/components/settings";
import { TrophyGroup, TrophyStatus, type TrophyFilter, type TrophySort } from "@/components/trophies";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui";
import { useProfiles } from "@/hooks";

const SORTS: { value: TrophySort; label: "trophies.sortGame" | "trophies.sortRecent" | "trophies.sortLocked" | "trophies.sortHighest" | "trophies.sortLowest" }[] = [
  { value: "game", label: "trophies.sortGame" },
  { value: "recent", label: "trophies.sortRecent" },
  { value: "locked", label: "trophies.sortLocked" },
  { value: "highest", label: "trophies.sortHighest" },
  { value: "lowest", label: "trophies.sortLowest" },
];

/** The Trophies tab: the active profile's recorded trophies, by game, sorted and filtered the same
 * way in each game. */
export function TrophiesPage() {
  const { t } = useTranslation();
  const { data } = useProfiles();
  const [sort, setSort] = useState<TrophySort>("game");
  const [filter, setFilter] = useState<TrophyFilter>("all");
  const profile = data?.profiles.find((entry) => entry.id === data.active);
  if (!profile) return null;
  const other = profile.trophy_unlocks.filter((unlock) => !unlock.group);
  return (
    <>
      <PageHeader title={t("profiles.trophies")} subtitle={profile.name} />
      <div className="space-y-5">
        <p className="text-sm text-muted-foreground">{t("trophies.local")}</p>
        <TrophyStatus profile={profile} />
        <div className="flex flex-wrap items-center justify-between gap-3">
          <Tabs value={filter} onValueChange={(value) => setFilter(value as TrophyFilter)}>
            <TabsList>
              <TabsTrigger value="all">{t("trophies.all")}</TabsTrigger>
              <TabsTrigger value="unlocked">{t("trophies.unlocked")}</TabsTrigger>
              <TabsTrigger value="locked">{t("trophies.locked")}</TabsTrigger>
            </TabsList>
          </Tabs>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">{t("trophies.sort")}
            <Choice label={t("trophies.sort")} value={sort} onChange={setSort}
              options={SORTS.map((option) => ({ value: option.value, label: t(option.label) }))} />
          </div>
        </div>
        <SettingsSection><div className="px-5 py-4"><TrophyGroup group="base" unlocks={profile.trophy_unlocks} sort={sort} filter={filter} /></div></SettingsSection>
        <SettingsSection><div className="px-5 py-4"><TrophyGroup group="old_hunters" unlocks={profile.trophy_unlocks} sort={sort} filter={filter} /></div></SettingsSection>
        {other.length > 0 && <SettingsSection><div className="px-5 py-4"><TrophyGroup group="other" unlocks={other} sort={sort} filter={filter} /></div></SettingsSection>}
      </div>
    </>
  );
}
