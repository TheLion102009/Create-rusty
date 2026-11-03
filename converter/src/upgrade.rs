//! 1.21.1 -> 26.3 data upgrades (recipes, loot tables, renamed ids).
//!
//! Taken unchanged from Horizon's `horizon-port-pack` (crates/horizon-port-pack/src/carry.rs,
//! commit f42ef42ef6ceccb3c6a20bea2481d87949a51c81), MIT licence, Copyright (c) Ludovic Marie
//! and the Horizon contributors. Kept in sync by hand: when Horizon's upgrader learns a new
//! rename, copy it here.
#![allow(dead_code)]
use serde_json::{Value, json};

/// A 1.21.1 ingredient (`{"item": id}`, `{"tag": t}` or a list of those) as
/// 26.3's (`"id"`, `"#t"`, `["id", ...]`); `None` for what has no 26.3 form
/// (custom ingredient types, tags inside lists).
pub fn ingredient(v: &Value) -> Option<Value> {
    match v {
        Value::String(_) => Some(v.clone()),
        Value::Object(o) if o.len() == 1 => {
            if let Some(Value::String(id)) = o.get("item") {
                Some(json!(id))
            } else if let Some(Value::String(tag)) = o.get("tag") {
                Some(json!(format!("#{tag}")))
            } else {
                None
            }
        }
        Value::Array(list) => {
            let ids: Option<Vec<Value>> = list
                .iter()
                .map(|x| {
                    x.get("item")
                        .filter(|_| x.as_object().is_some_and(|o| o.len() == 1))
                        .cloned()
                })
                .collect();
            match ids? {
                ids if ids.len() == 1 => ids.into_iter().next(),
                ids => Some(Value::Array(ids)),
            }
        }
        _ => None,
    }
}

/// A vanilla recipe upgraded in place; `false` (unchanged) when it is not
/// a vanilla type or one of its ingredients has no 26.3 form.
pub fn upgrade_recipe(recipe: &mut Value) -> bool {
    let kind = recipe["type"].as_str().unwrap_or("");
    if !kind.starts_with("minecraft:") || recipe.get("neoforge:conditions").is_some() {
        return false;
    }
    let mut out = recipe.clone();
    if let Some(key) = out.get_mut("key").and_then(Value::as_object_mut) {
        for v in key.values_mut() {
            let Some(new) = ingredient(v) else {
                return false;
            };
            *v = new;
        }
    }
    if let Some(list) = out.get_mut("ingredients").and_then(Value::as_array_mut) {
        for v in list.iter_mut() {
            let Some(new) = ingredient(v) else {
                return false;
            };
            *v = new;
        }
    }
    for field in ["ingredient", "base", "addition", "template"] {
        if let Some(v) = out.get_mut(field) {
            let Some(new) = ingredient(v) else {
                return false;
            };
            *v = new;
        }
    }
    *recipe = out;
    true
}

/// Block and item ids vanilla renamed after 1.21.1, as its datafixers
/// rename them in worlds (DataVersion 4541: the chain became the iron chain
/// when copper chains came in). A data file naming the old id fails to load
/// in 26.3: a recipe, a loot entry, a processor's block match.
const RENAMED_IDS: [(&str, &str); 1] = [("minecraft:chain", "minecraft:iron_chain")];

/// Every string of a data file that is a renamed id ([`RENAMED_IDS`]) set to
/// the new one, in place; whether any was.
pub fn renamed_ids(v: &mut Value) -> bool {
    match v {
        Value::String(s) => match RENAMED_IDS.iter().find(|(old, _)| s == old) {
            Some((_, new)) => {
                *s = (*new).to_owned();
                true
            }
            None => false,
        },
        Value::Array(a) => a.iter_mut().fold(false, |c, x| renamed_ids(x) | c),
        Value::Object(o) => o.values_mut().fold(false, |c, x| renamed_ids(x) | c),
        _ => false,
    }
}

/// A 1.21.1 advancement in 26.3's shape, in place: an `enter_block`
/// criterion names its block as `blocks`, which 26.3 reads as a block list.
/// 26.3 skips the old `block` field, so the criterion matched any block the
/// player entered (Regions Unexplored's "Downer" on the first dive).
pub fn upgrade_advancement(v: &mut Value) -> bool {
    let mut changed = false;
    let criteria = v.get_mut("criteria").and_then(Value::as_object_mut);
    for c in criteria.into_iter().flat_map(|o| o.values_mut()) {
        let trigger = c.get("trigger").and_then(Value::as_str);
        if !matches!(trigger, Some("minecraft:enter_block" | "enter_block")) {
            continue;
        }
        if let Some(conditions) = c.get_mut("conditions").and_then(Value::as_object_mut)
            && !conditions.contains_key("blocks")
            && let Some(block) = conditions.remove("block")
        {
            conditions.insert("blocks".into(), block);
            changed = true;
        }
    }
    changed
}

