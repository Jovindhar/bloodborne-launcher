import type { ReactNode } from "react";
import { Button, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";

type Props = { icon: ReactNode; label: string; value: ReactNode; detail?: ReactNode; onClick?: () => void };

/** One setting at a glance, as a small button under Play: its icon and value, its name (and detail)
 * on hover; opens its settings page. */
export function SummaryTile({ icon, label, value, detail, onClick }: Props) {
  return (
    <Tooltip>
      <TooltipTrigger render={<Button variant="secondary" size="sm" onClick={onClick} aria-label={`${label}: ${String(value)}`}
        className="min-w-0 max-w-56 shrink backdrop-blur-sm" />}>
        {icon} <span className="truncate">{value}</span>
      </TooltipTrigger>
      <TooltipContent>{label}{detail && <> · {detail}</>}</TooltipContent>
    </Tooltip>
  );
}
