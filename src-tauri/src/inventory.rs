// SPDX-License-Identifier: GPL-2.0-or-later
//! A character's inventory and storage in a decrypted Bloodborne save, and verified edits of them.
//! Format facts (entry layout, the sections' places relative to the character name, the weapon
//! table above the inventory) were checked against Noxde/Bloodborne-save-editor d8d0365 (GPL-3.0);
//! this parser is our own and refuses anything it does not recognize.
//!
//! An entry is 16 bytes: an ordering number, a handle (its high byte the kind: 0x80 weapon, 0x90
//! armor, 0xB0 item, 0xC0 gem or rune), the ID (a weapon's carries its level and imprint) and the
//! amount. A free entry has the ID 0xFFFFFFFF. Each weapon and armor piece also has a 60-byte block
//! above the inventory, keyed by the same handle and ID, with its five gem slots.
use crate::game_data::{GameParams, ItemText};
use crate::save_editor::{character_base, slot_bytes, slot_path};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

type Result<T> = std::result::Result<T, String>;

const ENTRY: usize = 16;
// The sections' sizes in entries (the 1,984th inventory entry's bytes belong to what follows it).
fn capacity(location: Location) -> usize {
    match location {
        Location::Inventory => 1983,
        Location::Storage => 1984,
    }
}
const INVENTORY_FROM_NAME: usize = 469;
const STORAGE_FROM_INVENTORY: usize = 34268;
const FREE_ID: u32 = 0xFFFF_FFFF;
const BARE_FISTS: u32 = 1_000_000; // the empty hand slots the game keeps as weapons
const RUNE_SLOTS: u32 = 1000; // the blocks that hold the equipped runes, keyed like weapons
const NO_ARMOR: u32 = 900_000; // armor from here up is the bare body's placeholders ("NOT NEEDED")
const UPGRADES_START: usize = 84; // gems and runes, before the weapon table
const SLOT_BLOCK: usize = 60;
const SLOT_CLOSED: u32 = 0x8000_0000;
const NO_GEM: u32 = 0;
// The counters of a section, from the character name: (entries ever taken, at least those in use;
// the next ordering number). Adding an entry moves both on.
const INVENTORY_COUNTERS: (usize, usize) = (453, 34257);
const STORAGE_COUNTERS: (usize, usize) = (34737, 68541);
const STORAGE_MAX: u32 = 99;
// EquipParamGoods' goodsType: what the save editor may add to the inventory (key items live in a
// list of their own).
const GOODS_ADDABLE: [(u32, &str); 3] = [(0, "consumable"), (2, "material"), (6, "chalice")];

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    Inventory,
    Storage,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Item,
    Weapon,
    Armor,
    Upgrade,
}

#[derive(Serialize, Debug)]
pub struct Entry {
    pub location: Location,
    /// The entry's position in its section (what an edit refers to).
    pub index: u16,
    pub kind: Kind,
    /// Items: the item; weapons: the full weapon ID (level and imprint included); armor: the piece.
    pub id: u32,
    pub amount: u32,
    pub name: Option<String>,
    /// Items: the short description the game lists them with.
    pub description: Option<String>,
    /// The game's long description (its lore), for the item, the weapon at its level and imprint,
    /// or the armor piece.
    pub caption: Option<String>,
    /// Weapons: +0 to +10, and 0 none, 1 Uncanny, 2 Lost.
    pub level: u8,
    pub imprint: u8,
    /// Weapons: the levels and imprints the player's game has names for (valid IDs).
    pub levels: Vec<u8>,
    pub imprints: Vec<u8>,
    /// The game's own name of each imprint's variant, in the player's language.
    pub imprint_names: Vec<String>,
    /// Items: the most this section holds of it.
    pub max: u32,
    /// The game's icon number (iconId), for game_data::icon.
    pub icon: Option<u32>,
    /// What the hover card shows: from the game's parameter tables.
    pub stats: Option<serde_json::Value>,
    /// Weapons: the hover card's numbers at each level and imprint it can change to, by weapon ID.
    pub variants: std::collections::BTreeMap<u32, serde_json::Value>,
    /// Weapons: their open gem slots.
    pub slots: Vec<WeaponSlot>,
    /// Weapons: the gem slots the game gives them at each level and imprint (empty when the game's
    /// tables are missing or do not list the weapon).
    pub slot_plan: Vec<PlannedSlot>,
}

/// A gem slot the game gives a weapon: for an imprint, the slot's shape and the level it opens at.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct PlannedSlot {
    pub imprint: u8,
    pub slot: u8,
    pub opens: u8,
    pub shape: u32,
}

/// An open gem slot of a weapon: its shape (the gem shapes it takes, as bits: 1 radial, 2
/// triangle, 4 waning, 8 circle) and the gem in it (its index in `Inventory::upgrades`).
#[derive(Serialize, Debug)]
pub struct WeaponSlot {
    pub slot: u8,
    pub shape: u32,
    pub gem: Option<u16>,
}

/// One item the save editor can add (catalog).
#[derive(Serialize)]
pub struct CatalogItem {
    pub id: u32,
    pub name: String,
    /// consumable, material, chalice (items), weapon or armor.
    pub category: &'static str,
    pub max: u32,
    pub icon: Option<u32>,
    /// Items: the short description; weapons and armor: none.
    pub description: Option<String>,
    /// The game's long description, and the hover card's numbers.
    pub caption: Option<String>,
    pub stats: Option<serde_json::Value>,
}

/// The items the game has names for and the editor can add: consumables, materials, chalices.
pub fn catalog(text: &ItemText, params: &GameParams) -> Vec<CatalogItem> {
    let mut items: Vec<CatalogItem> = params
        .goods
        .ids()
        .filter_map(|id| {
            let kind = params.goods.get(id, "goodsType")? as u32;
            let category = GOODS_ADDABLE.iter().find(|(k, _)| *k == kind)?.1;
            let name = text.goods.get(&id)?;
            if name.is_empty() || name.starts_with('?') {
                return None;
            }
            Some(CatalogItem {
                id,
                name: name.clone(),
                category,
                max: params.goods.get(id, "maxNum").unwrap_or(1.0).max(1.0) as u32,
                icon: params.goods.get(id, "iconId").map(|v| v as u32),
                description: text.goods_info.get(&id).cloned(),
                caption: text.goods_caption.get(&id).cloned(),
                stats: stats_of(Kind::Item, id, params),
            })
        })
        .collect();
    // Gems: one per effect the game has a gem of, added with the shape chosen.
    let mut gems: Vec<(u32, u32)> = gem_sources(params).into_iter().collect();
    gems.sort_unstable();
    items.extend(gems.into_iter().filter_map(|(effect, _)| Some(CatalogItem {
        id: effect,
        name: text.gem_effects.get(&effect)?.clone(),
        category: "gem",
        max: 1,
        // As a radial gem; the dialog shows the shape chosen (the same offset from its first icon).
        icon: gem_icon(params, false, 1, effect),
        description: None,
        caption: None,
        stats: None,
    })));
    // Weapons at +0 without an imprint, and armor pieces: one each per addition.
    items.extend(params.weapons.ids().filter(|&id| addable_weapon(id, text, params)).map(|id| CatalogItem {
        id,
        name: text.weapons[&id].clone(),
        category: "weapon",
        max: 1,
        icon: icon_of(Kind::Weapon, id, params),
        description: None,
        caption: text.weapon_caption.get(&id).cloned(),
        stats: stats_of(Kind::Weapon, id, params),
    }));
    items.extend(params.armor.ids().filter(|&id| addable_armor(id, text, params)).map(|id| CatalogItem {
        id,
        name: text.armor[&id].clone(),
        category: "armor",
        max: 1,
        icon: icon_of(Kind::Armor, id, params),
        description: None,
        caption: text.armor_caption.get(&id).cloned(),
        stats: stats_of(Kind::Armor, id, params),
    }));
    items.sort_by(|a, b| (a.category, a.id).cmp(&(b.category, b.id)));
    items
}

/// A weapon the editor can add: a named base weapon (+0, no imprint), not the empty hand or a
/// placeholder (named like the rune slots' block, "*").
fn addable_weapon(id: u32, text: &ItemText, params: &GameParams) -> bool {
    let placeholder = text.weapons.get(&RUNE_SLOTS);
    id >= 100_000 && id.is_multiple_of(100_000) && id != BARE_FISTS && params.weapons.has(id)
        && text.weapons.get(&id).is_some_and(|name| Some(name) != placeholder)
}

/// An armor piece the editor can add: a named one, not the bare body or a placeholder (named like
/// the bare body's, "NOT NEEDED" in English).
fn addable_armor(id: u32, text: &ItemText, params: &GameParams) -> bool {
    let placeholder = text.armor.get(&NO_ARMOR);
    id < NO_ARMOR && params.armor.has(id) && text.armor.get(&id).is_some_and(|name| Some(name) != placeholder)
}

fn stat(params: &crate::game_data::Param, id: u32, fields: &[(&str, &str)]) -> serde_json::Value {
    serde_json::Value::Object(
        fields
            .iter()
            .filter_map(|(key, field)| Some((key.to_string(), serde_json::json!(params.get(id, field)?.round()))))
            .collect(),
    )
}

/// The hover card's numbers. Bloodborne kept Dark Souls' field names: "magic" is Bloodtinge in
/// requirements and scaling and Arcane in attack and defense, "faith" is Arcane, "thunder" Bolt
/// (checked against the Chikage, the Holy Moonlight Sword, the Blade of Mercy and Tonitrus).
fn stats_of(kind: Kind, id: u32, params: &GameParams) -> Option<serde_json::Value> {
    match kind {
        Kind::Weapon if params.weapons.has(id) => Some(serde_json::json!({
            "attack": stat(&params.weapons, id, &[("physical", "attackBasePhysics"), ("arcane", "attackBaseMagic"), ("fire", "attackBaseFire"), ("bolt", "attackBaseThunder")]),
            "scaling": stat(&params.weapons, id, &[("strength", "correctStrength"), ("skill", "correctAgility"), ("bloodtinge", "correctMagic"), ("arcane", "correctFaith")]),
            "requirements": stat(&params.weapons, id, &[("strength", "properStrength"), ("skill", "properAgility"), ("bloodtinge", "properMagic"), ("arcane", "properFaith")]),
        })),
        Kind::Armor if params.armor.has(id) => Some(serde_json::json!({
            "defense": stat(&params.armor, id, &[("physical", "defensePhysics"), ("blunt", "defenseBlow"), ("thrust", "defenseThrust"), ("arcane", "defenseMagic"), ("fire", "defenseFire"), ("bolt", "defenseThunder")]),
        })),
        Kind::Item if params.goods.has(id) => Some(serde_json::json!({
            "max": params.goods.get(id, "maxNum")?,
        })),
        _ => None,
    }
}

