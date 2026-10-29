import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Gem as GemIcon } from "lucide-react";
import type { Gem, InventoryEntry } from "@/lib";
import { Button, Input } from "@/components/ui";
import { AddItemDialog } from "@/pages/saves/editor/AddItemDialog";
import { PendingAdditions } from "@/pages/saves/editor/PendingAdditions";
import { GemDetail } from "@/pages/saves/editor/GemDetail";
import { gemName } from "@/pages/saves/editor/gemName";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import type { SaveEdits } from "@/pages/saves/editor/useSaveEdits";

type Props = { gems: Gem[]; weapons: InventoryEntry[]; edits: SaveEdits; disabled: boolean };

/** The gems and runes as a list, and the chosen one in full beside it (below it on narrow windows),
 * as the weapons are shown. */
export function GemsSection({ gems, weapons, edits, disabled }: Props) {
  const { t, i18n } = useTranslation();
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<number>();
  // Gems can be added more than once: nothing is "already held".
  const held = useMemo(() => ({ inventory: new Set<number>(), storage: new Set<number>() }), []);
  const label = (gem: Gem) => gem.effects[0]?.text ?? t("inventory.effect", { id: gem.effects[0]?.id ?? 0 });
  const shown = useMemo(() => {
    const query = search.trim().toLocaleLowerCase(i18n.language);
    return gems.filter((gem) => !query || [gemName(t, gem), ...gem.effects.map((effect) => effect.text ?? "")]
      .some((text) => text.toLocaleLowerCase(i18n.language).includes(query)));
  }, [gems, search, i18n.language, t]);
  const current = gems.find((gem) => gem.index === selected) ?? shown[0];
  const picked = selected !== undefined && current !== undefined;
  const changed = (gem: Gem) => Object.keys(edits.effects[gem.index] ?? {}).length > 0
    || edits.gemMoved(gem.index);
  return <div className="grid gap-4 lg:grid-cols-[18rem_minmax(0,1fr)]">
    <div className={`content-start gap-2 self-start rounded-2xl surface p-3 ${picked ? "hidden lg:grid" : "grid"}`}>
      <div className="flex gap-2">
        <Input className="min-w-0 flex-1" type="search" placeholder={t("inventory.search")} value={search} onChange={(event) => setSearch(event.target.value)} />
        <AddItemDialog label={t("inventory.addGem")} held={held} location="inventory" categories={["gem"]} disabled={disabled}
          onAdd={(item, _amount, shape) => edits.setAdditions((all) => [...all, { item, location: "inventory", amount: 1, shape }])} />
      </div>
      <div className="own-scroll -mx-3 -mb-3 pb-1">
        <PendingAdditions edits={edits} location="inventory" categories={["gem"]} disabled={disabled} />
        {!shown.length && !edits.additions.some((a) => a.item.category === "gem") ? <p className="px-3 py-3 text-sm text-muted-foreground">{t("inventory.empty")}</p>
          : shown.map((gem) => <button key={gem.index} type="button" onClick={() => setSelected(gem.index)}
            className={`flex w-full items-center gap-3 px-3 py-2 text-left ${gem === current ? "bg-white/[0.08]" : "hover:bg-white/[0.04]"}`}>
            <ItemIcon icon={gem.icon} className="size-9" />
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm">{gemName(t, gem)}</span>
              <span className="block truncate text-xs text-muted-foreground">{label(gem)}</span>
            </span>
            {edits.placeOf(gem.index) && <span className="shrink-0 text-xs text-gold-400">{t("inventory.inWeapon")}</span>}
            {changed(gem) && <span aria-label={t("inventory.edited")} className="size-2 shrink-0 rounded-full bg-primary" />}
          </button>)}
      </div>
    </div>
    {current ? <div className={`self-start ${picked ? "grid" : "hidden lg:grid"} gap-3`}>
      <Button variant="ghost" size="sm" className="justify-self-start lg:hidden" onClick={() => setSelected(undefined)}>← {t("inventory.gems")}</Button>
      <div className="own-scroll rounded-2xl lg:surface lg:p-4">
        <GemDetail key={current.index} gem={current} gems={gems} weapons={weapons} edits={edits} disabled={disabled} />
      </div>
    </div> : <div className="hidden min-h-64 flex-col items-center justify-center gap-3 rounded-2xl surface p-6 text-center lg:flex">
      <GemIcon className="size-8 text-muted-foreground/60" aria-hidden="true" />
      <p className="text-sm text-muted-foreground">{t("inventory.noGems")}</p>
    </div>}
  </div>;
}
