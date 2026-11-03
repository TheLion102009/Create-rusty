//! Create's data files in 26.3's shapes: recipes, loot tables, tags and world generation.
use crate::upgrade;
use serde_json::{Map, Value, json};

/// What happened to a recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipe {
    /// A vanilla type, upgraded to 26.3: the game crafts it.
    Vanilla,
    /// A vanilla type the upgrader can't rewrite (a custom ingredient): kept as it is,
    /// and the game may refuse it.
    VanillaKept,
    /// One of Create's own types (`create:pressing`): loaded as data for the native mod
    /// (`recipes_of_type`), crafted by nothing until it handles it.
    Create,
    /// Its `neoforge:conditions` need a mod Horizon doesn't have.
    Dropped,
}

/// Upgrades a recipe in place.
pub fn recipe(v: &mut Value) -> Recipe {
    if let Some(conditions) = v.as_object_mut().and_then(|o| o.remove("neoforge:conditions")) {
        let all = conditions.as_array().into_iter().flatten().all(condition);
        if !all {
            return Recipe::Dropped;
        }
    }
    upgrade::renamed_ids(v);
    let kind = v["type"].as_str().unwrap_or("").to_owned();
    if kind.starts_with("minecraft:") {
        if upgrade::upgrade_recipe(v) {
            Recipe::Vanilla
        } else {
            Recipe::VanillaKept
        }
    } else {
        ingredients(v);
        Recipe::Create
    }
}

/// A NeoForge load condition, with no other mod present.
fn condition(c: &Value) -> bool {
    match c["type"].as_str().unwrap_or("") {
        "neoforge:mod_loaded" => matches!(c["modid"].as_str(), Some("minecraft" | "create")),
        "neoforge:not" => !condition(&c["value"]),
        "neoforge:and" => c["values"].as_array().into_iter().flatten().all(condition),
        "neoforge:or" => c["values"].as_array().into_iter().flatten().any(condition),
        "neoforge:true" => true,
        "neoforge:false" => false,
        // `tag_empty`, `item_exists` and the rest ask about other mods' content: leave
        // the recipe out rather than guess.
        _ => false,
    }
}

/// Create's processing recipes name ingredients as 1.21.1 did (`{"item": ...}`); written in
/// 26.3's form where one exists, so the native mod reads one shape. Fluid ingredients and
/// others without one stay as they are.
fn ingredients(v: &mut Value) {
    match v {
        Value::Object(o) => {
            for (k, child) in o.iter_mut() {
                match k.as_str() {
                    "ingredients" => {
                        for i in child.as_array_mut().into_iter().flatten() {
                            if let Some(new) = upgrade::ingredient(i) {
                                *i = new;
                            } else {
                                ingredients(i);
                            }
                        }
                    }
                    "key" => {
                        for i in child.as_object_mut().into_iter().flat_map(|m| m.values_mut()) {
                            if let Some(new) = upgrade::ingredient(i) {
                                *i = new;
                            }
                        }
                    }
                    _ => ingredients(child),
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(ingredients),
        _ => {}
    }
}

/// Upgrades a loot table in place.
pub fn loot_table(v: &mut Value) {
    upgrade::upgrade_loot(v);
    upgrade::renamed_ids(v);
}

/// Upgrades a tag file in place: renamed vanilla ids.
pub fn tag(v: &mut Value) {
    upgrade::renamed_ids(v);
}

/// Block and fluid states as 26.3 writes them: 1.21.1's `{"Name", "Properties"}` became
/// `{"id", "properties"}` (the old shape fails a whole feature in 26.3).
pub fn block_states(v: &mut Value) -> bool {
    match v {
        Value::Object(o) => {
            // As Horizon's own carry-over does: only an object that is nothing but a state.
            let state = o.get("Name").is_some_and(Value::is_string)
                && o.keys().all(|k| k == "Name" || k == "Properties");
            if state {
                let mut new = Map::new();
                if let Some(name) = o.remove("Name") {
                    new.insert("id".into(), name);
                }
                if let Some(p) = o.remove("Properties") {
                    new.insert("properties".into(), p);
                }
                *o = new;
                return true;
            }
            o.values_mut().fold(false, |c, x| block_states(x) | c)
        }
        Value::Array(a) => a.iter_mut().fold(false, |c, x| block_states(x) | c),
        _ => false,
    }
}

/// Whether a configured feature is one 26.3 can read: a vanilla type. Create's striated
/// ore layers (`create:layered_ore`) are Java code.
pub fn vanilla_feature(v: &Value) -> bool {
    v["type"].as_str().is_some_and(|t| t.starts_with("minecraft:"))
}

/// The configured feature a placed feature places, if it names one by id.
pub fn placed_feature_target(v: &Value) -> Option<&str> {
    v["feature"].as_str()
}

/// The placed features a NeoForge biome modifier adds.
pub fn modifier_features(v: &Value) -> Vec<String> {
    match &v["features"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

/// An item model definition (1.21.4's `items/<name>.json`) for a 1.21.1 item model.
pub fn item_definition(model: &str) -> Value {
    json!({ "model": { "type": "minecraft:model", "model": model } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_for_other_mods_are_dropped() {
        let mut r = json!({
            "neoforge:conditions": [{"type": "neoforge:mod_loaded", "modid": "mekanism"}],
            "type": "create:crushing", "ingredients": [{"item": "mekanism:ore"}]
        });
        assert_eq!(recipe(&mut r), Recipe::Dropped);
        let mut not = json!({
            "neoforge:conditions": [{"type": "neoforge:not", "value": {"type": "neoforge:mod_loaded", "modid": "quark"}}],
            "type": "minecraft:crafting_shapeless", "ingredients": [{"item": "minecraft:stone"}],
            "result": {"id": "create:cut_andesite", "count": 1}
        });
        assert_eq!(recipe(&mut not), Recipe::Vanilla);
        assert!(not.get("neoforge:conditions").is_none());
        assert_eq!(not["ingredients"][0], "minecraft:stone");
    }

    #[test]
    fn create_recipes_take_26_3_ingredients_and_keep_fluids() {
        let mut r = json!({
            "type": "create:mixing",
            "ingredients": [
                {"item": "minecraft:copper_ingot"},
                {"tag": "c:ingots/zinc"},
                {"type": "neoforge:single", "fluid": "minecraft:lava", "amount": 100}
            ],
            "results": [{"id": "create:brass_ingot", "count": 2}],
            "heat_requirement": "heated"
        });
        assert_eq!(recipe(&mut r), Recipe::Create);
        assert_eq!(r["ingredients"][0], "minecraft:copper_ingot");
        assert_eq!(r["ingredients"][1], "#c:ingots/zinc");
        assert_eq!(r["ingredients"][2]["fluid"], "minecraft:lava");
    }

    #[test]
    fn block_states_take_ids() {
        let mut f = json!({"targets": [{"state": {"Name": "create:zinc_ore"}}]});
        assert!(block_states(&mut f));
        assert_eq!(f["targets"][0]["state"], json!({"id": "create:zinc_ore"}));
    }
}