fn icon_of(kind: Kind, id: u32, params: &GameParams) -> Option<u32> {
    let value = match kind {
        Kind::Item => params.goods.get(id, "iconId"),
        Kind::Weapon => params.weapons.get(id, "iconId"),
        Kind::Armor => params.armor.get(id, "iconIdM"),
        Kind::Upgrade => None,
    }?;
    (value > 0.0).then_some(value as u32)
}

#[derive(Serialize)]
pub struct Inventory {
    pub title_id: String,
    pub slot: u8,
    pub fingerprint: String,
    pub entries: Vec<Entry>,
    pub upgrades: Vec<Upgrade>,
}

/// A gem or rune: a 40-byte record from byte 84 of the file (handle, source, type, shape and six
/// effect IDs, 0xFFFFFFFF for none), referenced by an inventory entry or a weapon's slot.
#[derive(Serialize, Debug)]
pub struct Upgrade {
    /// Its position in the list (what an edit refers to) and its handle.
    pub index: u16,
    pub handle: u32,
    pub rune: bool,
    /// Gems: 1 radial, 2 triangle, 4 waning, 8 circle, 63 droplet; runes: 1, 2 (Oath) or 4.
    pub shape: u8,
    pub effects: Vec<Effect>,
    /// The game's icon number (gem_icon), for game_data::icon.
    pub icon: Option<u32>,
    /// Where it is: an inventory or storage entry, or a weapon's slot (the weapon's name).
    pub location: Option<Location>,
    /// In a slot: the weapon's name when the slot is a weapon's (runes have slots of their own).
    pub equipped: bool,
    pub weapon: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Effect {
    /// Its position among the record's six (what an edit refers to).
    pub slot: u8,
    pub id: u32,
    pub text: Option<String>,
    /// The same kind of effect at its other strengths (the game's effects of the same category),
    /// weakest first: what this effect can be changed to.
    pub options: Vec<EffectOption>,
}

#[derive(Serialize, Debug)]
pub struct EffectOption {
    pub id: u32,
    pub text: String,
}

/// The effects of the same category as `id` that the game names, by rank: the alternatives an
/// edit may choose. A rune's effects and a gem's never mix (their categories differ).
fn effect_options(id: u32, text: &ItemText, params: &GameParams) -> Vec<EffectOption> {
    let Some(category) = params.gem_effects.get(id, "categoryId") else {
        return Vec::new();
    };
    let mut options: Vec<(f64, u32)> = params
        .gem_effects
        .ids()
        .filter(|&other| params.gem_effects.get(other, "categoryId") == Some(category))
        .filter(|other| text.gem_effects.contains_key(other))
        .map(|other| (params.gem_effects.get(other, "effectRank").unwrap_or(0.0), other))
        .collect();
    options.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    options.into_iter().map(|(_, other)| EffectOption { id: other, text: text.gem_effects[&other].clone() }).collect()
}

const UPGRADE: usize = 40;
const NO_EFFECT: u32 = 0xFFFF_FFFF;
const GEM_SHAPES: [u8; 5] = [1, 2, 4, 8, 63];
// Runes: 1 and 2 (Oath) as known, 4 seen in a real save, 16 written by the game for that same
// rune (a save it wrote back in a test run).
const RUNE_SHAPES: [u8; 4] = [1, 2, 4, 16];

/// A gem or rune record: (its offset, handle, whether a rune, shape, effect IDs).
type Record = (usize, u32, bool, u8, [u32; 6]);
/// A weapon's or armor piece's slot block: (its offset, the weapon or armor ID, the five slots'
/// (shape, gem handle)).
type SlotBlock = (usize, u32, [(u32, u32); 5]);

/// The table before the character: 4,096 entries from byte 84, each a gem or rune record (40 bytes,
/// handle 0xC0...), a weapon's or armor piece's block (60 bytes, handle 0x8080... or 0x9080...) or a
/// free entry (8 bytes), in any order (Noxde/Bloodborne-save-editor d8d0365 walks it the same way).
/// Returns (offset, the handle's high byte) of every entry that is not free, and the table's end.
fn table(bytes: &[u8]) -> Result<(Vec<(usize, u8)>, usize)> {
    const ENTRIES: usize = 4096;
    let mut used = Vec::new();
    let mut at = UPGRADES_START;
    for _ in 0..ENTRIES {
        let kind = *bytes.get(at + 3).ok_or("Unrecognized weapon table.")?;
        match kind {
            0x80 | 0x90 => { used.push((at, kind)); at += SLOT_BLOCK; }
            0xC0 => { used.push((at, kind)); at += UPGRADE; }
            0x00 => at += 8,
            _ => return Err("Unrecognized weapon table.".into()),
        }
    }
    Ok((used, at))
}

/// The gem and rune records, in table order (what an edit's gem index refers to).
fn upgrade_records(bytes: &[u8]) -> Result<Vec<Record>> {
    let mut found = Vec::new();
    for (at, _) in table(bytes)?.0.into_iter().filter(|&(_, kind)| kind == 0xC0) {
        let record = &bytes[at..at + UPGRADE];
        let (kind, shape) = (record[8], record[12]);
        let valid = match kind {
            1 => GEM_SHAPES.contains(&shape),
            2 => RUNE_SHAPES.contains(&shape),
            _ => false,
        };
        if !valid || record[9..12] != [0, 0, 0] || record[13..16] != [0, 0, 0] {
            return Err("Unrecognized gem record.".into());
        }
        let mut effects = [0u32; 6];
        for (i, effect) in effects.iter_mut().enumerate() {
            *effect = word(record, 16 + i * 4)?;
        }
        found.push((at, word(record, 0)?, kind == 2, shape, effects));
    }
    Ok(found)
}

/// The weapons' and armor's blocks: (block offset, its weapon or armor ID, the five slots' (shape,
/// gem handle)). Equipped runes sit in blocks too (the character's rune slots).
fn slot_blocks(bytes: &[u8]) -> Result<Vec<SlotBlock>> {
    let mut blocks = Vec::new();
    for (at, _) in table(bytes)?.0.into_iter().filter(|&(_, kind)| kind != 0xC0) {
        let mut slots = [(0u32, 0u32); 5];
        for (i, slot) in slots.iter_mut().enumerate() {
            *slot = (word(bytes, at + 20 + i * 8)?, word(bytes, at + 24 + i * 8)?);
        }
        blocks.push((at, word(bytes, at + 4)?, slots));
    }
    Ok(blocks)
}

fn upgrades(bytes: &[u8], entries: &[Entry], text: Option<&ItemText>, params: Option<&GameParams>) -> Result<Vec<Upgrade>> {
    let blocks = slot_blocks(bytes)?;
    // Every handle a slot holds: weapons' and armor's blocks, and the rune slots' blocks.
    let slotted: std::collections::HashSet<u32> = blocks
        .iter()
        .flat_map(|(_, _, slots)| slots.iter().map(|&(_, gem)| gem))
        .filter(|handle| handle >> 24 == 0xC0)
        .collect();
    let mut held = std::collections::HashMap::new();
    for location in [Location::Inventory, Location::Storage] {
        let start = section(bytes, location)?;
        for entry in entries.iter().filter(|e| e.location == location && e.kind == Kind::Upgrade) {
            held.insert(word(bytes, start + usize::from(entry.index) * ENTRY + 4)?, location);
        }
    }
    // Records nothing refers to (gems and runes discarded or used up) are left out.
    Ok(upgrade_records(bytes)?
        .into_iter()
        .enumerate()
        .filter(|(_, (_, handle, ..))| held.contains_key(handle) || slotted.contains(handle))
        .map(|(index, (_, handle, rune, shape, effects))| {
            let block = blocks.iter().find(|(_, _, slots)| slots.iter().any(|(_, gem)| *gem == handle));
            let weapon = block.and_then(|(_, weapon, _)| {
                let (base, _, _) = weapon_parts(*weapon);
                text.and_then(|t| t.weapons.get(weapon).or_else(|| t.weapons.get(&base)).cloned())
            });
            Upgrade {
                index: index as u16,
                handle,
                rune,
                shape,
                effects: effects
                    .iter()
                    .enumerate()
                    .filter(|&(_, &id)| id != NO_EFFECT)
                    .map(|(slot, &id)| Effect {
                        slot: slot as u8,
                        id,
                        text: text.and_then(|t| t.gem_effects.get(&id).cloned()),
                        options: match (text, params) {
                            (Some(text), Some(params)) => effect_options(id, text, params),
                            _ => Vec::new(),
                        },
                    })
                    .collect(),
                icon: params.and_then(|params| gem_icon(params, rune, shape, *effects.iter().find(|&&id| id != NO_EFFECT)?)),
                location: held.get(&handle).copied(),
                equipped: slotted.contains(&handle),
                weapon,
            }
        })
        .collect())
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    Amount { location: Location, index: u16, amount: u32 },
    Weapon { location: Location, index: u16, level: u8, imprint: u8 },
    Add { location: Location, id: u32, amount: u32 },
    /// A gem's or rune's effect (by its position among the record's six) changed to another of
    /// the same category.
    Effect { upgrade: u16, slot: u8, effect: u32 },
    /// A gem from the inventory into an empty, open slot of a weapon whose shape takes it.
    Equip { upgrade: u16, location: Location, index: u16, slot: u8 },
    /// A gem out of its weapon's slot, back into the inventory.
    Unequip { upgrade: u16 },
    /// A weapon (+0, no imprint) or armor piece into the inventory or storage.
    AddEquipment { location: Location, id: u32 },
    /// A new gem into the inventory: its shape and effects (the first one names the game's row
    /// that generates such a gem, its source).
    AddGem { shape: u8, effects: Vec<u32> },
}

#[derive(Deserialize)]
pub struct Edit {
    pub title_id: String,
    pub slot: u8,
    pub fingerprint: String,
    pub changes: Vec<Change>,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes.get(at..at + 4).ok_or("Truncated inventory.")?.try_into().unwrap(),
    ))
}

/// Where a section starts in the character file.
fn section(bytes: &[u8], location: Location) -> Result<usize> {
    let base = character_base(bytes)?.ok_or("The character slot is empty.")?;
    let inventory = base + INVENTORY_FROM_NAME;
    let start = match location {
        Location::Inventory => inventory,
        Location::Storage => inventory + STORAGE_FROM_INVENTORY,
    };
    if start + capacity(location) * ENTRY > bytes.len() {
        return Err("Unrecognized inventory layout.".into());
    }
    Ok(start)
}

/// A weapon ID's parts: (base ID, level, imprint).
fn weapon_parts(id: u32) -> (u32, u8, u8) {
    let base = id / 100_000 * 100_000;
    ((base), ((id % 10_000) / 100) as u8, ((id % 100_000) / 10_000) as u8)
}
fn weapon_id(base: u32, level: u8, imprint: u8) -> u32 {
    base + u32::from(imprint) * 10_000 + u32::from(level) * 100
}