/// A 1.21.1 loot table in 26.3's shape, in place: `conditions` lists become
/// one `condition` (`all_of` for several), `functions` become `modifier`,
/// and conditions and functions name their kind in `type`. A tool match on
/// silk touch alone, or on shears alone, becomes 26.3's
/// `minecraft:tool/can_silk_touch` or `minecraft:tool/can_shear`. Number
/// providers written as a bare `{min, max}` get their `type`
/// ([`type_number`]).
pub fn upgrade_loot(v: &mut Value) {
    match v {
        Value::Array(a) => a.iter_mut().for_each(upgrade_loot),
        Value::Object(o) => {
            if let Some(Value::Array(pools)) = o.get_mut("pools") {
                for pool in pools {
                    for key in ["rolls", "bonus_rolls"] {
                        if let Some(n) = pool.get_mut(key) {
                            type_number(n);
                        }
                    }
                }
            }
            if let Some(Value::Array(mut list)) = o.remove("conditions") {
                list.iter_mut().for_each(upgrade_condition);
                let one = match <[Value; 1]>::try_from(list) {
                    Ok([c]) => c,
                    Err(list) => json!({"type": "minecraft:all_of", "terms": list}),
                };
                o.insert("condition".to_owned(), one);
            }
            if let Some(Value::Array(mut list)) = o.remove("functions") {
                for f in &mut list {
                    rename_kind(f, "function");
                    type_function_numbers(f);
                    upgrade_loot(f);
                }
                o.insert("modifier".to_owned(), Value::Array(list));
            }
            for (k, x) in o.iter_mut() {
                if k != "condition" && k != "modifier" {
                    upgrade_loot(x);
                }
            }
        }
        _ => {}
    }
}

/// A number provider in 26.3's shape, in place. 1.21.1 read a `{min, max}`
/// with no `type` as a uniform one (NumberProviders.CODEC's fallback to
/// UniformGenerator); 26.3 dispatches on `type` alone and refuses the table.
/// Bounds nested in a uniform or binomial one get theirs too.
fn type_number(v: &mut Value) {
    let Some(o) = v.as_object_mut() else {
        return;
    };
    if !o.contains_key("type") && o.contains_key("min") && o.contains_key("max") {
        o.insert("type".to_owned(), json!("minecraft:uniform"));
    }
    let kind = o.get("type").and_then(Value::as_str).unwrap_or_default();
    let bounds: &[&str] = match kind.strip_prefix("minecraft:").unwrap_or(kind) {
        "uniform" => &["min", "max"],
        "binomial" => &["n", "p"],
        _ => &[],
    };
    for key in bounds {
        if let Some(b) = o.get_mut(*key) {
            type_number(b);
        }
    }
}

/// The number providers of one loot function ([`type_number`]): the fields
/// its 1.21.1 codec reads with NumberProviders.CODEC. Range bounds such as
/// `limit_count`'s are providers too, the range itself is not.
fn type_function_numbers(f: &mut Value) {
    let kind = f["type"].as_str().unwrap_or_default().to_owned();
    let Some(f) = f.as_object_mut() else {
        return;
    };
    let mut each = |list: &str, key: &str| {
        for x in f
            .get_mut(list)
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            if let Some(n) = x.get_mut(key) {
                type_number(n);
            }
        }
    };
    match kind.strip_prefix("minecraft:").unwrap_or(&kind) {
        "set_stew_effect" => each("effects", "duration"),
        "set_attributes" => each("modifiers", "amount"),
        "set_enchantments" => {
            for level in f
                .get_mut("enchantments")
                .and_then(Value::as_object_mut)
                .into_iter()
                .flatten()
            {
                type_number(level.1);
            }
        }
        "limit_count" => {
            if let Some(limit) = f.get_mut("limit") {
                for key in ["min", "max"] {
                    if let Some(n) = limit.get_mut(key) {
                        type_number(n);
                    }
                }
            }
        }
        k => {
            let key = match k {
                "set_count" | "enchanted_count_increase" => "count",
                "set_damage" => "damage",
                "enchant_with_levels" => "levels",
                "set_ominous_bottle_amplifier" => "amplifier",
                _ => return,
            };
            if let Some(n) = f.get_mut(key) {
                type_number(n);
            }
        }
    }
}

/// `{"<key>": kind, ...}` as `{"type": kind, ...}`.
fn rename_kind(v: &mut Value, key: &str) {
    if let Some(o) = v.as_object_mut()
        && !o.contains_key("type")
        && let Some(kind) = o.remove(key)
    {
        o.insert("type".to_owned(), kind);
    }
}

/// One 1.21.1 loot condition in 26.3's shape.
fn upgrade_condition(c: &mut Value) {
    rename_kind(c, "condition");
    if let Some(term) = c.get_mut("term") {
        upgrade_condition(term);
    }
    if let Some(Value::Array(terms)) = c.get_mut("terms") {
        terms.iter_mut().for_each(upgrade_condition);
    }
    if c["type"] == "minecraft:match_tool"
        && let Some(named) = named_tool_condition(&c["predicate"])
    {
        *c = json!(named);
    }
    match c["type"].as_str() {
        Some("minecraft:entity_properties") => {
            if let Some(p) = c.get_mut("predicate") {
                upgrade_entity_predicate(p);
            }
        }
        Some("minecraft:damage_source_properties") => {
            for key in ["direct_entity", "source_entity"] {
                if let Some(p) = c["predicate"].get_mut(key) {
                    upgrade_entity_predicate(p);
                }
            }
        }
        _ => {}
    }
}

