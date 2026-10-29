import { useTranslation } from "react-i18next";
import type { Gem, InventoryEntry } from "@/lib";
import { Button } from "@/components/ui";
import { GemSlots } from "@/pages/saves/editor/GemSlots";
import { ItemIcon } from "@/pages/saves/editor/ItemIcon";
import { StatsTable } from "@/pages/saves/editor/StatsTable";
import { entryKey, slotsAt, type SaveEdits } from "@/pages/saves/editor/useSaveEdits";

// The imprints' names when the game's own word cannot be taken from the weapon names (0 normal, 1 Uncanny, 2 Lost).
const IMPRINTS = ["inventory.normal", "inventory.uncanny", "inventory.lost"] as const;

/** A weapon's name at a level and imprint, as the game writes it ("Uncanny Saw Cleaver +10"). */
export function weaponName(entry: InventoryEntry, level: number, imprint: number) {
  const base = entry.imprint_names[entry.imprints.indexOf(imprint)] ?? entry.name ?? "";
  return level > 0 ? `${base} +${level}` : base;
}

type Props = { entry: InventoryEntry; gems: Gem[]; edits: SaveEdits; disabled: boolean };

/** One weapon: its level and imprint with their numbers beside the current ones, how its imprints
 * compare, and its gem slots. */
export function WeaponDetail({ entry, gems, edits, disabled }: Props) {
  const { t } = useTranslation();
  const key = entryKey(entry);
  const chosen = edits.weapon(entry);
  const set = (next: Partial<typeof chosen>) => edits.setWeapons((all) => ({ ...all, [key]: { ...chosen, ...next } }));
  const changed = chosen.level !== entry.level || chosen.imprint !== entry.imprint;
  const stats = (level: number, imprint: number) =>
    entry.variants[String(entry.id + (level - entry.level) * 100 + (imprint - entry.imprint) * 10000)] ?? null;
  // The game's own word for an imprint: its name less the normal one ("Uncanny Saw Cleaver" → "Uncanny").
  const normal = entry.imprint_names[entry.imprints.indexOf(0)];
  const imprintLabel = (imprint: number) => {
    const name = entry.imprint_names[entry.imprints.indexOf(imprint)] ?? "";
    const own = imprint > 0 && normal && name.includes(normal) ? name.replace(normal, "").trim() : "";
    return own || (imprint === 0 && normal ? normal : t(IMPRINTS[imprint] ?? IMPRINTS[0]));
  };
  const chosenStats = stats(chosen.level, chosen.imprint);
  const slots = slotsAt(entry, chosen, gems);
  const imprints = entry.imprints.flatMap((imprint) => {
    const numbers = stats(chosen.level, imprint);
    return numbers ? [{ label: imprintLabel(imprint), stats: numbers, chosen: imprint === chosen.imprint }] : [];
  });
  const section = "grid gap-2 pt-2";
  return <div className="grid gap-4">
    <div className="flex items-center gap-4">
      <ItemIcon icon={entry.icon} className="size-20" />
      <div className="min-w-0 flex-1">
        <p className="font-medium">{weaponName(entry, chosen.level, chosen.imprint)}</p>
        {changed && <p className="text-xs text-muted-foreground">{t("inventory.was", { name: entry.name })}</p>}
        {entry.location === "storage" && <p className="text-xs text-muted-foreground">{t("inventory.storage")}</p>}
      </div>
      {changed && <Button size="sm" variant="ghost" disabled={disabled} onClick={() => set({ level: entry.level, imprint: entry.imprint })}>{t("inventory.undo")}</Button>}
    </div>
    {entry.caption && <p className="whitespace-pre-line text-sm text-muted-foreground">{entry.caption}</p>}
    {entry.levels.length > 1 && <div className={section}>
      <p className="text-sm font-medium">{t("inventory.level")}</p>
      <div className="flex flex-wrap gap-1">
        {entry.levels.map((level) => <Button key={level} size="sm" variant={chosen.level === level ? "default" : "outline"} disabled={disabled}
          className="min-w-11 tabular-nums" onClick={() => set({ level })}>+{level}</Button>)}
      </div>
    </div>}
    {entry.imprints.length > 1 && <div className={section}>
      <p className="text-sm font-medium">{t("inventory.imprint")}</p>
      <div className="flex flex-wrap gap-1">
        {entry.imprints.map((imprint) => <Button key={imprint} size="sm" variant={chosen.imprint === imprint ? "default" : "outline"} disabled={disabled}
          onClick={() => set({ imprint })}>{imprintLabel(imprint)}</Button>)}
      </div>
    </div>}
    {entry.stats && <div className={section}>
      <StatsTable columns={changed && chosenStats
        ? [{ label: t("inventory.current"), stats: entry.stats }, { label: t("inventory.chosen"), stats: chosenStats, chosen: true }]
        : [{ label: "", stats: entry.stats }]} />
    </div>}
    {imprints.length > 1 && <div className={section}>
      <p className="text-sm font-medium">{t("inventory.variations", { level: chosen.level })}</p>
      <StatsTable columns={imprints} differences />
    </div>}
    <div className={section}>
      <p className="text-sm font-medium">{t("inventory.gemSlots")}</p>
      {slots.length ? <GemSlots slots={slots} chosen={edits.gemSlots[key] ?? {}} gems={gems} available={edits.available} disabled={disabled}
        onChoose={(slot, gem) => edits.setGemSlots((all) => ({ ...all, [key]: { ...all[key], [slot]: gem } }))} />
        : <p className="text-sm text-muted-foreground">{t("inventory.noSlots")}</p>}
    </div>
  </div>;
}
