import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CatalogItem, InventoryLocation } from "@/lib";
import { Button, Input } from "@/components/ui";
import { gemIcon } from "@/pages/saves/editor/gemName";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import { ItemStats } from "@/pages/saves/editor/ItemStats";
import { useItemCatalog } from "@/pages/saves/editor/useItemCatalog";
import { Choice, Message } from "@/components/common";

/** What the panel offers: items, weapons, armor or gems (storage takes all but gems). */
export type CatalogCategory = CatalogItem["category"];

type Props = {
  /** Items already in each section (by item ID): adding them again is refused. */
  held: Record<InventoryLocation, Set<number>>;
  /** Where the item goes (the section it is added from). */
  location: InventoryLocation;
  /** The catalog's categories this section takes. */
  categories: CatalogCategory[];
  disabled: boolean;
  /** The confirm button's text (the dialog's "Add weapon", "Add gem"...). */
  label: string;
  onAdd: (item: CatalogItem, amount: number, shape?: number) => void;
};

// The shapes a new gem can have, as the game numbers them (gemName.ts names them).
const GEM_SHAPES = [[1, "inventory.radial"], [2, "inventory.triangle"], [4, "inventory.waning"], [8, "inventory.circle"], [63, "inventory.droplet"]] as const;

/** What the game has that can be added here: a category, a search and an amount (weapons, armor
 * and gems one at a time; a gem by its effect, with a shape). */
export function AddItemPanel({ held, location, categories, disabled, label, onAdd }: Props) {
  const { t, i18n } = useTranslation();
  const [category, setCategory] = useState<CatalogCategory>(categories[0]);
  const [search, setSearch] = useState("");
  const [chosen, setChosen] = useState<CatalogItem>();
  const [amount, setAmount] = useState("1");
  const [shape, setShape] = useState(1);
  const catalog = useItemCatalog();
  const shown = useMemo(() => {
    const query = search.trim().toLocaleLowerCase(i18n.language);
    return (catalog.data ?? []).filter((item) => item.category === category)
      .filter((item) => !query || item.name.toLocaleLowerCase(i18n.language).includes(query));
  }, [catalog.data, category, search, i18n.language]);
  // Weapons and armor: one at a time, and as many of the same as wanted; items: up to the game's
  // limit (99 in storage), once per section.
  const equipment = chosen?.category === "weapon" || chosen?.category === "armor" || chosen?.category === "gem";
  const max = !chosen || equipment ? 1 : location === "storage" ? Math.max(99, chosen.max) : chosen.max;
  const already = chosen && !equipment ? held[location].has(chosen.id) : false;
  const valid = chosen && !already && /^\d+$/.test(amount) && Number(amount) >= 1 && Number(amount) <= max;
  return <div className="grid gap-3 rounded-xl surface-inset p-3">
    <div className="flex flex-wrap gap-2">
      {categories.length > 1 && categories.map((name) => <Button key={name} size="sm" variant={category === name ? "default" : "outline"} onClick={() => { setCategory(name); setChosen(undefined); }}>
        {t(`inventory.${name}`)}</Button>)}
      <Input className="min-w-40 flex-1" type="search"
        placeholder={t("inventory.search")} value={search} onChange={(event) => setSearch(event.target.value)} />
    </div>
    {/* Gems have no icons here: their effects as a list. */}
    {category === "gem" ? <div className="grid max-h-72 content-start overflow-y-auto">
      {catalog.error ? <Message tone="error">{String(catalog.error)}</Message>
        : shown.map((item) => <button key={item.id} type="button" onClick={() => setChosen(item)}
          className={`flex items-center gap-3 rounded-md px-2 py-1 text-left text-sm ${chosen?.id === item.id ? "bg-gold-400/25" : "hover:bg-white/[0.06]"}`}>
          <ItemIcon icon={gemIcon(item.icon, shape)} className="size-8" />{item.name}</button>)}
    </div> : <div className="grid max-h-72 grid-cols-[repeat(auto-fill,minmax(3rem,1fr))] gap-1 overflow-y-auto">
      {catalog.error ? <Message tone="error" className="col-span-full">{String(catalog.error)}</Message>
        : shown.map((item) => <ItemStats key={item.id} hoverOnly className="block" name={item.name} description={item.description}
          caption={item.caption} icon={item.icon} stats={item.stats}><button type="button" aria-label={item.name}
          className={`rounded-md p-0.5 ${chosen?.id === item.id ? "bg-gold-400/25" : "hover:bg-white/[0.06]"}`}
          onClick={() => { setChosen(item); setAmount(String(Math.min(item.max, 1))); }}>
          <ItemIcon icon={item.icon} className="size-full" />
        </button></ItemStats>)}
    </div>}
    {chosen ? <div className="flex flex-wrap items-end gap-3">
      <div className="flex min-w-40 flex-1 items-center gap-2">
        <ItemIcon icon={chosen.category === "gem" ? gemIcon(chosen.icon, shape) : chosen.icon} />
        <div><p className="text-sm">{chosen.name}</p>
          {(chosen.description ?? chosen.caption) && <p className="line-clamp-2 text-xs text-muted-foreground">{chosen.description ?? chosen.caption}</p>}</div>
      </div>
      {chosen.category === "gem" && <Choice label={t("inventory.shape")} className="w-44" value={shape}
        options={GEM_SHAPES.map(([value, key]) => ({ value, label: t(key) }))} onChange={setShape} />}
      {!equipment && <label className="grid gap-1 text-xs">{t("inventory.amount")}
        <Input className="w-24" type="number" min="1" max={max}
          value={amount} onChange={(event) => setAmount(event.target.value)} />
      </label>}
      <Button size="sm" disabled={disabled || !valid} onClick={() => { onAdd(chosen, Number(amount), chosen.category === "gem" ? shape : undefined); setChosen(undefined); }}>
        {label}</Button>
      {already && <Message tone="warning" className="basis-full text-xs">{t("inventory.alreadyHeld")}</Message>}
    </div> : <p className="text-xs text-muted-foreground">{t("inventory.choose")}</p>}
  </div>;
}
