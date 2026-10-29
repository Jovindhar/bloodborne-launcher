import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { SaveProfile } from "@/lib";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";

type Props = {
  profile: SaveProfile; className?: string; iconClassName: string;
  /** A fixed width for the count, so a list of profiles lines up; off for a single one. */
  keepSlot?: boolean;
};

/** A profile's recorded trophies: the icon and the count, laid out the same way in every state so
 * lists line up. When its trophies are off (edited save) or failed to import, only the icon differs:
 * the trophy greyed and slashed (in the style of lucide's "-off" icons), the reason on hover. */
export function TrophyCount({ profile, className, iconClassName, keepSlot = true }: Props) {
  const { t } = useTranslation();
  const off = !profile.trophies_eligible || profile.trophy_error;
  const reason = profile.trophy_error ? `${t("trophies.error")} ${profile.trophy_error}` : t("trophies.edited");
  const icon = off
    ? <Tooltip>
      <TooltipTrigger render={<span className="relative inline-flex text-muted-foreground" aria-label={reason} />}>
        {/* The trophy with a transparent band along the slash (a mask, so it works on any background). */}
        <Trophy className={iconClassName} aria-hidden="true"
          style={{ mask: "linear-gradient(to bottom right, #000 calc(50% - 1.6px), transparent calc(50% - 1.6px) calc(50% + 1.6px), #000 calc(50% + 1.6px))" }} />
        <svg className="pointer-events-none absolute inset-0 size-full overflow-visible" viewBox="0 0 24 24" fill="none" strokeLinecap="round" aria-hidden="true">
          <line x1="1" y1="23" x2="23" y2="1" stroke="currentColor" strokeWidth="1.75" />
        </svg>
      </TooltipTrigger>
      <TooltipContent className="max-w-60">{reason}</TooltipContent>
    </Tooltip>
    : <Trophy className={iconClassName} aria-hidden="true" />;
  return <span className={`flex shrink-0 items-center gap-1 tabular-nums ${className ?? ""}`}
    title={off ? undefined : t("trophies.recordedCount", { count: profile.trophy_unlocks.length })}>
    {icon}
    <span className={keepSlot ? "min-w-[2ch] text-left" : undefined}>{profile.trophy_unlocks.length}</span>
  </span>;
}
