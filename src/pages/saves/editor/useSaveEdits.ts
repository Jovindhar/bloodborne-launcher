import { useMemo, useState } from "react";
import type { CatalogItem, Character, CharacterEdit, Gem, Inventory, InventoryChange, InventoryEntry, InventoryLocation, WeaponSlot } from "@/lib";

export const ATTRIBUTES = ["vitality", "endurance", "strength", "skill", "bloodtinge", "arcane"] as const;
export type Attribute = (typeof ATTRIBUTES)[number];
export type CharacterFields = { echoes: string; insight: string } & Record<Attribute, string>;
/** Something to add: an item (with its amount), a weapon or armor piece, or a gem (by its first
 * effect, with the shape chosen). */
export type Addition = { item: CatalogItem; location: InventoryLocation; amount: number; shape?: number };
export type WeaponChoice = { level: number; imprint: number };

export const entryKey = (entry: InventoryEntry) => `${entry.location}:${entry.index}`;

/** A weapon's slots at a level and imprint, as the game gives them (the save's open slots when the
 * game's tables do not list the weapon), each with the gem it keeps: a gem stays only where it still fits. */
export function slotsAt(entry: InventoryEntry, choice: WeaponChoice, gems: Gem[]): WeaponSlot[] {
  const planned = entry.slot_plan.filter((p) => p.imprint === choice.imprint);
  if (!planned.length) return entry.slots;
  return planned.filter((p) => p.opens <= choice.level).map((p) => {
    const gem = entry.slots.find((s) => s.slot === p.slot)?.gem ?? null;
    const fits = gem !== null && ((gems.find((g) => g.index === gem)?.shape ?? 0) & p.shape) !== 0;
    return { slot: p.slot, shape: p.shape, gem: fits ? gem : null };
  });
}

const fieldsOf = (character: Character): CharacterFields => ({
  echoes: String(character.echoes), insight: String(character.insight),
  ...Object.fromEntries(ATTRIBUTES.map((field) => [field, String(character[field])])) as Record<Attribute, string>,
});

/** Everything the save editor has changed but not saved, and the edit it makes: the character's
 * fields and the inventory's changes, checked as the backend checks them. */