fn kind_of(handle: u32, id: u32) -> Option<(Kind, u32)> {
    match (handle >> 24, id >> 24) {
        (0xB0, 0x40) => Some((Kind::Item, id & 0x00FF_FFFF)),
        (0x90, 0x10) => Some((Kind::Armor, id & 0x0FFF_FFFF)),
        (0x80, _) => Some((Kind::Weapon, id)),
        (0xC0, _) => Some((Kind::Upgrade, id)),
        _ => None,
    }
}

fn entries(bytes: &[u8], text: Option<&ItemText>, params: Option<&GameParams>) -> Result<Vec<Entry>> {
    let mut found = Vec::new();
    for location in [Location::Inventory, Location::Storage] {
        let start = section(bytes, location)?;
        for index in 0..capacity(location) {
            let at = start + index * ENTRY;
            let (handle, id, amount) = (word(bytes, at + 4)?, word(bytes, at + 8)?, word(bytes, at + 12)?);
            if id == FREE_ID {
                continue;
            }
            let Some((kind, item)) = kind_of(handle, id) else {
                return Err(format!("Unrecognized inventory entry {index}."));
            };
            if kind == Kind::Weapon && item == BARE_FISTS {
                continue;
            }
            let mut entry = Entry {
                location,
                index: index as u16,
                kind,
                id: item,
                amount,
                name: None,
                description: None,
                caption: None,
                level: 0,
                imprint: 0,
                levels: Vec::new(),
                imprints: Vec::new(),
                imprint_names: Vec::new(),
                max: 0,
                icon: None,
                stats: None,
                variants: Default::default(),
                slots: Vec::new(),
                slot_plan: Vec::new(),
            };
            if let Some(params) = params {
                entry.icon = icon_of(kind, item, params);
                entry.stats = stats_of(kind, item, params);
                if kind == Kind::Item {
                    entry.max = item_max(params, item, location).unwrap_or(STORAGE_MAX);
                }
            }
            if let Some(text) = text {
                match kind {
                    Kind::Item => {
                        entry.name = text.goods.get(&item).cloned();
                        entry.description = text.goods_info.get(&item).cloned();
                        entry.caption = text.goods_caption.get(&item).cloned();
                    }
                    Kind::Armor => {
                        entry.name = text.armor.get(&item).cloned();
                        entry.caption = text.armor_caption.get(&item).cloned();
                    }
                    Kind::Weapon => {
                        let (base, level, imprint) = weapon_parts(item);
                        entry.level = level;
                        entry.imprint = imprint;
                        // The game's name for the full ID ("Uncanny Saw Cleaver +10"): the base ID's
                        // can differ (pt-BR's 7000000 has a typo the upgraded names do not).
                        entry.name = text.weapons.get(&item).or_else(|| text.weapons.get(&base)).cloned();
                        entry.caption = text.weapon_caption.get(&item).or_else(|| text.weapon_caption.get(&base)).cloned();
                        entry.levels = (0..=10)
                            .filter(|&l| text.weapons.contains_key(&weapon_id(base, l, imprint)))
                            .collect();
                        entry.imprints = (0..=2)
                            .filter(|&i| text.weapons.contains_key(&weapon_id(base, level, i)))
                            .collect();
                        // Each imprint's name without a level: its +1 name less " +1" where it has one.
                        entry.imprint_names = entry
                            .imprints
                            .iter()
                            .map(|&i| {
                                text.weapons
                                    .get(&weapon_id(base, 1, i))
                                    .and_then(|name| name.strip_suffix(" +1"))
                                    .map(str::to_string)
                                    .unwrap_or_else(|| text.weapons[&weapon_id(base, 0, i)].clone())
                            })
                            .collect();
                        if let Some(params) = params {
                            entry.variants = (0..=2)
                                .flat_map(|i| (0..=10).map(move |l| weapon_id(base, l, i)))
                                .filter(|id| text.weapons.contains_key(id))
                                .filter_map(|id| Some((id, stats_of(Kind::Weapon, id, params)?)))
                                .collect();
                        }
                    }
                    Kind::Upgrade => {}
                }
            }
            found.push(entry);
        }
    }
    Ok(found)
}

/// The most a section holds of an item: the game's maxNum in the inventory, 99 in storage.
fn item_max(params: &GameParams, id: u32, location: Location) -> Option<u32> {
    let max = params.goods.get(id, "maxNum")?.max(1.0) as u32;
    Some(match location {
        Location::Inventory => max,
        Location::Storage => STORAGE_MAX.max(max),
    })
}

/// A gem's or rune's icon: its shape's first icon (gems: 3000 radial, 3200 triangle, 3400 waning,
/// 3600 circle, 4000 droplet, each followed by a cursed set at +100 not used here; runes from 3000)
/// plus its first effect's gemIconIdOffset, which picks the colour (tens) and how elaborate it is
/// (units, 0-6). Checked against the game's menu icons: gems land on 3000-4166, Caryll runes on
/// 4200-4254.
fn gem_icon(params: &GameParams, rune: bool, shape: u8, effect: u32) -> Option<u32> {
    let base = if rune {
        3000
    } else {
        match shape {
            1 => 3000,
            2 => 3200,
            4 => 3400,
            8 => 3600,
            63 => 4000,
            _ => return None,
        }
    };
    // Offsets outside the sets (a few special effects, runes without one) have no icon here.
    let offset = params.gem_effects.get(effect, "gemIconIdOffset")? as u32;
    let fits = if rune { (1000..1300).contains(&offset) } else { offset % 10 <= 6 && offset < 70 };
    fits.then_some(base + offset)
}

/// The game's rows that generate a weapon gem with one fixed effect (GemGenParam), by that effect:
/// what a new gem's source is (the game keeps it as the record's second word and the inventory
/// entry's ID, 0x80000000 | row).
fn gem_sources(params: &GameParams) -> std::collections::HashMap<u32, u32> {
    let table = &params.gem_sources;
    let mut found = std::collections::HashMap::new();
    let mut rows: Vec<u32> = table.ids().collect();
    rows.sort_unstable();
    for row in rows {
        let field = |name: &str| table.get(row, name).unwrap_or(-1.0);
        let fixed = field("gemeffectGenParamType_0") == 1.0 && (1..6).all(|i| field(&format!("gemeffectGenParamType_{i}")) == 0.0);
        if field("equipableSegmentWeapon") == 1.0 && fixed && field("gemeffectGenParam_0") >= 0.0 {
            found.entry(field("gemeffectGenParam_0") as u32).or_insert(row);
        }
    }
    found
}

/// Adds a gem record and its inventory entry. The record (40 bytes) takes the place of a free
/// table entry (8 bytes), so everything after the table moves 32 bytes on, and the file's last 32
/// bytes go (the file has a fixed size; the game writes the character from the start of it and
/// leaves earlier writes' bytes after its end). Returns the entry's index and ID, and where the
/// record went. Refused for files that hold two copies of the character, as additions are.
fn add_gem(bytes: &mut Vec<u8>, shape: u8, effects: &[u32], text: &ItemText, params: &GameParams) -> Result<(u16, u32, usize)> {
    if !GEM_SHAPES.contains(&shape) {
        return Err("Unknown gem shape.".into());
    }
    if effects.is_empty() || effects.len() > 6 || effects.iter().any(|e| !text.gem_effects.contains_key(e) || params.gem_effects.get(*e, "categoryId").is_none()) {
        return Err("A gem has one to six of the game's effects.".into());
    }
    let row = *gem_sources(params).get(&effects[0]).ok_or("The game has no gem with this effect.")?;
    if crate::save_editor::copy_count(bytes)? != 1 {
        return Err("Gems cannot be added to this character yet: the file holds two copies of it.".into());
    }
    let (used, _) = table(bytes)?;
    let mut at = UPGRADES_START;
    let mut free = None;
    for _ in 0..4096 {
        match bytes[at + 3] {
            0x80 | 0x90 => at += SLOT_BLOCK,
            0xC0 => at += UPGRADE,
            _ => {
                free.get_or_insert(at);
                at += 8;
            }
        }
    }
    let free = free.ok_or("The character has no free gem record left.")?;
    let next = used.iter().map(|&(at, _)| word(bytes, at).map(|h| h & 0x00FF_FFFF)).collect::<Result<Vec<_>>>()?.into_iter().max().unwrap_or(0x80_0000) + 1;
    if next > 0x00FF_FFFF {
        return Err("The character has no free gem record left.".into());
    }
    let (handle, source) = (0xC000_0000 | next, 0x8000_0000 | row);
    let mut record = Vec::with_capacity(UPGRADE);
    record.extend_from_slice(&handle.to_le_bytes());
    record.extend_from_slice(&source.to_le_bytes());
    record.extend_from_slice(&[1, 0, 0, 0, shape, 0, 0, 0]);
    for i in 0..6 {
        record.extend_from_slice(&effects.get(i).copied().unwrap_or(NO_EFFECT).to_le_bytes());
    }
    let size = bytes.len();
    bytes.splice(free..free + 8, record);
    bytes.truncate(size);
    let index = add_entry(bytes, Location::Inventory, handle, source, 1)?;
    Ok((index, source, free))
}

/// The game's gem slots by weapon ID (WeaponGenParam: one row per ID, level and imprint included;
/// a slot's shape, or a closed slot with the high bit set).
type SlotRows = std::collections::HashMap<u32, Vec<[u32; 5]>>;

fn slot_rows(params: &GameParams) -> SlotRows {
    let table = &params.weapon_slots;
    let mut rows = SlotRows::new();
    for row in table.ids() {
        let Some(id) = table.get(row, "wepParamId") else { continue };
        let shapes = std::array::from_fn(|i| table.get(row, &format!("gemSlotType_{i}")).map_or(SLOT_CLOSED, |v| v as u32));
        rows.entry(id as u32).or_default().push(shapes);
    }
    rows
}

/// A weapon's slots for each imprint: each slot's shape where it first opens (+1, +3 and +6 for most
/// weapons). A few DLC weapons have rows that disagree at +9 and +10; their slots opened earlier.
fn slot_plan(rows: &SlotRows, base: u32) -> Vec<PlannedSlot> {
    let mut plan: Vec<PlannedSlot> = Vec::new();
    for imprint in 0..=2u8 {
        for level in 0..=10u8 {
            let Some(found) = rows.get(&weapon_id(base, level, imprint)) else { continue };
            for slot in 0..5u8 {
                let open = found.iter().map(|shapes| shapes[usize::from(slot)]).find(|&shape| shape != 0 && shape & SLOT_CLOSED == 0);
                if let Some(shape) = open.filter(|_| !plan.iter().any(|p| p.imprint == imprint && p.slot == slot)) {
                    plan.push(PlannedSlot { imprint, slot, opens: level, shape });
                }
            }
        }
    }
    plan
}

/// A slot's shape at a level and imprint, by the plan; None: closed.
fn planned_shape(plan: &[PlannedSlot], slot: u8, level: u8, imprint: u8) -> Option<u32> {
    plan.iter().find(|p| p.imprint == imprint && p.slot == slot && p.opens <= level).map(|p| p.shape)
}

