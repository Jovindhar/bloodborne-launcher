import { useTranslation } from "react-i18next";
import { Check, LockKeyhole } from "lucide-react";
import { Badge } from "@/components/ui";
import { TrophyEmblem } from "@/components/trophy-emblem";
import { trophyText, trophyCatalog } from "@/lib/trophy-text";
import type { TrophyUnlock } from "@/lib";

/** How a trophy list is ordered: as in the game, newest unlock first, locked first, or by grade. */
export type TrophySort = "game" | "recent" | "locked" | "highest" | "lowest";
/** Which trophies a list shows. */
export type TrophyFilter = "all" | "unlocked" | "locked";

// Grades from the highest (hardest) down.
const GRADE_RANK = { platinum: 0, gold: 1, silver: 2, bronze: 3, unknown: 4 } as const;

type Props = { group: "base" | "old_hunters" | "other"; unlocks: TrophyUnlock[]; sort?: TrophySort; filter?: TrophyFilter };

export function TrophyGroup({ group, unlocks, sort = "game", filter = "all" }: Props) {
  const { t, i18n } = useTranslation();
  const entries = group === "other" ? unlocks.map((trophy) => ({ trophy, unlocked: trophy })) :
    trophyCatalog.filter((trophy) => trophy.group === group).map((trophy) => ({ trophy,
      unlocked: unlocks.find((unlock) => unlock.title_id === trophy.title_id && unlock.trophy_id === trophy.trophy_id) }));
  const earned = entries.filter((entry) => entry.unlocked).length;
  // The game's order is the catalog's; the other orders keep it for ties (a stable sort).
  const time = (entry: (typeof entries)[number]) => entry.unlocked?.unlocked_at ?? 0;
  const order: Record<TrophySort, (a: (typeof entries)[number], b: (typeof entries)[number]) => number> = {
    game: () => 0,
    recent: (a, b) => time(b) - time(a),
    locked: (a, b) => Number(Boolean(a.unlocked)) - Number(Boolean(b.unlocked)),
    highest: (a, b) => GRADE_RANK[a.trophy.grade] - GRADE_RANK[b.trophy.grade],
    lowest: (a, b) => GRADE_RANK[b.trophy.grade] - GRADE_RANK[a.trophy.grade],
  };
  const shown = entries
    .filter((entry) => filter === "all" || (filter === "unlocked") === Boolean(entry.unlocked))
    .sort(order[sort]);
  return <section className="space-y-3">
    <h2 className="font-display text-lg">{t(`trophies.${group}`)} <span className="font-sans text-sm text-muted-foreground">{earned}/{entries.length}</span></h2>
    {!shown.length && <p className="px-3 text-sm text-muted-foreground">{t("trophies.noneHere")}</p>}
    <ul>{shown.map(({ trophy, unlocked }) => {
      const text = trophyText(trophy, i18n.resolvedLanguage ?? "en");
      return <li key={`${trophy.title_id}:${trophy.trophy_id}`} className={`flex items-start gap-4 rounded-lg px-3 py-4 text-sm ${unlocked ? "bg-secondary/30" : ""}`}>
        <div className={`mt-1 shrink-0 ${unlocked ? "" : "opacity-35 grayscale"}`}><TrophyEmblem grade={trophy.grade} size={28} /></div>
        <div className="min-w-0 flex-1 space-y-1">
          <div className={`flex flex-wrap items-center justify-between gap-2 ${unlocked ? "" : "opacity-60"}`}>
            <span className="font-medium">{text.name ?? t("trophies.id", { id: trophy.trophy_id })}</span>
            <span className={`text-xs ${unlocked ? `trophy-grade trophy-grade-${trophy.grade}` : "text-muted-foreground"}`}>{t(`trophies.${trophy.grade}`)}</span>
          </div>
          {text.description && <p className={`text-muted-foreground ${unlocked ? "" : "opacity-65"}`}>{text.description}</p>}
          <div className={`flex flex-wrap items-center gap-2 text-xs ${unlocked ? "text-parchment/80" : "text-muted-foreground"}`}>
            <Badge variant="outline" className={`gap-1.5 ${unlocked ? "border-parchment/20 bg-parchment/5 text-parchment" : "border-border bg-secondary text-muted-foreground"}`}>
              {unlocked ? <Check className="size-3" aria-hidden="true" /> : <LockKeyhole className="size-3" aria-hidden="true" />}
              {t(unlocked ? "trophies.unlocked" : "trophies.locked")}
            </Badge>
            {unlocked && <time dateTime={new Date(unlocked.unlocked_at * 1000).toISOString()}>{new Date(unlocked.unlocked_at * 1000).toLocaleString(i18n.resolvedLanguage)}</time>}
          </div>
        </div>
      </li>;
    })}</ul>
  </section>;
}
