//! A Horizon item declaration (`data/create/horizon/item/<name>.json`) for each Create item
//! that isn't a block's.
//!
//! An item copies a vanilla template's data components. Most of Create's items are crafting
//! materials, so they copy a stick (no components of note). Food copies food, armour copies
//! the matching vanilla armour piece (copper diving gear the copper armour 26.3 has), and
//! stack sizes, durability, rarity and nutrition come from `AllItems.java`. What the items
//! *do* (a wrench turning blocks, goggles showing stress) is code: the native mod's job.
use crate::java::JavaItem;
use serde_json::{Map, Value, json};

pub struct Declared {
    pub json: Value,
    pub template: &'static str,
}

pub fn declare(name: &str, java: Option<&JavaItem>) -> Declared {
    let java = java.cloned().unwrap_or_default();
    let template = template(name, &java);
    let mut v = json!({ "template": template });
    let mut components = Map::new();

    // A template's own stack size stands unless Create sets one (armour and tools are 1).
    let one = matches!(
        name,
        "wrench"
            | "goggles"
            | "extendo_grip"
            | "potato_cannon"
            | "wand_of_symmetry"
            | "handheld_worldshaper"
            | "linked_controller"
            | "crafting_blueprint"
            | "schematic"
            | "schematic_and_quill"
            | "filter"
            | "attribute_filter"
            | "package_filter"
            | "schedule"
            | "shopping_list"
            | "minecart_coupling"
            | "package"
            | "rare_package"
            | "chest_minecart_contraption"
            | "furnace_minecart_contraption"
            | "minecart_contraption"
            | "honey_bucket"
            | "chocolate_bucket"
            | "copper_backtank_placeable"
            | "netherite_backtank_placeable"
    );
    let stack = java.stacks_to.or(one.then_some(1));
    let armour = is_armour(template);
    if let Some(n) = stack.filter(|_| !armour) {
        v["max_stack_size"] = json!(n.clamp(1, 99));
    }
    if let Some(r) = java.rarity.as_deref().filter(|r| r != &"common") {
        v["rarity"] = json!(r);
    }
    if let Some((nutrition, modifier)) = java.food {
        // 1.21.1's saturation modifier is per nutrition point, times two.
        let saturation = (nutrition as f64 * modifier * 2.0 * 1000.0).round() / 1000.0;
        components.insert(
            "minecraft:food".into(),
            json!({ "nutrition": nutrition, "saturation": saturation }),
        );
    }
    if let Some(d) = java.durability {
        components.insert("minecraft:max_damage".into(), json!(d));
    }
    if name == "goggles" {
        // Worn on the head, drawn by nothing yet: no vanilla armour model on the player.
        components.insert("minecraft:equippable".into(), json!({ "slot": "head" }));
    }
    if !components.is_empty() {
        v["components"] = Value::Object(components);
    }
    if matches!(name, "honey_bucket" | "chocolate_bucket") {
        v["crafting_remainder"] = json!("minecraft:bucket");
    }
    Declared { json: v, template }
}

fn is_armour(template: &str) -> bool {
    ["_helmet", "_chestplate", "_leggings", "_boots"]
        .iter()
        .any(|s| template.ends_with(s))
}

fn template(name: &str, java: &JavaItem) -> &'static str {
    match name {
        "copper_diving_helmet" => "minecraft:copper_helmet",
        "copper_diving_boots" => "minecraft:copper_boots",
        "copper_backtank" => "minecraft:copper_chestplate",
        "netherite_diving_helmet" => "minecraft:netherite_helmet",
        "netherite_diving_boots" => "minecraft:netherite_boots",
        "netherite_backtank" => "minecraft:netherite_chestplate",
        "cardboard_helmet" => "minecraft:leather_helmet",
        "cardboard_chestplate" => "minecraft:leather_chestplate",
        "cardboard_leggings" => "minecraft:leather_leggings",
        "cardboard_boots" => "minecraft:leather_boots",
        "cardboard_sword" => "minecraft:wooden_sword",
        "builders_tea" => "minecraft:honey_bottle",
        "honeyed_apple" => "minecraft:apple",
        "bar_of_chocolate" | "chocolate_glazed_berries" => "minecraft:cookie",
        "sweet_roll" => "minecraft:bread",
        _ if java.food.is_some() => "minecraft:bread",
        _ => "minecraft:stick",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_copy_a_stick() {
        let d = declare("brass_ingot", None);
        assert_eq!(d.json, json!({"template": "minecraft:stick"}));
    }

    #[test]
    fn food_takes_its_nutrition() {
        let java = JavaItem {
            food: Some((6, 0.8)),
            ..Default::default()
        };
        let d = declare("sweet_roll", Some(&java));
        assert_eq!(d.template, "minecraft:bread");
        let food = &d.json["components"]["minecraft:food"];
        assert_eq!(food["nutrition"], 6);
        assert!((food["saturation"].as_f64().unwrap() - 9.6).abs() < 1e-4);
    }

    #[test]
    fn tools_stack_to_one_and_armour_keeps_its_template() {
        assert_eq!(declare("wrench", None).json["max_stack_size"], 1);
        let java = JavaItem {
            durability: Some(77),
            ..Default::default()
        };
        let helmet = declare("copper_diving_helmet", Some(&java));
        assert_eq!(helmet.template, "minecraft:copper_helmet");
        assert!(helmet.json.get("max_stack_size").is_none());
        assert_eq!(helmet.json["components"]["minecraft:max_damage"], 77);
    }

    #[test]
    fn buckets_leave_a_bucket() {
        let d = declare("honey_bucket", None);
        assert_eq!(d.json["crafting_remainder"], "minecraft:bucket");
    }
}