pub fn read(user: &Path, title: &str, slot: u8, text: Option<&ItemText>, params: Option<&GameParams>) -> Result<Inventory> {
    let bytes = slot_bytes(&slot_path(user, title, slot)?)?;
    let mut entries = entries(&bytes, text, params)?;
    let upgrades = upgrades(&bytes, &entries, text, params)?;
    let rows = params.map(slot_rows).unwrap_or_default();
    for entry in entries.iter_mut().filter(|e| e.kind == Kind::Weapon) {
        entry.slot_plan = slot_plan(&rows, weapon_parts(entry.id).0);
        let at = section(&bytes, entry.location)? + usize::from(entry.index) * ENTRY;
        // A weapon without its block is shown without slots (and its level cannot change).
        let Ok(block) = slot_block(&bytes, word(&bytes, at + 4)?, word(&bytes, at + 8)?) else {
            continue;
        };
        for slot in 0..5u8 {
            let (shape, gem) = slot_at(&bytes, block, slot)?;
            if shape != SLOT_CLOSED && shape != 0 {
                let gem = upgrades.iter().find(|u| gem != NO_GEM && u.handle == gem).map(|u| u.index);
                entry.slots.push(WeaponSlot { slot, shape, gem });
            }
        }
    }
    Ok(Inventory {
        title_id: title.into(),
        slot,
        fingerprint: format!("{:x}", Sha256::digest(&bytes)),
        upgrades,
        entries,
    })
}

/// A slot of a slot block: (its shape, the handle of the gem in it or 0).
fn slot_at(bytes: &[u8], block: usize, slot: u8) -> Result<(u32, u32)> {
    let at = block + 20 + usize::from(slot) * 8;
    Ok((word(bytes, at)?, word(bytes, at + 4)?))
}

/// Where a gem's handle is held: the inventory entry (index) that refers to it.
fn gem_entry(bytes: &[u8], handle: u32) -> Result<Option<u16>> {
    let start = section(bytes, Location::Inventory)?;
    for index in 0..capacity(Location::Inventory) {
        if word(bytes, start + index * ENTRY + 4)? == handle && word(bytes, start + index * ENTRY + 8)? != FREE_ID {
            return Ok(Some(index as u16));
        }
    }
    Ok(None)
}

/// The weapon slot that holds a gem: (block offset, slot), from the weapons' blocks only.
fn gem_slot(bytes: &[u8], handle: u32) -> Result<Option<(usize, u8)>> {
    let blocks = slot_blocks(bytes)?;
    let mut found = blocks.iter().filter(|(at, id, _)| word(bytes, *at).map(|h| h >> 16 == 0x8080).unwrap_or(false) && *id != RUNE_SLOTS)
        .flat_map(|(at, _, slots)| slots.iter().enumerate().filter(|(_, (_, gem))| *gem == handle).map(move |(i, _)| (*at, i as u8)));
    match (found.next(), found.next()) {
        (Some(slot), None) => Ok(Some(slot)),
        (None, _) => Ok(None),
        _ => Err("The gem is in more than one slot.".into()),
    }
}

/// The 60-byte block above the inventory for this handle and ID: exactly one, or an error.
fn slot_block(bytes: &[u8], handle: u32, id: u32) -> Result<usize> {
    let base = character_base(bytes)?.ok_or("The character slot is empty.")?;
    let end = base.checked_sub(147).ok_or("Invalid character layout.")?;
    let mut key = [0u8; 8];
    key[..4].copy_from_slice(&handle.to_le_bytes());
    key[4..].copy_from_slice(&id.to_le_bytes());
    let mut matches = (UPGRADES_START..end.saturating_sub(SLOT_BLOCK))
        .filter(|&at| bytes[at..at + 8] == key);
    match (matches.next(), matches.next()) {
        (Some(at), None) => Ok(at),
        (None, _) => Err("The weapon's slot block was not found.".into()),
        _ => Err("The weapon's slot block is ambiguous.".into()),
    }
}

/// Adds an item in the section's first free entry. An item the section already holds is refused
/// (its amount is what to change).
fn add_item(bytes: &mut [u8], location: Location, id: u32, amount: u32) -> Result<u16> {
    add_entry(bytes, location, 0xB000_0000 | id, 0x4000_0000 | id, amount)
}

/// Writes an entry in the section's first free one: the handle and ID words, the amount, the next
/// ordering number, and both counters moved on.
fn add_entry(bytes: &mut [u8], location: Location, new_handle: u32, new_id: u32, amount: u32) -> Result<u16> {
    let base = character_base(bytes)?.ok_or("The character slot is empty.")?;
    let start = section(bytes, location)?;
    let (count_at, next_at) = match location {
        Location::Inventory => INVENTORY_COUNTERS,
        Location::Storage => STORAGE_COUNTERS,
    };
    let (count_at, next_at) = (base + count_at, base + next_at);
    let (count, next) = (word(bytes, count_at)?, word(bytes, next_at)?);
    let mut free = None;
    let mut used = 0u32;
    for index in 0..capacity(location) {
        let at = start + index * ENTRY;
        let (handle, entry_id) = (word(bytes, at + 4)?, word(bytes, at + 8)?);
        if entry_id == FREE_ID {
            free.get_or_insert(index);
            continue;
        }
        used += 1;
        if handle == new_handle {
            return Err("The character already has this item: change its amount instead.".into());
        }
    }
    // The first counter is at least the entries in use (the game does not lower it when a stack
    // runs out); one below that means this is not the layout this editor knows.
    if count < used {
        return Err("The section's counter does not match its entries.".into());
    }
    let index = free.ok_or("This section is full.")?;
    let at = start + index * ENTRY;
    bytes[at] = next as u8;
    bytes[at + 4..at + 8].copy_from_slice(&new_handle.to_le_bytes());
    bytes[at + 8..at + 12].copy_from_slice(&new_id.to_le_bytes());
    bytes[at + 12..at + 16].copy_from_slice(&amount.to_le_bytes());
    bytes[count_at..count_at + 4].copy_from_slice(&(count + 1).to_le_bytes());
    bytes[next_at..next_at + 4].copy_from_slice(&(next + 1).to_le_bytes());
    Ok(index as u16)
}

/// A block of the given kind (0x80 weapon, 0x90 armor) the game no longer uses: no inventory or
/// storage entry holds its handle and nothing else in the file refers to it (the equipped empty
/// hands and bare body do). Characters keep many (blocks of weapons and armor long gone); reusing
/// one adds a weapon or armor piece without moving anything in the file.
fn free_block(bytes: &[u8], kind: u8) -> Result<usize> {
    let (used, end) = table(bytes)?;
    let mut held = std::collections::HashSet::new();
    let mut lists = Vec::new();
    for location in [Location::Inventory, Location::Storage] {
        let start = section(bytes, location)?;
        lists.push((start, start + capacity(location) * ENTRY));
        for index in 0..capacity(location) {
            let at = start + index * ENTRY;
            if word(bytes, at + 8)? != FREE_ID {
                held.insert(word(bytes, at + 4)?);
            }
        }
    }
    let candidates: Vec<(usize, u32)> = used
        .into_iter()
        .filter(|&(_, k)| k == kind)
        .map(|(at, _)| (at, u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())))
        .filter(|(_, handle)| !held.contains(handle))
        .collect();
    let wanted: std::collections::HashSet<u32> = candidates.iter().map(|&(_, handle)| handle).collect();
    let skip = [(UPGRADES_START, end), lists[0], lists[1]];
    let mut referenced = std::collections::HashSet::new();
    let mut at = 0;
    while at + 4 <= bytes.len() {
        if let Some(&(_, to)) = skip.iter().find(|&&(from, to)| (from..to).contains(&at)) {
            at = to;
            continue;
        }
        let value = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        if wanted.contains(&value) {
            referenced.insert(value);
        }
        at += 1;
    }
    candidates
        .into_iter()
        .find(|(_, handle)| !referenced.contains(handle))
        .map(|(at, _)| at)
        .ok_or_else(|| "This character has no free weapon or armor slot left.".into())
}

