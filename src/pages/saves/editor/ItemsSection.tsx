import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InventoryEntry, InventoryLocation } from "@/lib";
import { Input } from "@/components/ui";
import { AddItemDialog } from "@/pages/saves/editor/AddItemDialog";
import type { CatalogCategory } from "@/pages/saves/editor/AddItemPanel";
import { PendingAdditions } from "@/pages/saves/editor/PendingAdditions";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import { ItemStats } from "@/pages/saves/editor/ItemStats";
import { entryKey, type SaveEdits } from "@/pages/saves/editor/useSaveEdits";

type Props = {
  entries: InventoryEntry[];
  edits: SaveEdits;
  /** Where added items go; none: this list takes no additions. */
  addTo?: InventoryLocation;
  /** What can be added here, and the button's text. */
  categories?: CatalogCategory[];
  addLabel?: string;
  disabled: boolean;
};

/** A list of entries: items with their amounts, anything else as it is; and items to add. */
export function ItemsSection({ entries, edits, addTo, categories = ["consumable", "material", "chalice"], addLabel, disabled }: Props) {
  const { t, i18n } = useTranslation();
  const [search, setSearch] = useState("");
  const { amounts, setAmounts, additions, setAdditions, changed } = edits;
  const shown = useMemo(() => {
    const query = search.trim().toLocaleLowerCase(i18n.language);
    return entries.filter((entry) => !query || (entry.name ?? "").toLocaleLowerCase(i18n.language).includes(query));
  }, [entries, search, i18n.language]);
  const held = useMemo(() => {
    const sets: Record<InventoryLocation, Set<number>> = { inventory: new Set(), storage: new Set() };
    for (const entry of entries) if (entry.kind === "item") sets[entry.location].add(entry.id);
    for (const addition of additions) sets[addition.location].add(addition.item.id);
    return sets;
  }, [entries, additions]);
  const label = (entry: InventoryEntry) => entry.name ?? t("inventory.unnamed", { id: entry.id });
  return <div className="grid gap-3 rounded-2xl surface p-3">
    <div className="flex flex-wrap gap-2">
      <Input className="min-w-40 flex-1" type="search"
        placeholder={t("inventory.search")} value={search} onChange={(event) => setSearch(event.target.value)} />
      {addTo && <AddItemDialog label={addLabel ?? t("inventory.add")} held={held} location={addTo} categories={categories} disabled={disabled}
        onAdd={(item, amount) => setAdditions((all) => [...all, { item, location: addTo, amount }])} />}
    </div>
    <div className="own-scroll -mx-3 -mb-3 pb-1">
      {addTo && <PendingAdditions edits={edits} location={addTo} categories={categories} disabled={disabled} />}
      {!shown.length && !additions.some((a) => a.location === addTo && categories.includes(a.item.category)) ? <p className="px-3 py-3 text-sm text-muted-foreground">{t("inventory.empty")}</p>
        : shown.map((entry) => <div key={entryKey(entry)}
          className={`flex items-center gap-3 px-3 py-2 ${changed(entry) ? "bg-primary/10" : ""}`}>
          <ItemStats name={label(entry)} description={entry.description} caption={entry.caption} icon={entry.icon} stats={entry.stats}>
            <ItemIcon icon={entry.icon} />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm">{label(entry)}</p>
              {entry.description && <p className="line-clamp-1 text-xs text-muted-foreground">{entry.description}</p>}
            </div>
          </ItemStats>
          {entry.kind === "item" ? <Input aria-label={t("inventory.amount")} className="w-20 tabular-nums"
            type="number" min="1" max={entry.max || 999} step="1" disabled={disabled}
            value={amounts[entryKey(entry)] ?? String(entry.amount)}
            onChange={(event) => setAmounts((current) => ({ ...current, [entryKey(entry)]: event.target.value }))} />
            : entry.amount > 1 && <span className="text-sm tabular-nums text-muted-foreground">×{entry.amount}</span>}
        </div>)}
    </div>
  </div>;
}
