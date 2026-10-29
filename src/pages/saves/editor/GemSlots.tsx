import { useTranslation } from "react-i18next";
import type { Gem } from "@/lib";
import { gemName } from "@/pages/saves/editor/gemName";
import { Choice } from "@/components/common";

type Props = {
  slots: { slot: number; shape: number; gem: number | null }[];
  /** The gem chosen for each slot (null: empty), when changed. */
  chosen: Record<number, number | null>;
  gems: Gem[];
  /** Whether a gem can go in a slot now: in the inventory, or taken out of its slot in this edit, and not chosen elsewhere. */
  available: (gem: Gem) => boolean;
  disabled: boolean;
  onChoose: (slot: number, gem: number | null) => void;
};

/** A weapon's open gem slots, each a select of the gems whose shape it takes. */
export function GemSlots({ slots, chosen, gems, available, disabled, onChoose }: Props) {
  const { t } = useTranslation();
  const describe = (gem: Gem) => [gemName(t, gem), ...gem.effects.map((effect) => effect.text ?? t("inventory.effect", { id: effect.id }))].join(" · ");
  return <div className="grid gap-2">
    {slots.map((slot) => {
      const current = slot.slot in chosen ? chosen[slot.slot] : slot.gem;
      // A slot takes the gem shapes of its bits; droplet gems have them all.
      const fitting = gems.filter((gem) => !gem.rune && (gem.shape & slot.shape) !== 0 && (gem.index === current || available(gem)));
      // "empty", or the gem's index.
      return <Choice key={slot.slot} label={t("inventory.gemSlot", { slot: slot.slot + 1 })} disabled={disabled} className="w-full"
        value={current === null ? "empty" : String(current)}
        options={[{ value: "empty", label: t("inventory.emptySlot", { slot: slot.slot + 1 }) },
          ...fitting.map((gem) => ({ value: String(gem.index), label: describe(gem) }))]}
        onChange={(value) => onChoose(slot.slot, value === "empty" ? null : Number(value))} />;
    })}
  </div>;
}
