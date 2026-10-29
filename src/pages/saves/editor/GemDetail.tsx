import { useTranslation } from "react-i18next";
import type { Gem, InventoryEntry } from "@/lib";
import { Choice } from "@/components/common";
import { gemName } from "@/pages/saves/editor/gemName";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import { entryKey, slotsAt, type SaveEdits } from "@/pages/saves/editor/useSaveEdits";
import { weaponName } from "@/pages/saves/editor/WeaponDetail";

type Props = { gem: Gem; gems: Gem[]; weapons: InventoryEntry[]; edits: SaveEdits; disabled: boolean };

/** One gem or rune: where it is, the weapon slot it goes in, and its effects, each with the
 * strengths the game has. */
export function GemDetail({ gem, gems, weapons, edits, disabled }: Props) {
  const { t } = useTranslation();
  const effectText = (effect: Gem["effects"][number]) => effect.text ?? t("inventory.effect", { id: effect.id });
  const place = edits.placeOf(gem.index);
  // Every slot it can go in after the edit: open at the weapon's chosen level, of a shape it fits,
  // and empty (or holding it). A gem in storage goes to the inventory first, in the game.
  const targets = gem.rune || gem.location === "storage" ? [] : weapons.flatMap((entry) => {
    const chosen = edits.gemSlots[entryKey(entry)] ?? {};
    return slotsAt(entry, edits.weapon(entry), gems).filter((slot) => {
      const holding = slot.slot in chosen ? chosen[slot.slot] : slot.gem;
      return (gem.shape & slot.shape) !== 0 && (holding === null || holding === gem.index);
    }).map((slot) => ({ entry, slot: slot.slot }));
  });
  const value = (target: { entry: InventoryEntry; slot: number }) => `${entryKey(target.entry)}#${target.slot}`;
  const where = place ? t("inventory.in", { weapon: weaponName(place.entry, edits.weapon(place.entry).level, edits.weapon(place.entry).imprint) })
    : gem.equipped ? t("inventory.equipped")
    : gem.location === "storage" ? t("inventory.storage") : t("inventory.inventory");
  return <div className="grid gap-4">
    <div className="flex items-center gap-4">
      <ItemIcon icon={gem.icon} className="size-16" />
      <div>
        <p className="font-medium">{gemName(t, gem)}</p>
        <p className="text-xs text-muted-foreground">{where}</p>
      </div>
    </div>
    <div className="grid gap-2">
      <p className="text-sm font-medium">{t("inventory.weaponSlot")}</p>
      {targets.length > 0 || place ? <Choice label={t("inventory.weaponSlot")} disabled={disabled} className="w-full"
        value={place ? value(place) : "none"}
        options={[{ value: "none", label: t("inventory.notInWeapon") },
          ...targets.map((target) => ({ value: value(target),
            label: t("inventory.inSlot", { weapon: weaponName(target.entry, edits.weapon(target.entry).level, edits.weapon(target.entry).imprint), slot: target.slot + 1 }) }))]}
        onChange={(chosen) => edits.moveGem(gem.index, targets.find((target) => value(target) === chosen) ?? null)} />
        : <p className="text-xs text-muted-foreground">{t(gem.rune ? "inventory.runeNoWeapon" : gem.location === "storage" ? "inventory.gemInStorage" : "inventory.noFittingSlot")}</p>}
    </div>
    <div className="grid gap-2">
      <p className="text-sm font-medium">{t("inventory.effects")}</p>
      {gem.effects.map((effect) => effect.options.length > 1
        ? <Choice key={effect.slot} label={effectText(effect)} disabled={disabled} className="w-full"
          value={edits.effects[gem.index]?.[effect.slot] ?? effect.id} options={effect.options.map((option) => ({ value: option.id, label: option.text }))}
          onChange={(chosen) => edits.setEffects((all) => ({ ...all, [gem.index]: { ...all[gem.index], [effect.slot]: chosen } }))} />
        : <p key={effect.slot} className="text-sm text-muted-foreground">{effectText(effect)}</p>)}
    </div>
  </div>;
}