/// Adds a weapon (+0, no imprint) or armor piece: a free block of its kind gets its ID, full
/// durability (weapons; armor has none) and five closed gem slots, as the game writes a new one,
/// and the inventory or storage an entry for its handle. Returns the entry's index and ID word.
fn add_equipment(bytes: &mut [u8], location: Location, id: u32, text: &ItemText, params: &GameParams) -> Result<(u16, u32)> {
    if crate::save_editor::copy_count(bytes)? != 1 {
        return Err("This save holds two copies of the character; weapons and armor cannot be added to it yet.".into());
    }
    let (kind, id_word, durability) = if addable_weapon(id, text, params) {
        (0x80, id, params.weapons.get(id, "durabilityMax").unwrap_or(0.0) as u32)
    } else if addable_armor(id, text, params) {
        (0x90, 0x1000_0000 | id, 0)
    } else {
        return Err("Only the game's weapons and armor can be added.".into());
    };
    let block = free_block(bytes, kind)?;
    let handle = word(bytes, block)?;
    let mut fields = vec![id_word, durability, 0, 1];
    for _ in 0..5 {
        fields.extend([SLOT_CLOSED, NO_GEM]);
    }
    for (i, value) in fields.into_iter().enumerate() {
        bytes[block + 4 + i * 4..block + 8 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    let index = add_entry(bytes, location, handle, id_word, 1)?;
    Ok((index, id_word))
}

/// Applies the changes to one character file in a staged save set, then checks the result.
pub fn rewrite(savedata: &Path, edit: &Edit, text: &ItemText, params: &GameParams) -> Result<()> {
    let user = savedata.parent().ok_or("Invalid save staging folder.")?;
    let path = slot_path(user, &edit.title_id, edit.slot)?;
    let mut bytes = slot_bytes(&path)?;
    let original = bytes.clone();
    if format!("{:x}", Sha256::digest(&bytes)) != edit.fingerprint {
        return Err("The character changed since it was opened. Refresh the inventory.".into());
    }
    if edit.changes.is_empty() {
        return Err("No inventory values changed.".into());
    }
    let mut expected = Vec::new();
    let mut expected_effects = Vec::new();
    for change in &edit.changes {
        match *change {
            Change::Amount { location, index, amount } => {
                if usize::from(index) >= capacity(location) {
                    return Err("Invalid inventory entry.".into());
                }
                let at = section(&bytes, location)? + usize::from(index) * ENTRY;
                let (handle, id) = (word(&bytes, at + 4)?, word(&bytes, at + 8)?);
                let Some((Kind::Item, item)) = kind_of(handle, id) else {
                    return Err("Only items have amounts.".into());
                };
                let max = item_max(params, item, location).ok_or("This item is not in the game's tables.")?;
                if !(1..=max).contains(&amount) {
                    return Err(format!("This item's amount must be between 1 and {max}."));
                }
                bytes[at + 12..at + 16].copy_from_slice(&amount.to_le_bytes());
                expected.push((location, index, id, amount));
            }
            Change::Weapon { location, index, level, imprint } => {
                if usize::from(index) >= capacity(location) {
                    return Err("Invalid inventory entry.".into());
                }
                let at = section(&bytes, location)? + usize::from(index) * ENTRY;
                let (handle, id, amount) = (word(&bytes, at + 4)?, word(&bytes, at + 8)?, word(&bytes, at + 12)?);
                if !matches!(kind_of(handle, id), Some((Kind::Weapon, _))) || id == BARE_FISTS {
                    return Err("Only weapons have levels.".into());
                }
                let (base, _, _) = weapon_parts(id);
                let new_id = weapon_id(base, level, imprint);
                // Only IDs the player's own game names: a level or imprint this weapon has.
                if level > 10 || imprint > 2 || !text.weapons.contains_key(&new_id) {
                    return Err("This weapon has no such level or imprint.".into());
                }
                let block = slot_block(&bytes, handle, id)?;
                bytes[at + 8..at + 12].copy_from_slice(&new_id.to_le_bytes());
                bytes[block + 4..block + 8].copy_from_slice(&new_id.to_le_bytes());
                // The slots the game gives the weapon at its new level and imprint (an upgrade
                // opens them; a lower level or another imprint closes or reshapes them).
                let plan = slot_plan(&slot_rows(params), base);
                if !plan.is_empty() {
                    let records = upgrade_records(&bytes)?;
                    for slot in 0..5u8 {
                        let shape = planned_shape(&plan, slot, level, imprint).unwrap_or(SLOT_CLOSED);
                        let (_, gem) = slot_at(&bytes, block, slot)?;
                        if gem != NO_GEM {
                            let fits = records.iter().find(|r| r.1 == gem).is_some_and(|r| shape != SLOT_CLOSED && u32::from(r.3) & shape != 0);
                            if !fits {
                                return Err("Take the gems out of the slots that close or change shape first.".into());
                            }
                        }
                        let field = block + 20 + usize::from(slot) * 8;
                        bytes[field..field + 4].copy_from_slice(&shape.to_le_bytes());
                        expected_effects.push((field, shape));
                    }
                }
                expected.push((location, index, new_id, amount));
            }
            Change::Effect { upgrade, slot, effect } => {
                let records = upgrade_records(&bytes)?;
                let (at, ..) = *records.get(usize::from(upgrade)).ok_or("Unknown gem.")?;
                let field = at + 16 + usize::from(slot) * 4;
                if slot >= 6 {
                    return Err("Unknown gem effect.".into());
                }
                let current = word(&bytes, field)?;
                let category = |id| params.gem_effects.get(id, "categoryId");
                if current == NO_EFFECT || category(current).is_none() || category(current) != category(effect)
                    || !text.gem_effects.contains_key(&effect)
                {
                    return Err("An effect can only change to another of the same kind.".into());
                }
                bytes[field..field + 4].copy_from_slice(&effect.to_le_bytes());
                expected_effects.push((field, effect));
            }
            Change::Equip { upgrade, location, index, slot } => {
                let records = upgrade_records(&bytes)?;
                let &(_, handle, rune, shape, _) = records.get(usize::from(upgrade)).ok_or("Unknown gem.")?;
                if rune {
                    return Err("Runes are not equipped in weapons.".into());
                }
                let held = gem_entry(&bytes, handle)?.ok_or("Only a gem in the inventory can be equipped.")?;
                if usize::from(index) >= capacity(location) || slot >= 5 {
                    return Err("Invalid weapon slot.".into());
                }
                let at = section(&bytes, location)? + usize::from(index) * ENTRY;
                let (weapon_handle, id) = (word(&bytes, at + 4)?, word(&bytes, at + 8)?);
                if !matches!(kind_of(weapon_handle, id), Some((Kind::Weapon, _))) || id == BARE_FISTS {
                    return Err("Gems go in weapons.".into());
                }
                let block = slot_block(&bytes, weapon_handle, id)?;
                let (mut slot_shape, current) = slot_at(&bytes, block, slot)?;
                // A slot the game opens at the weapon's level but the save still has closed (a level
                // changed before the editor opened slots) opens now.
                let (base, level, imprint) = weapon_parts(id);
                if slot_shape == SLOT_CLOSED {
                    if let Some(shape) = planned_shape(&slot_plan(&slot_rows(params), base), slot, level, imprint) {
                        let field = block + 20 + usize::from(slot) * 8;
                        bytes[field..field + 4].copy_from_slice(&shape.to_le_bytes());
                        expected_effects.push((field, shape));
                        slot_shape = shape;
                    }
                }
                // A slot takes the gem shapes of its bits; droplet gems (all bits) fit any open slot.
                if slot_shape == SLOT_CLOSED || u32::from(shape) & slot_shape == 0 {
                    return Err("This slot does not take this gem's shape.".into());
                }
                if current != NO_GEM {
                    return Err("This slot already holds a gem: take it out first.".into());
                }
                let entry = section(&bytes, Location::Inventory)? + usize::from(held) * ENTRY;
                bytes[entry + 4..entry + 8].copy_from_slice(&0u32.to_le_bytes());
                bytes[entry + 8..entry + 12].copy_from_slice(&FREE_ID.to_le_bytes());
                bytes[entry + 12..entry + 16].copy_from_slice(&0u32.to_le_bytes());
                let field = block + 24 + usize::from(slot) * 8;
                bytes[field..field + 4].copy_from_slice(&handle.to_le_bytes());
                expected_effects.push((field, handle));
            }
            Change::Unequip { upgrade } => {
                let records = upgrade_records(&bytes)?;
                let &(record, handle, ..) = records.get(usize::from(upgrade)).ok_or("Unknown gem.")?;
                let (block, slot) = gem_slot(&bytes, handle)?.ok_or("This gem is not in a weapon.")?;
                let field = block + 24 + usize::from(slot) * 8;
                bytes[field..field + 4].copy_from_slice(&NO_GEM.to_le_bytes());
                // The inventory entry of a gem carries the record's second word as its ID.
                let source = word(&bytes, record + 4)?;
                let index = add_entry(&mut bytes, Location::Inventory, handle, source, 1)?;
                expected_effects.push((field, NO_GEM));
                expected.push((Location::Inventory, index, source, 1));
            }
            Change::AddEquipment { location, id } => {
                let (index, id_word) = add_equipment(&mut bytes, location, id, text, params)?;
                expected.push((location, index, id_word, 1));
            }
            // Last, below: a new gem moves every byte after the table.
            Change::AddGem { .. } => {}
            Change::Add { location, id, amount } => {
                let kind = params.goods.get(id, "goodsType").ok_or("This item is not in the game's tables.")? as u32;
                if !GOODS_ADDABLE.iter().any(|(k, _)| *k == kind) || id > 0x00FF_FFFF {
                    return Err("Only consumables, materials and chalices can be added.".into());
                }
                let max = item_max(params, id, location).ok_or("This item is not in the game's tables.")?;
                if !(1..=max).contains(&amount) {
                    return Err(format!("This item's amount must be between 1 and {max}."));
                }
                let index = add_item(&mut bytes, location, id, amount)?;
                expected.push((location, index, 0x4000_0000 | id, amount));
            }
        }
    }
    for change in &edit.changes {
        if let Change::AddGem { shape, effects } = change {
            let (index, source, at) = add_gem(&mut bytes, *shape, effects, text, params)?;
            for (field, _) in expected_effects.iter_mut().filter(|(field, _)| *field >= at) {
                *field += UPGRADE - 8;
            }
            expected.push((Location::Inventory, index, source, 1));
        }
    }
    // The edited file must still be a character whose entries are what was asked.
    let after = entries(&bytes, None, None)?;
    crate::save_editor::parse(&bytes, &edit.title_id, edit.slot)?
        .ok_or("Character validation failed after editing.")?;
    // A field written twice (a gem out of a slot, another into it) keeps the last value.
    let last: std::collections::HashMap<usize, u32> = expected_effects.into_iter().collect();
    for (field, effect) in last {
        if word(&bytes, field)? != effect {
            return Err("Gem validation failed after editing.".into());
        }
    }
    for (location, index, id, amount) in expected {
        let found = after
            .iter()
            .find(|e| e.location == location && e.index == index)
            .ok_or("Inventory validation failed after editing.")?;
        let raw = word(&bytes, section(&bytes, location)? + usize::from(index) * ENTRY + 8)?;
        if raw != id || found.amount != amount {
            return Err("Inventory validation failed after editing.".into());
        }
    }
    crate::save_editor::mirror(&original, &mut bytes)?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}

/// A character edit and an inventory edit of the same character, applied to one staged save set:
/// the character's first (it checks the fingerprint both were opened on), then the inventory's
/// against the rewritten bytes.
pub fn rewrite_save(
    savedata: &Path,
    character: Option<&crate::save_editor::Edit>,
    items: Option<Edit>,
    tables: Option<(&ItemText, &GameParams)>,
) -> Result<()> {
    let mut items = items;
    if let Some(character) = character {
        if let Some(items) = &items {
            if items.fingerprint != character.fingerprint || items.title_id != character.title_id || items.slot != character.slot {
                return Err("The character and inventory edits were opened on different files.".into());
            }
        }
        crate::save_editor::rewrite(savedata, character)?;
        if let Some(items) = items.as_mut() {
            items.fingerprint = crate::save_editor::fingerprint(savedata, &items.title_id, items.slot)?;
        }
    }
    match (items, tables) {
        (Some(items), Some((text, params))) => rewrite(savedata, &items, text, params),
        (Some(_), None) => Err("The game's item tables are needed to edit the inventory.".into()),
        (None, _) if character.is_some() => Ok(()),
        (None, _) => Err("Nothing to change.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const BASE: usize = 0xc000;
    /// The sample's weapon block: in the table, after the one gem record.
    const SAMPLE_BLOCK: usize = UPGRADES_START + UPGRADE;

    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// A synthetic character: the editor's sample, two items, a weapon with its slot block, armor.
    fn sample() -> Vec<u8> {
        let mut bytes = vec![0; 0x140000];
        let inventory = BASE + INVENTORY_FROM_NAME;
        bytes[inventory..inventory + 4].copy_from_slice(&[0x40, 0xf0, 0xff, 0xff]);
        bytes[inventory + 34028..inventory + 34032].copy_from_slice(b"FACE");
        for (i, unit) in "Hunter".encode_utf16().enumerate() {
            bytes[BASE + 1 + i * 2..BASE + 3 + i * 2].copy_from_slice(&unit.to_le_bytes());
        }
        for (relative, value) in [(-103isize, 12u32), (-95, 11), (-79, 22), (-71, 9), (-63, 6), (-55, 7), (-23, 17)] {
            put(&mut bytes, BASE.checked_add_signed(relative).unwrap(), value);
        }
        for (location, start) in [(Location::Inventory, inventory), (Location::Storage, inventory + STORAGE_FROM_INVENTORY)] {
            for index in 0..capacity(location) {
                put(&mut bytes, start + index * ENTRY + 8, FREE_ID);
            }
        }
        let entry = |bytes: &mut Vec<u8>, index: usize, handle: u32, id: u32, amount: u32| {
            let at = inventory + index * ENTRY;
            put(bytes, at + 4, handle);
            put(bytes, at + 8, id);
            put(bytes, at + 12, amount);
        };
        entry(&mut bytes, 0, 0x8080_004e, BARE_FISTS, 1);
        entry(&mut bytes, 1, 0xB000_03E8, 0x4000_03E8, 17); // item 1000
        entry(&mut bytes, 2, 0x8080_0050, 2_000_000, 1); // weapon 2000000 +0
        entry(&mut bytes, 3, 0x9080_0051, 0x1003_8270, 1); // armor 230000
        // A radial gem with one effect, held in the inventory (entry 5).
        put(&mut bytes, UPGRADES_START, 0xC080_0041);
        put(&mut bytes, UPGRADES_START + 4, 0x8001_E460);
        bytes[UPGRADES_START + 8] = 1;
        bytes[UPGRADES_START + 12] = 1;
        put(&mut bytes, UPGRADES_START + 16, 126201);
        for i in 1..6 {
            put(&mut bytes, UPGRADES_START + 16 + i * 4, NO_EFFECT);
        }
        entry(&mut bytes, 5, 0xC080_0041, 0x8001_E460, 1);
        put(&mut bytes, BASE + INVENTORY_COUNTERS.0, 5);
        put(&mut bytes, BASE + INVENTORY_COUNTERS.1, 0x70);
        // The table after the gem: the weapon's block (radial, triangle, then closed slots), a
        // free weapon block and a free armor block nobody refers to, and an empty hand's block the
        // character's equipment refers to; the rest of the table is free entries.
        let blocks = [(SAMPLE_BLOCK, 0x8080_0050, 2_000_000), (SAMPLE_BLOCK + 60, 0x8080_0060, BARE_FISTS),
            (SAMPLE_BLOCK + 120, 0x9080_0061, 0x1000_0000 | NO_ARMOR), (SAMPLE_BLOCK + 180, 0x8080_0062, BARE_FISTS)];
        for (at, handle, id) in blocks {
            put(&mut bytes, at, handle);
            put(&mut bytes, at + 4, id);
            put(&mut bytes, at + 16, 1);
            for slot in 0..5 {
                put(&mut bytes, at + 20 + slot * 8, SLOT_CLOSED);
            }
        }
        put(&mut bytes, SAMPLE_BLOCK + 20, 1);
        put(&mut bytes, SAMPLE_BLOCK + 28, 2);
        put(&mut bytes, BASE + 0x100, 0x8080_0062);
        bytes
    }

    fn params() -> GameParams {
        use crate::game_data::Param;
        GameParams {
            goods: Param::synthetic(&[
                (1000, &[("goodsType", 0.0), ("maxNum", 20.0), ("iconId", 146.0)]),
                (3000, &[("goodsType", 2.0), ("maxNum", 99.0), ("iconId", 83.0)]),
                (4000, &[("goodsType", 1.0), ("maxNum", 1.0), ("iconId", 1.0)]),
            ]),
            weapons: Param::synthetic(&[(2_000_000, &[("attackBasePhysics", 90.0), ("properStrength", 8.0), ("iconId", 7.0), ("durabilityMax", 150.0)])]),
            armor: Param::synthetic(&[(230000, &[("defensePhysics", 20.0), ("iconIdM", 9.0)])]),
            gem_effects: Param::synthetic(&[
                (126201, &[("categoryId", 1.0), ("effectRank", 1.0)]),
                (126207, &[("categoryId", 1.0), ("effectRank", 7.0)]),
                (122401, &[("categoryId", 2.0), ("effectRank", 1.0)]),
            ]),
            weapon_slots: slot_params(),
            // A gem source: row 132000 makes a gem with effect 126201.
            gem_sources: Param::synthetic(&[(132000, &[("equipableSegmentWeapon", 1.0), ("gemeffectGenParamType_0", 1.0), ("gemeffectGenParam_0", 126201.0),
                ("gemeffectGenParamType_1", 0.0), ("gemeffectGenParamType_2", 0.0), ("gemeffectGenParamType_3", 0.0), ("gemeffectGenParamType_4", 0.0),
                ("gemeffectGenParamType_5", 0.0)])]),
        }
    }

    /// The test weapon's slots: radial at +1 and +3, then a triangle at +6 (Uncanny: waning).
    fn slot_params() -> crate::game_data::Param {
        let closed = f64::from(SLOT_CLOSED | 0x100);
        let rows: Vec<(u32, Vec<(&str, f64)>)> = (0..=10u32)
            .flat_map(|level| (0..=1u32).map(move |imprint| (level, imprint)))
            .map(|(level, imprint)| {
                let open = |from: u32, shape: f64| if level >= from { shape } else { closed };
                (1000 + level * 10 + imprint, vec![
                    ("wepParamId", f64::from(2_000_000 + imprint * 10_000 + level * 100)),
                    ("gemSlotType_0", open(1, 1.0)),
                    ("gemSlotType_1", open(3, 1.0)),
                    ("gemSlotType_2", open(6, if imprint == 0 { 2.0 } else { 4.0 })),
                    ("gemSlotType_3", closed),
                    ("gemSlotType_4", closed),
                ])
            })
            .collect();
        let borrowed: Vec<(u32, &[(&str, f64)])> = rows.iter().map(|(id, fields)| (*id, &fields[..])).collect();
        crate::game_data::Param::synthetic(&borrowed)
    }

    fn text() -> ItemText {
        ItemText {
            goods: HashMap::from([(1000, "Vial".to_string())]),
            armor: HashMap::from([(230000, "Hood".to_string())]),
            weapons: (0..=10).map(|l| (2_000_000 + l * 100, "Blade".to_string())).collect(),
            gem_effects: HashMap::from([
                (126201, "Physical +0.7%".to_string()),
                (126207, "Physical +7%".to_string()),
                (122401, "Blood +1.8%".to_string()),
            ]),
            ..Default::default()
        }
    }

    #[test]
    fn adds_weapons_and_armor_into_free_blocks() {
        let bytes = sample();
        let (root, savedata) = staged(bytes);
        let path = savedata.join("1/CUSA03173/SPRJ0005/userdata0000");
        let edit = |changes| Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            changes,
        };
        // Not a weapon or armor piece the game names: refused.
        assert!(rewrite(&savedata, &edit(vec![Change::AddEquipment { location: Location::Inventory, id: 3_000_000 }]), &text(), &params()).is_err());
        rewrite(&savedata, &edit(vec![
            Change::AddEquipment { location: Location::Inventory, id: 2_000_000 },
            Change::AddEquipment { location: Location::Storage, id: 230000 },
        ]), &text(), &params()).unwrap();
        let after = fs::read(&path).unwrap();
        assert_eq!(after.len(), 0x140000);
        // The free weapon block holds the weapon with full durability and closed slots; the free
        // armor block the armor; the equipped empty hand's block is untouched.
        let free = SAMPLE_BLOCK + 60;
        assert_eq!((word(&after, free).unwrap(), word(&after, free + 4).unwrap(), word(&after, free + 8).unwrap()), (0x8080_0060, 2_000_000, 150));
        assert!((0..5).all(|slot| word(&after, free + 20 + slot * 8).unwrap() == SLOT_CLOSED));
        assert_eq!(word(&after, SAMPLE_BLOCK + 124).unwrap(), 0x1000_0000 | 230000);
        assert_eq!(word(&after, SAMPLE_BLOCK + 184).unwrap(), BARE_FISTS);
        let found = entries(&after, Some(&text()), Some(&params())).unwrap();
        assert!(found.iter().any(|e| e.location == Location::Inventory && e.kind == Kind::Weapon && e.id == 2_000_000 && e.index != 2));
        assert!(found.iter().any(|e| e.location == Location::Storage && e.kind == Kind::Armor && e.id == 230000));
        // No free weapon block is left (the empty hand's is in use): refused.
        assert!(rewrite(&savedata, &edit(vec![Change::AddEquipment { location: Location::Inventory, id: 2_000_000 }]), &text(), &params()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_items_weapons_and_armor() {
        let found = entries(&sample(), Some(&text()), Some(&params())).unwrap();
        assert_eq!(found.len(), 4); // the bare fists are not shown; the gem's entry is
        assert_eq!((found[0].kind, found[0].id, found[0].amount), (Kind::Item, 1000, 17));
        assert_eq!(found[0].name.as_deref(), Some("Vial"));
        assert_eq!((found[1].kind, found[1].level, found[1].levels.len()), (Kind::Weapon, 0, 11));
        assert_eq!(found[1].imprints, vec![0]);
        assert_eq!((found[2].kind, found[2].id), (Kind::Armor, 230000));
        assert_eq!((found[0].max, found[0].icon), (20, Some(146)));
        assert_eq!(found[1].stats.as_ref().unwrap()["attack"]["physical"], 90.0);
        assert_eq!(found[1].stats.as_ref().unwrap()["requirements"]["strength"], 8.0);
    }

    fn staged(bytes: Vec<u8>) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("bb-inventory-{}", uuid::Uuid::new_v4()));
        let set = root.join("user/savedata/1/CUSA03173/SPRJ0005");
        fs::create_dir_all(&set).unwrap();
        fs::write(set.join("userdata0000"), bytes).unwrap();
        (root.clone(), root.join("user/savedata"))
    }

    #[test]
    fn edits_amounts_and_weapon_levels_in_both_places() {
        let bytes = sample();
        let (root, savedata) = staged(bytes.clone());
        let edit = Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(&bytes)),
            changes: vec![
                Change::Amount { location: Location::Inventory, index: 1, amount: 20 },
                Change::Weapon { location: Location::Inventory, index: 2, level: 10, imprint: 0 },
            ],
        };
        rewrite(&savedata, &edit, &text(), &params()).unwrap();
        let after = fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap();
        let found = entries(&after, Some(&text()), None).unwrap();
        assert_eq!(found[0].amount, 20);
        assert_eq!((found[1].id, found[1].level), (2_001_000, 10));
        assert_eq!(word(&after, SAMPLE_BLOCK + 4).unwrap(), 2_001_000);
        // A stale preview and an ID the game does not have are refused.
        assert!(rewrite(&savedata, &edit, &text(), &params()).is_err());
        let bad = Edit {
            fingerprint: format!("{:x}", Sha256::digest(&after)),
            changes: vec![Change::Weapon { location: Location::Inventory, index: 2, level: 10, imprint: 1 }],
            ..edit
        };
        assert!(rewrite(&savedata, &bad, &text(), &params()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    /// A copy of a real save and the player's dump (read-only): BB_TEST_SAVE_USER=<user dir>
    /// BB_CHECK_GAME=<dump> cargo test real_inventory -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_inventory() {
        let user = std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER");
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let inventory = read(Path::new(&user), &title, 0, Some(&text), Some(&params)).unwrap();
        for entry in &inventory.entries {
            println!("{:?} {:?} {} {:?} x{} +{}", entry.location, entry.kind, entry.id, entry.name, entry.amount, entry.level);
        }
        for gem in &inventory.upgrades {
            println!("gem {} rune {} shape {} icon {:?} {:?} {:?} {:?}", gem.index, gem.rune, gem.shape, gem.icon, gem.location, gem.weapon,
                gem.effects.iter().map(|e| e.text.clone()).collect::<Vec<_>>());
        }
        assert!(inventory.entries.iter().all(|e| e.kind == Kind::Upgrade || e.name.is_some()));
    }

    #[test]
    fn adds_items_with_counters_and_limits() {
        let bytes = sample();
        let (root, savedata) = staged(bytes.clone());
        let add = |amount, id| Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(&bytes)),
            changes: vec![Change::Add { location: Location::Inventory, id, amount }],
        };
        // Over the stack limit, a key item and an item already held are refused.
        assert!(rewrite(&savedata, &add(100, 3000), &text(), &params()).is_err());
        assert!(rewrite(&savedata, &add(1, 4000), &text(), &params()).is_err());
        assert!(rewrite(&savedata, &add(1, 1000), &text(), &params()).is_err());
        rewrite(&savedata, &add(99, 3000), &text(), &params()).unwrap();
        let after = fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap();
        let added = entries(&after, None, None).unwrap().into_iter().find(|e| e.id == 3000).unwrap();
        assert_eq!((added.index, added.amount), (4, 99));
        assert_eq!(after[BASE + INVENTORY_FROM_NAME + 4 * ENTRY], 0x70);
        assert_eq!(word(&after, BASE + INVENTORY_COUNTERS.0).unwrap(), 6);
        assert_eq!(word(&after, BASE + INVENTORY_COUNTERS.1).unwrap(), 0x71);
        fs::remove_dir_all(root).unwrap();
    }

    /// Adds to a scratch copy of a real save (never the player's own): BB_TEST_SAVE_USER=<copy>
    /// BB_CHECK_GAME=<dump> cargo test real_add_items -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_add_items() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let held = |id: u32, location: Location| before.entries.iter().any(|e| e.id == id && e.location == location);
        let mut changes = Vec::new();
        if !held(3020, Location::Inventory) {
            changes.push(Change::Add { location: Location::Inventory, id: 3020, amount: 5 });
        }
        if !held(3010, Location::Storage) {
            changes.push(Change::Add { location: Location::Storage, id: 3010, amount: 30 });
        }
        let edit = Edit { title_id: title.clone(), slot: 0, fingerprint: before.fingerprint.clone(), changes };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
        for entry in read(&user, &title, 0, Some(&text), Some(&params)).unwrap().entries {
            if [3020, 3010].contains(&entry.id) {
                println!("{:?} {} {:?} x{}", entry.location, entry.index, entry.name, entry.amount);
            }
        }
    }

    /// A weapon level and an added item on a scratch copy (never the player's own):
    /// BB_TEST_SAVE_USER=<copy> BB_CHECK_GAME=<dump> cargo test real_weapon_and_item -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_weapon_and_item() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let weapon = before.entries.iter().find(|e| e.kind == Kind::Weapon && e.levels.contains(&5) && e.level != 5).unwrap();
        println!("weapon {:?} +{} -> +5", weapon.name, weapon.level);
        let edit = Edit {
            title_id: title.clone(),
            slot: 0,
            fingerprint: before.fingerprint.clone(),
            changes: vec![
                Change::Weapon { location: weapon.location, index: weapon.index, level: 5, imprint: weapon.imprint },
                Change::Add { location: Location::Inventory, id: 3030, amount: 1 },
            ],
        };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
    }

    #[test]
    fn gems_read_and_change_within_their_category() {
        let bytes = sample();
        let found = entries(&bytes, None, None).unwrap();
        let gems = upgrades(&bytes, &found, Some(&text()), Some(&params())).unwrap();
        assert_eq!(gems.len(), 1);
        assert_eq!((gems[0].shape, gems[0].location), (1, Some(Location::Inventory)));
        let options: Vec<u32> = gems[0].effects[0].options.iter().map(|o| o.id).collect();
        assert_eq!(options, vec![126201, 126207]);
        let (root, savedata) = staged(bytes.clone());
        let change = |effect| Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(&bytes)),
            changes: vec![Change::Effect { upgrade: 0, slot: 0, effect }],
        };
        // Another kind of effect, and an empty effect slot, are refused.
        assert!(rewrite(&savedata, &change(122401), &text(), &params()).is_err());
        let empty = Edit { changes: vec![Change::Effect { upgrade: 0, slot: 1, effect: 126207 }], ..change(126207) };
        assert!(rewrite(&savedata, &empty, &text(), &params()).is_err());
        rewrite(&savedata, &change(126207), &text(), &params()).unwrap();
        let after = fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap();
        assert_eq!(word(&after, UPGRADES_START + 16).unwrap(), 126207);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gems_equip_into_fitting_slots_and_back() {
        let bytes = sample();
        let block = SAMPLE_BLOCK;
        let (root, savedata) = staged(bytes.clone());
        let path = savedata.join("1/CUSA03173/SPRJ0005/userdata0000");
        let edit = |changes| Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            changes,
        };
        let equip = |slot| Change::Equip { upgrade: 0, location: Location::Inventory, index: 2, slot };
        // A radial gem does not fit the triangle slot, nor a closed one.
        assert!(rewrite(&savedata, &edit(vec![equip(1)]), &text(), &params()).is_err());
        assert!(rewrite(&savedata, &edit(vec![equip(2)]), &text(), &params()).is_err());
        rewrite(&savedata, &edit(vec![equip(0)]), &text(), &params()).unwrap();
        let after = fs::read(&path).unwrap();
        assert_eq!(word(&after, block + 24).unwrap(), 0xC080_0041);
        let inventory = BASE + INVENTORY_FROM_NAME;
        assert_eq!(word(&after, inventory + 5 * ENTRY + 8).unwrap(), FREE_ID);
        let read_back = read(&root.join("user"), "CUSA03173", 0, Some(&text()), Some(&params())).unwrap();
        let weapon = read_back.entries.iter().find(|e| e.kind == Kind::Weapon).unwrap();
        assert_eq!((weapon.slots.len(), weapon.slots[0].gem), (2, Some(0)));
        assert!(read_back.upgrades[0].equipped && read_back.upgrades[0].location.is_none());
        // Equipped, it is no longer in the inventory to equip again; taken out, it is back.
        assert!(rewrite(&savedata, &edit(vec![equip(0)]), &text(), &params()).is_err());
        rewrite(&savedata, &edit(vec![Change::Unequip { upgrade: 0 }]), &text(), &params()).unwrap();
        let after = fs::read(&path).unwrap();
        assert_eq!(word(&after, block + 24).unwrap(), NO_GEM);
        let held = gem_entry(&after, 0xC080_0041).unwrap().unwrap();
        assert_eq!(word(&after, inventory + usize::from(held) * ENTRY + 8).unwrap(), 0x8001_E460);
        assert_eq!(word(&after, BASE + INVENTORY_COUNTERS.0).unwrap(), 6);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn levels_open_close_and_reshape_gem_slots() {
        let (root, savedata) = staged(sample());
        let path = savedata.join("1/CUSA03173/SPRJ0005/userdata0000");
        let mut text = text();
        text.weapons.extend((0..=10).map(|l| (2_010_000 + l * 100, "Uncanny Blade".to_string())));
        let apply = |changes| rewrite(&savedata, &Edit {
            title_id: "CUSA03173".into(),
            slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            changes,
        }, &text, &params());
        let level = |level, imprint| Change::Weapon { location: Location::Inventory, index: 2, level, imprint };
        let shapes = || -> Vec<u32> {
            let bytes = fs::read(&path).unwrap();
            (0..5).map(|slot| slot_at(&bytes, SAMPLE_BLOCK, slot).unwrap().0).collect()
        };
        // +6: the game's three slots (radial, radial, triangle).
        apply(vec![level(6, 0)]).unwrap();
        assert_eq!(shapes(), [1, 1, 2, SLOT_CLOSED, SLOT_CLOSED]);
        let inventory = read(&root.join("user"), "CUSA03173", 0, Some(&text), Some(&params())).unwrap();
        let weapon = inventory.entries.iter().find(|e| e.index == 2).unwrap();
        assert_eq!(weapon.slot_plan.iter().filter(|p| p.imprint == 1).map(|p| (p.slot, p.opens, p.shape)).collect::<Vec<_>>(),
            [(0, 1, 1), (1, 3, 1), (2, 6, 4)]);
        // A radial gem in the second slot; Uncanny reshapes the third slot and keeps the gem.
        apply(vec![Change::Equip { upgrade: 0, location: Location::Inventory, index: 2, slot: 1 }]).unwrap();
        apply(vec![level(6, 1)]).unwrap();
        assert_eq!(shapes(), [1, 1, 4, SLOT_CLOSED, SLOT_CLOSED]);
        // +2 closes the second slot: refused while the gem is in it, done once it is taken out.
        assert!(apply(vec![level(2, 1)]).is_err());
        apply(vec![Change::Unequip { upgrade: 0 }, level(2, 1)]).unwrap();
        assert_eq!(shapes(), [1, SLOT_CLOSED, SLOT_CLOSED, SLOT_CLOSED, SLOT_CLOSED]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equipping_opens_a_slot_the_level_already_has() {
        // A weapon at +6 whose second slot the save still has closed (a level changed by an older
        // editor): equipping there opens it as the game would.
        let mut bytes = sample();
        let at = BASE + INVENTORY_FROM_NAME + 2 * ENTRY;
        bytes[at + 8..at + 12].copy_from_slice(&2_000_600u32.to_le_bytes());
        bytes[SAMPLE_BLOCK + 4..SAMPLE_BLOCK + 8].copy_from_slice(&2_000_600u32.to_le_bytes());
        bytes[SAMPLE_BLOCK + 28..SAMPLE_BLOCK + 32].copy_from_slice(&SLOT_CLOSED.to_le_bytes());
        let fingerprint = format!("{:x}", Sha256::digest(&bytes));
        let (root, savedata) = staged(bytes);
        let edit = Edit {
            title_id: "CUSA03173".into(), slot: 0, fingerprint,
            changes: vec![Change::Equip { upgrade: 0, location: Location::Inventory, index: 2, slot: 1 }],
        };
        rewrite(&savedata, &edit, &text(), &params()).unwrap();
        let after = fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap();
        assert_eq!(slot_at(&after, SAMPLE_BLOCK, 1).unwrap(), (1, 0xC080_0041));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn adds_a_gem_and_moves_what_follows_the_table() {
        let bytes = sample();
        let (root, savedata) = staged(bytes.clone());
        let path = savedata.join("1/CUSA03173/SPRJ0005/userdata0000");
        let base = character_base(&bytes).unwrap().unwrap();
        let edit = |changes| Edit {
            title_id: "CUSA03173".into(), slot: 0,
            fingerprint: format!("{:x}", Sha256::digest(fs::read(&path).unwrap())), changes,
        };
        // An effect the game has no gem of, and a shape it does not have, are refused.
        assert!(rewrite(&savedata, &edit(vec![Change::AddGem { shape: 1, effects: vec![122401] }]), &text(), &params()).is_err());
        assert!(rewrite(&savedata, &edit(vec![Change::AddGem { shape: 3, effects: vec![126201] }]), &text(), &params()).is_err());
        // A new triangle gem, with the existing gem's effect changed in the same edit.
        rewrite(&savedata, &edit(vec![
            Change::AddGem { shape: 2, effects: vec![126201, 122401] },
            Change::Effect { upgrade: 0, slot: 0, effect: 126207 },
        ]), &text(), &params()).unwrap();
        let after = fs::read(&path).unwrap();
        assert_eq!(after.len(), bytes.len());
        assert_eq!(character_base(&after).unwrap(), Some(base + 32));
        let inventory = read(&root.join("user"), "CUSA03173", 0, Some(&text()), Some(&params())).unwrap();
        assert_eq!(inventory.upgrades.len(), 2);
        let new = inventory.upgrades.iter().find(|g| g.shape == 2).unwrap();
        assert_eq!(new.effects.iter().map(|e| e.id).collect::<Vec<_>>(), [126201, 122401]);
        assert_eq!(new.location, Some(Location::Inventory));
        assert!(inventory.upgrades.iter().any(|g| g.effects[0].id == 126207));
        // The character's own values moved with it.
        let character = crate::save_editor::parse(&after, "CUSA03173", 0).unwrap().unwrap();
        assert_eq!(character.echoes, crate::save_editor::parse(&bytes, "CUSA03173", 0).unwrap().unwrap().echoes);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gem_icons_follow_shape_and_effect() {
        use crate::game_data::Param;
        let mut params = params();
        params.gem_effects = Param::synthetic(&[(1, &[("gemIconIdOffset", 13.0)]), (2, &[("gemIconIdOffset", 1203.0)]), (3, &[("gemIconIdOffset", 247.0)])]);
        assert_eq!(gem_icon(&params, false, 1, 1), Some(3013));
        assert_eq!(gem_icon(&params, false, 4, 1), Some(3413));
        assert_eq!(gem_icon(&params, false, 63, 1), Some(4013));
        assert_eq!(gem_icon(&params, true, 1, 2), Some(4203));
        // Outside the sets: no icon rather than another's.
        assert_eq!(gem_icon(&params, false, 1, 3), None);
        assert_eq!(gem_icon(&params, true, 1, 1), None);
    }

        #[test]
    fn character_and_inventory_in_one_edit() {
        let bytes = sample();
        let fingerprint = format!("{:x}", Sha256::digest(&bytes));
        let (root, savedata) = staged(bytes.clone());
        let character = crate::save_editor::Edit {
            title_id: "CUSA03173".into(), slot: 0, fingerprint: fingerprint.clone(),
            echoes: 5000, insight: 3, vitality: 12, endurance: 11, strength: 22, skill: 9, bloodtinge: 6, arcane: 7,
        };
        let items = |fingerprint: &str| Edit {
            title_id: "CUSA03173".into(), slot: 0, fingerprint: fingerprint.into(),
            changes: vec![Change::Amount { location: Location::Inventory, index: 1, amount: 9 }],
        };
        // Opened on other bytes: refused, nothing written.
        assert!(rewrite_save(&savedata, Some(&character), Some(items("other")), Some((&text(), &params()))).is_err());
        assert_eq!(fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap(), bytes);
        rewrite_save(&savedata, Some(&character), Some(items(&fingerprint)), Some((&text(), &params()))).unwrap();
        let after = fs::read(savedata.join("1/CUSA03173/SPRJ0005/userdata0000")).unwrap();
        let parsed = crate::save_editor::parse(&after, "CUSA03173", 0).unwrap().unwrap();
        assert_eq!((parsed.echoes, parsed.insight), (5000, 3));
        assert_eq!(word(&after, BASE + INVENTORY_FROM_NAME + ENTRY + 12).unwrap(), 9);
        fs::remove_dir_all(root).unwrap();
    }

    /// A weapon into the inventory and an armor piece into storage, from the catalog, on a scratch
    /// copy (never the player's own):
    /// BB_TEST_SAVE_USER=<copy> BB_CHECK_GAME=<dump> cargo test real_add_equipment -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_add_equipment() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let owned: std::collections::HashSet<u32> = before.entries.iter().map(|e| weapon_parts(e.id).0).collect();
        let items = catalog(&text, &params);
        let weapon = items.iter().find(|i| i.category == "weapon" && !owned.contains(&i.id)).unwrap();
        let armor = items.iter().find(|i| i.category == "armor" && !before.entries.iter().any(|e| e.id == i.id)).unwrap();
        println!("catalog: {} weapons, {} armor; first armor {:?}", items.iter().filter(|i| i.category == "weapon").count(),
            items.iter().filter(|i| i.category == "armor").count(), items.iter().filter(|i| i.category == "armor").take(4).map(|i| (i.id, &i.name)).collect::<Vec<_>>());
        println!("adding {} ({}) to the inventory and {} ({}) to storage", weapon.name, weapon.id, armor.name, armor.id);
        let edit = Edit {
            title_id: title.clone(),
            slot: 0,
            fingerprint: before.fingerprint.clone(),
            changes: vec![
                Change::AddEquipment { location: Location::Inventory, id: weapon.id },
                Change::AddEquipment { location: Location::Storage, id: armor.id },
            ],
        };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
        let after = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        for e in after.entries.iter().filter(|e| e.id == weapon.id || e.id == armor.id) {
            println!("after: {:?} {:?} {:?} slots {:?}", e.location, e.kind, e.name, e.slots);
        }
    }

    /// A new gem and a weapon raised to +6 (its slots open), on a scratch copy (never the player's
    /// own), for a game run that saves them back:
    /// BB_TEST_SAVE_USER=<copy> BB_CHECK_GAME=<dump> cargo test real_add_gem -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_add_gem() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let items = catalog(&text, &params);
        let gem = items.iter().find(|i| i.category == "gem" && i.name.contains("Physical")).unwrap();
        let weapon = before.entries.iter().find(|e| e.kind == Kind::Weapon && e.level < 6 && !e.slot_plan.is_empty()).unwrap();
        println!("{} gem effects; adding {} ({}); {:?} +{} -> +6, slots {:?}", items.iter().filter(|i| i.category == "gem").count(),
            gem.name, gem.id, weapon.name, weapon.level, weapon.slots);
        let edit = Edit {
            title_id: title.clone(),
            slot: 0,
            fingerprint: before.fingerprint.clone(),
            changes: vec![
                Change::Weapon { location: weapon.location, index: weapon.index, level: 6, imprint: weapon.imprint },
                Change::AddGem { shape: 1, effects: vec![gem.id] },
            ],
        };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
        let after = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let raised = after.entries.iter().find(|e| e.location == weapon.location && e.index == weapon.index).unwrap();
        println!("after: {:?} slots {:?}", raised.name, raised.slots);
        for g in &after.upgrades {
            println!("gem {} shape {} {:?} {:?}", g.index, g.shape, g.location, g.effects.iter().map(|e| e.text.clone()).collect::<Vec<_>>());
        }
    }

    /// A gem taken out of its weapon, and an inventory gem put in another weapon's fitting slot,
    /// on a scratch copy (never the player's own):
    /// BB_TEST_SAVE_USER=<copy> BB_CHECK_GAME=<dump> cargo test real_gem_equip -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_gem_equip() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let mut changes = Vec::new();
        let out = before.upgrades.iter().find(|g| !g.rune && g.weapon.is_some()).map(|g| g.index);
        if let Some(gem) = out {
            println!("unequip gem {gem}");
            changes.push(Change::Unequip { upgrade: gem });
        }
        let gem = before.upgrades.iter().find(|g| !g.rune && g.location == Some(Location::Inventory)).unwrap();
        let (weapon, slot) = before.entries.iter().filter(|e| e.location == Location::Inventory)
            .flat_map(|e| e.slots.iter().map(move |s| (e, s)))
            .find(|(_, s)| (s.gem.is_none() || s.gem == out) && u32::from(gem.shape) & s.shape != 0)
            .unwrap();
        println!("equip gem {} (shape {}) into {:?} slot {}", gem.index, gem.shape, weapon.name, slot.slot);
        changes.push(Change::Equip { upgrade: gem.index, location: weapon.location, index: weapon.index, slot: slot.slot });
        let edit = Edit { title_id: title.clone(), slot: 0, fingerprint: before.fingerprint.clone(), changes };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
        for g in read(&user, &title, 0, Some(&text), Some(&params)).unwrap().upgrades.iter().filter(|g| !g.rune) {
            println!("after: gem {} {:?} {:?}", g.index, g.location, g.weapon);
        }
    }

    /// A gem effect raised to its strongest kind on a scratch copy (never the player's own):
    /// BB_TEST_SAVE_USER=<copy> BB_CHECK_GAME=<dump> cargo test real_gem_effect -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_gem_effect() {
        let user = std::path::PathBuf::from(std::env::var("BB_TEST_SAVE_USER").expect("BB_TEST_SAVE_USER"));
        let game = std::env::var("BB_CHECK_GAME").expect("BB_CHECK_GAME");
        let text = crate::game_data::item_text(Path::new(&game), "en").unwrap();
        let params = crate::game_data::game_params(Path::new(&game)).unwrap();
        let title = std::env::var("BB_TEST_SAVE_TITLE").unwrap_or_else(|_| "CUSA00207".into());
        let before = read(&user, &title, 0, Some(&text), Some(&params)).unwrap();
        let gem = before.upgrades.iter().find(|g| !g.rune && g.location == Some(Location::Inventory) && g.effects[0].options.len() > 1).unwrap();
        let best = gem.effects[0].options.last().unwrap();
        println!("gem {}: {:?} -> {}", gem.index, gem.effects[0].text, best.text);
        let edit = Edit {
            title_id: title.clone(),
            slot: 0,
            fingerprint: before.fingerprint.clone(),
            changes: vec![Change::Effect { upgrade: gem.index, slot: 0, effect: best.id }],
        };
        rewrite(&user.join("savedata"), &edit, &text, &params).unwrap();
    }
}

