import type { ReactNode } from "react";
import { Button, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";

type Props = { label: string; onClick?: () => void; disabled?: boolean; children: ReactNode };

/** A small icon-only button that says what it does on hover (and to screen readers). */
export function IconAction({ label, onClick, disabled, children }: Props) {
  return (
    <Tooltip>
      <TooltipTrigger render={<Button variant="ghost" size="icon-sm" aria-label={label} onClick={onClick} disabled={disabled} />}>{children}</TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
