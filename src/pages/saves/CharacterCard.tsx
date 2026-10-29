import { useTranslation } from "react-i18next";
import { Droplet, Eye } from "lucide-react";
import type { Character } from "@/lib";
import { Message } from "@/components/common";
import { EditCharacterButton } from "@/pages/saves/editor";

const ATTRIBUTES = ["vitality", "endurance", "strength", "skill", "bloodtinge", "arcane"] as const;

/** One character: its name and where it is, its
 * level, playtime and journey, the six attributes, Insight and Blood Echoes, and the editor. */
export function CharacterCard({ character, disabled }: { character: Character; disabled: boolean }) {
  const { t, i18n } = useTranslation();
  const number = (value: number) => value.toLocaleString(i18n.language);
  const hours = (character.playtime_ms / 3_600_000).toLocaleString(i18n.language, { maximumFractionDigits: 1 });
  return (
    <div className="rounded-2xl surface p-4 sm:p-5">
      <div className="min-w-0 flex-1 space-y-4">
        <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
          <h2 className="font-display text-2xl font-semibold tracking-wide text-gold-300">{character.name}</h2>
          <span className="text-xs text-muted-foreground">{character.title_id} · {t("saves.slot", { slot: character.slot + 1 })}</span>
        </div>
        <p className="-mt-2 text-sm text-muted-foreground">
          {t("saves.level", { level: character.level })} · {t("saves.playtime", { hours })} · {t("saves.newGameCycle", { cycle: character.new_game_cycle + 1 })}
        </p>
        <dl className="grid grid-cols-3 gap-y-3 rounded-xl surface-inset py-3 sm:grid-cols-6">
          {/* Labels at the top, numbers on one line at the bottom (a label may wrap). */}
          {ATTRIBUTES.map((field) => <div key={field} className="flex flex-col items-center justify-between px-1 text-center">
            <dt className="text-[10px] uppercase tracking-[0.15em] text-muted-foreground">{t(`saves.${field}`)}</dt>
            <dd className="mt-1 font-display text-xl tabular-nums">{number(character[field])}</dd>
          </div>)}
        </dl>
        <dl className="flex flex-wrap gap-x-8 gap-y-2 px-1 text-sm">
          <div className="flex items-center gap-2"><Eye className="size-4 text-gold-400" aria-hidden="true" />
            <dt className="text-xs uppercase tracking-[0.15em] text-muted-foreground">{t("saves.insight")}</dt><dd className="tabular-nums">{number(character.insight)}</dd></div>
          <div className="flex items-center gap-2"><Droplet className="size-4 text-gold-400" aria-hidden="true" />
            <dt className="text-xs uppercase tracking-[0.15em] text-muted-foreground">{t("saves.echoes")}</dt><dd className="tabular-nums">{number(character.echoes)}</dd></div>
        </dl>
        {!character.stats_consistent && <Message tone="error" className="text-xs">{t("saves.inconsistentStats")}</Message>}
        <div className="flex flex-wrap gap-2"><EditCharacterButton character={character} disabled={disabled} /></div>
      </div>
    </div>
  );
}
