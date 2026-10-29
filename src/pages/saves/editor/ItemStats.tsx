import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { InventoryStats } from "@/lib";
import { useCanHover } from "@/hooks";
import { Dialog, DialogContent, DialogTitle, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import { StatsTable } from "@/pages/saves/editor/StatsTable";

type Props = {
  name: string; description: string | null; caption?: string | null; icon: number | null; stats: InventoryStats | null; children: ReactNode;
  /** Hover only (the children already act on a tap, like the add panel's tiles); and the wrapper's classes. */
  hoverOnly?: boolean; className?: string;
};

/** An inventory row's details: the icon, the name, the descriptions and the game's numbers. A hover
 * card with a mouse or trackpad; on a touch screen, a tap opens them in a window. */
export function ItemStats({ name, description, caption, icon, stats, children, hoverOnly = false, className }: Props) {
  const { t } = useTranslation();
  const canHover = useCanHover();
  const [open, setOpen] = useState(false);
  const details = (title: ReactNode) => <>
    <div className="flex items-center gap-3">
      <ItemIcon icon={icon} className="size-16" />
      {title}
    </div>
    {description && <p className="whitespace-pre-line">{description}</p>}
    {caption && <p className="whitespace-pre-line text-muted-foreground">{caption}</p>}
    {stats && <StatsTable columns={[{ label: "", stats }]} />}
    {stats?.max ? <p className="text-muted-foreground">{t("inventory.holds", { max: stats.max })}</p> : null}
  </>;
  if (!canHover && hoverOnly) return children;
  if (!canHover) return <>
    <button type="button" className="flex min-w-0 flex-1 items-center gap-3 text-left" onClick={() => setOpen(true)}>{children}</button>
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] gap-3 overflow-y-auto">
        {details(<DialogTitle className="pr-8 text-sm">{name}</DialogTitle>)}
      </DialogContent>
    </Dialog>
  </>;
  return <Tooltip>
    <TooltipTrigger render={<div className={className ?? "flex min-w-0 flex-1 items-center gap-3"} />}>{children}</TooltipTrigger>
    <TooltipContent side="right" align="start" className="flex max-h-[min(36rem,80dvh)] w-80 max-w-80 flex-col items-stretch gap-2 overflow-y-auto border border-border bg-popover p-3 text-popover-foreground shadow-lg">
      {details(<p className="text-sm font-medium">{name}</p>)}
    </TooltipContent>
  </Tooltip>;
}