export function useSaveEdits(character: Character, inventory: Inventory | undefined) {
  const [fields, setFields] = useState(() => fieldsOf(character));
  // Inventory edits by entry (entryKey): amounts, a weapon's level and imprint, its slots' gems
  // (null: empty); gem effects by gem index, then effect slot; items to add.
  const [amounts, setAmounts] = useState<Record<string, string>>({});
  const [weapons, setWeapons] = useState<Record<string, WeaponChoice>>({});
  const [gemSlots, setGemSlots] = useState<Record<string, Record<number, number | null>>>({});
  const [effects, setEffects] = useState<Record<number, Record<number, number>>>({});
  const [additions, setAdditions] = useState<Addition[]>([]);
  const entries = useMemo(() => inventory?.entries ?? [], [inventory]);
  const gems = useMemo(() => inventory?.upgrades ?? [], [inventory]);

  const reset = () => {
    setFields(fieldsOf(character)); setAmounts({}); setWeapons({}); setGemSlots({}); setEffects({}); setAdditions([]);
  };

  // The character: the level follows the attributes; Vitality and Endurance have the game's minimums.
  const level = ATTRIBUTES.reduce((sum, field) => sum + Number(fields[field]), 0) - 50;
  const characterChanged = (Object.keys(fields) as (keyof CharacterFields)[]).some((field) => Number(fields[field]) !== character[field]);
  const characterValid = /^\d+$/.test(fields.echoes) && /^\d+$/.test(fields.insight)
    && Number(fields.echoes) <= 999_999_999 && Number(fields.insight) <= 99
    && ATTRIBUTES.every((field) => /^\d+$/.test(fields[field]) && Number(fields[field]) >= 1 && Number(fields[field]) <= 99)
    && (Number(fields.vitality) === character.vitality || Number(fields.vitality) >= 7)
    && (Number(fields.endurance) === character.endurance || Number(fields.endurance) >= 8)
    && level >= 4 && level <= 544;
  const characterEdit: CharacterEdit | null = characterChanged ? {
    title_id: character.title_id, slot: character.slot, fingerprint: character.fingerprint,
    echoes: Number(fields.echoes), insight: Number(fields.insight),
    vitality: Number(fields.vitality), endurance: Number(fields.endurance), strength: Number(fields.strength),
    skill: Number(fields.skill), bloodtinge: Number(fields.bloodtinge), arcane: Number(fields.arcane),
  } : null;

  const weapon = (entry: InventoryEntry): WeaponChoice => weapons[entryKey(entry)] ?? { level: entry.level, imprint: entry.imprint };
  const entryChanges = entries.flatMap((entry): InventoryChange[] => {
    const amount = amounts[entryKey(entry)];
    if (entry.kind === "item" && amount !== undefined && Number(amount) !== entry.amount) {
      return [{ type: "amount", location: entry.location, index: entry.index, amount: Number(amount) }];
    }
    const chosen = weapon(entry);
    if (entry.kind === "weapon" && (chosen.level !== entry.level || chosen.imprint !== entry.imprint)) {
      return [{ type: "weapon", location: entry.location, index: entry.index, level: chosen.level, imprint: chosen.imprint }];
    }
    return [];
  });
  // Each weapon slot's gem before and after the edit, at the weapon's chosen level and imprint: a slot
  // that closes or changes shape lets go of a gem that no longer fits.
  const slotted = entries.filter((entry) => entry.kind === "weapon").flatMap((entry) => {
    const chosen = gemSlots[entryKey(entry)] ?? {};
    const after = slotsAt(entry, weapon(entry), gems);
    const numbers = new Set([...entry.slots, ...after].map((slot) => slot.slot));
    return [...numbers].map((number) => {
      const slot = after.find((s) => s.slot === number);
      const gem = !slot ? null : number in chosen ? chosen[number] : slot.gem;
      return { entry, slot: number, before: entry.slots.find((s) => s.slot === number)?.gem ?? null, gem };
    });
  });
  const moved = slotted.filter(({ before, gem }) => gem !== before);
  // Gems come out first and go in last, so a gem can move between slots, and the weapon's level and
  // imprint (which open, close and reshape slots) change in between.
  const unequips = moved.flatMap(({ before }): InventoryChange[] => before === null ? [] : [{ type: "unequip", upgrade: before }]);
  const equips = moved.flatMap(({ entry, slot, gem }): InventoryChange[] => gem === null ? []
    : [{ type: "equip", upgrade: gem, location: entry.location, index: entry.index, slot }]);
  const inSlots = new Set(slotted.map(({ gem }) => gem));
  /** Whether the edit moves a gem in or out of a weapon's slot. */
  const gemMoved = (gem: number) => moved.some(({ before, gem: after }) => before === gem || after === gem);
  /** Where a gem is after the edit: a weapon's slot, or null. */
  const placeOf = (gem: number) => slotted.find((place) => place.gem === gem) ?? null;
  /** Puts a gem into a weapon's slot (out of the slot it was in), or takes it out (null). */
  const moveGem = (gem: number, to: { entry: InventoryEntry; slot: number } | null) => setGemSlots((all) => {
    const next = { ...all };
    const from = placeOf(gem);
    if (from) next[entryKey(from.entry)] = { ...next[entryKey(from.entry)], [from.slot]: null };
    if (to) next[entryKey(to.entry)] = { ...next[entryKey(to.entry)], [to.slot]: gem };
    return next;
  });
  const takenOut = new Set(moved.map(({ before }) => before));
  /** Whether a gem can go in a slot now: in the inventory or taken out in this edit, and in no other slot. */
  const available = (gem: Gem) => !inSlots.has(gem.index) && (gem.location === "inventory" || takenOut.has(gem.index));
  const effectChanges = gems.flatMap((gem) => gem.effects.flatMap((effect): InventoryChange[] => {
    const chosen = effects[gem.index]?.[effect.slot];
    return chosen !== undefined && chosen !== effect.id ? [{ type: "effect", upgrade: gem.index, slot: effect.slot, effect: chosen }] : [];
  }));
  const isEquipment = (addition: Addition) => addition.item.category === "weapon" || addition.item.category === "armor";
  const additionChanges = additions.map((addition): InventoryChange => addition.item.category === "gem"
    ? { type: "add_gem", shape: addition.shape ?? 1, effects: [addition.item.id] }
    : isEquipment(addition)
      ? { type: "add_equipment", location: addition.location, id: addition.item.id }
      : { type: "add", location: addition.location, id: addition.item.id, amount: addition.amount });
  const changes = [...unequips, ...entryChanges, ...equips, ...effectChanges, ...additionChanges];
  const amountsValid = entries.every((entry) => {
    const value = amounts[entryKey(entry)];
    return value === undefined || (/^\d+$/.test(value) && Number(value) >= 1 && Number(value) <= (entry.max || 999));
  });

  // Pending changes by section, for the editor's navigation.
  const changed = (entry: InventoryEntry) => changes.some((change) => "index" in change && change.location === entry.location && change.index === entry.index);
  const counts = {
    character: characterChanged ? 1 : 0,
    item: entries.filter((e) => e.location === "inventory" && e.kind === "item" && changed(e)).length
      + additions.filter((a) => a.location === "inventory" && !isEquipment(a) && a.item.category !== "gem").length,
    weapon: entries.filter((e) => e.kind === "weapon" && changed(e)).length
      + additions.filter((a) => a.location === "inventory" && a.item.category === "weapon").length,
    armor: additions.filter((a) => a.location === "inventory" && a.item.category === "armor").length,
    gems: new Set([...effectChanges.map((change) => change.type === "effect" && change.upgrade),
      ...moved.flatMap(({ before, gem }) => [before, gem]).filter((gem) => gem !== null)]).size
      + additions.filter((a) => a.item.category === "gem").length,
    storage: entries.filter((e) => e.location === "storage" && e.kind === "item" && changed(e)).length
      + additions.filter((a) => a.location === "storage").length,
  };
  const total = (characterChanged ? 1 : 0) + changes.length;

  return {
    fields, setFields, level, characterValid, characterEdit,
    amounts, setAmounts, weapon, setWeapons, gemSlots, setGemSlots, placeOf, moveGem, gemMoved, effects, setEffects, additions, setAdditions,
    available, changes, counts, total, changed,
    valid: (!characterChanged || characterValid) && amountsValid,
    reset,
  };
}

export type SaveEdits = ReturnType<typeof useSaveEdits>;