/// The fields of a 1.21 entity predicate that are, in 26.3, sub-predicates
/// keyed by their registry name (`minecraft:<field>`), as 26.3's own data
/// writes them (`"minecraft:flags": {"is_on_fire": true}`).
const ENTITY_FIELDS: [&str; 15] = [
    "distance",
    "movement",
    "location",
    "stepping_on",
    "movement_affected_by",
    "effects",
    "nbt",
    "flags",
    "equipment",
    "periodic_tick",
    "team",
    "slots",
    "components",
    "predicates",
    "type_specific",
];

/// Entity predicates nested in an entity predicate (1.21 field names).
const ENTITY_NESTED: [&str; 3] = ["vehicle", "passenger", "targeted_entity"];

/// Kinds of 1.21's `type_specific` whose `variant` 26.3 matches as a component
/// (`"minecraft:components": {"minecraft:cat/variant": ...}`).
const VARIANT_KINDS: [&str; 14] = [
    "axolotl",
    "cat",
    "chicken",
    "cow",
    "fox",
    "frog",
    "horse",
    "llama",
    "mooshroom",
    "painting",
    "parrot",
    "pig",
    "rabbit",
    "wolf",
];

/// A 1.21 entity predicate in 26.3's shape, in place: `type` is
/// `minecraft:entity_type`, every other field `minecraft:<field>`, `vehicle`,
/// `passenger` and `targeted_entity` recursively; `type_specific {"type": k,
/// ...}` is `minecraft:type_specific/<k>` (`slime` is `cube_mob`), a variant
/// kind's `variant` and a sheep's `color` go to `minecraft:components`. The
/// game's loot code reads only these keys, so an old-shaped predicate never
/// matched. A predicate already in 26.3's shape stays as it is.
pub(crate) fn upgrade_entity_predicate(p: &mut Value) {
    let Some(o) = p.as_object_mut() else {
        return;
    };
    let key = |f: &str| format!("minecraft:{f}");
    if let Some(t) = o.remove("type") {
        o.insert(key("entity_type"), t);
    }
    for f in ENTITY_NESTED {
        if let Some(mut inner) = o.remove(f) {
            upgrade_entity_predicate(&mut inner);
            o.insert(key(f), inner);
        }
    }
    for f in ENTITY_FIELDS {
        if let Some(v) = o.remove(f) {
            o.insert(key(f), v);
        }
    }
    let Some(Value::Object(mut ts)) = o.remove("minecraft:type_specific") else {
        return;
    };
    let kind = ts.remove("type");
    let kind = kind.as_ref().and_then(Value::as_str).unwrap_or("any");
    let kind = match kind.trim_start_matches("minecraft:") {
        "slime" => "cube_mob",
        k => k,
    };
    let component = match kind {
        k if VARIANT_KINDS.contains(&k) => {
            ts.remove("variant").map(|v| (format!("{k}/variant"), v))
        }
        "sheep" => ts.remove("color").map(|v| ("sheep/color".to_owned(), v)),
        _ => None,
    };
    if let Some((name, v)) = component {
        let components = o.entry(key("components")).or_insert_with(|| json!({}));
        if let Some(c) = components.as_object_mut() {
            c.insert(key(&name), v);
        }
    }
    // A player's `looking_at`, a lightning bolt's `entity_struck`.
    for f in ["looking_at", "entity_struck"] {
        if let Some(v) = ts.get_mut(f) {
            upgrade_entity_predicate(v);
        }
    }
    // A variant kind with nothing else to match is the component alone.
    if !ts.is_empty() || !(kind == "any" || VARIANT_KINDS.contains(&kind)) {
        o.insert(key(&format!("type_specific/{kind}")), Value::Object(ts));
    }
}

/// 26.3's named tool conditions for the 1.21.1 predicates they replace.
fn named_tool_condition(predicate: &Value) -> Option<&'static str> {
    let keys: Vec<&str> = predicate.as_object()?.keys().map(String::as_str).collect();
    match keys.as_slice() {
        ["predicates"] => {
            let list = predicate["predicates"]["minecraft:enchantments"].as_array()?;
            let [e] = list.as_slice() else {
                return None;
            };
            let silk = e["enchantments"] == "minecraft:silk_touch"
                && e["levels"]["min"].as_u64().unwrap_or(1) <= 1
                && e["levels"].get("max").is_none()
                && predicate["predicates"].as_object()?.len() == 1;
            silk.then_some("minecraft:tool/can_silk_touch")
        }
        ["items"] => {
            let items = &predicate["items"];
            let shears = items == "minecraft:shears" || *items == json!(["minecraft:shears"]);
            shears.then_some("minecraft:tool/can_shear")
        }
        _ => None,
    }
}

