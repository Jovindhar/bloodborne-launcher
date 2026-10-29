import { useTranslation } from "react-i18next";
import type { InventoryLocation } from "@/lib";
import { Button } from "@/components/ui";
import type { CatalogCategory } from "@/pages/saves/editor/AddItemPanel";
import { gemIcon, gemName } from "@/pages/saves/editor/gemName";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import type { SaveEdits } from "@/pages/saves/editor/useSaveEdits";

type Props = { edits: SaveEdits; location: InventoryLocation; categories: CatalogCategory[]; disabled: boolean };

/** What this list will gain when saved, at its top, each with a way to take it back. */
export function PendingAdditions({ edits, location, categories, disabled }: Props) {
  const { t } = useTranslation();
  const { additions, setAdditions } = edits;
  return additions.map((addition, at) => ({ addition, at }))
    .filter(({ addition }) => addition.location === location && categories.includes(addition.item.category))
    .map(({ addition, at }) => <div key={`added:${at}`} className="flex items-center gap-3 bg-primary/10 px-3 py-2">
      <ItemIcon icon={addition.item.category === "gem" ? gemIcon(addition.item.icon, addition.shape ?? 1) : addition.item.icon} />
      <p className="min-w-0 flex-1 text-sm">
        {addition.item.category === "gem" && <span className="block">{gemName(t, { rune: false, shape: addition.shape ?? 1 })}</span>}
        {addition.item.name} <span className="text-xs text-muted-foreground">· {t("inventory.added")}</span>
      </p>
      {addition.item.category !== "gem" && <span className="text-sm tabular-nums">×{addition.amount}</span>}
      <Button size="sm" variant="ghost" disabled={disabled} onClick={() => setAdditions((all) => all.filter((_, other) => other !== at))}>{t("inventory.remove")}</Button>
    </div>);
}
