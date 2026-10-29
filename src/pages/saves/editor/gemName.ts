import type { TFunction } from "i18next";
import type { Gem } from "@/lib";

// The shapes' texts (gems: radial 1, triangle 2, waning 4, circle 8, droplet 63).
const SHAPES = { 1: "inventory.radial", 2: "inventory.triangle", 4: "inventory.waning", 8: "inventory.circle", 63: "inventory.droplet" } as const;

// Each gem shape's first icon (inventory.rs, gem_icon); a gem's icon is its effect's offset from it.
const SHAPE_ICONS: Record<number, number> = { 1: 3000, 2: 3200, 4: 3400, 8: 3600, 63: 4000 };

/** The icon of a gem to add: the catalog gives it as a radial gem; the same offset from the chosen
 * shape's first icon. */
export function gemIcon(radialIcon: number | null, shape: number) {
  return radialIcon === null || !(shape in SHAPE_ICONS) ? null : radialIcon - SHAPE_ICONS[1] + SHAPE_ICONS[shape];
}

/** A gem's or rune's kind as the game calls it: its shape, or Caryll/Oath rune. */
export function gemName(t: TFunction, gem: Pick<Gem, "rune" | "shape">) {
  if (gem.rune) return gem.shape === 2 ? t("inventory.oathRune") : t("inventory.rune");
  return gem.shape in SHAPES ? t(SHAPES[gem.shape as keyof typeof SHAPES]) : t("inventory.gem");
}
