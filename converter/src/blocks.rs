//! A Horizon block declaration (`data/create/horizon/block/<name>.json`) for each Create block.
//!
//! Horizon blocks copy a vanilla *template*: its behaviour, states, shapes and, importantly,
//! its occlusion. So the template is picked for what the block does (a slab, a door, a
//! ladder, a block placed along an axis) and for whether it hides its neighbours' faces:
//! a shaft must not take stone's occlusion, or the blocks around it show holes. What the
//! template lacks comes from the declaration: state properties of its own (so every
//! property Create's blockstate files test exists), shapes ported from `AllShapes.java`,
//! and hardness, sounds, light and map colour from the Java registration.
use crate::java::JavaBlock;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// A blockstate file's properties: each name with the values its variants and multipart
/// conditions test, in first-seen order.
pub type Properties = BTreeMap<String, Vec<String>>;

pub fn blockstate_properties(blockstate: &Value) -> Properties {
    let mut out: Properties = BTreeMap::new();
    let mut add = |k: &str, v: &str| {
        let values = out.entry(k.to_owned()).or_default();
        if !values.iter().any(|x| x == v) {
            values.push(v.to_owned());
        }
    };
    if let Some(variants) = blockstate["variants"].as_object() {
        for key in variants.keys() {
            for pair in key.split(',').filter(|p| !p.is_empty()) {
                if let Some((k, v)) = pair.split_once('=') {
                    add(k, v);
                }
            }
        }
    }
    fn walk(when: &Value, add: &mut dyn FnMut(&str, &str)) {
        let Some(o) = when.as_object() else { return };
        for (k, v) in o {
            if k == "OR" || k == "AND" {
                for c in v.as_array().into_iter().flatten() {
                    walk(c, add);
                }
            } else {
                let text = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                for value in text.trim_start_matches('!').split('|') {
                    add(k, value);
                }
            }
        }
    }
    for part in blockstate["multipart"].as_array().into_iter().flatten() {
        walk(&part["when"], &mut add);
    }
    out
}

/// A vanilla template: its id and its state properties with their values.
struct Template {
    id: &'static str,
    props: &'static [(&'static str, &'static [&'static str])],
}

const FACING4: &[&str] = &["north", "south", "west", "east"];
const FACING6: &[&str] = &["down", "up", "north", "south", "west", "east"];
const BOOL: &[&str] = &["false", "true"];
const AXIS: &[&str] = &["x", "y", "z"];
const WALL_SIDE: &[&str] = &["none", "low", "tall"];

const SLAB: Template = Template {
    id: "minecraft:stone_slab",
    props: &[("type", &["top", "bottom", "double"]), ("waterlogged", BOOL)],
};
const STAIRS: Template = Template {
    id: "minecraft:stone_stairs",
    props: &[
        ("facing", FACING4),
        ("half", &["top", "bottom"]),
        (
            "shape",
            &["straight", "inner_left", "inner_right", "outer_left", "outer_right"],
        ),
        ("waterlogged", BOOL),
    ],
};
const WALL: Template = Template {
    id: "minecraft:cobblestone_wall",
    props: &[
        ("up", BOOL),
        ("north", WALL_SIDE),
        ("east", WALL_SIDE),
        ("south", WALL_SIDE),
        ("west", WALL_SIDE),
        ("waterlogged", BOOL),
    ],
};
const PANE_PROPS: &[(&str, &[&str])] = &[
    ("north", BOOL),
    ("east", BOOL),
    ("south", BOOL),
    ("west", BOOL),
    ("waterlogged", BOOL),
];
const PANE: Template = Template {
    id: "minecraft:glass_pane",
    props: PANE_PROPS,
};
const BARS: Template = Template {
    id: "minecraft:iron_bars",
    props: PANE_PROPS,
};
const DOOR: Template = Template {
    id: "minecraft:copper_door",
    props: &[
        ("facing", FACING4),
        ("half", &["upper", "lower"]),
        ("hinge", &["left", "right"]),
        ("open", BOOL),
        ("powered", BOOL),
    ],
};
const TRAPDOOR: Template = Template {
    id: "minecraft:copper_trapdoor",
    props: &[
        ("facing", FACING4),
        ("half", &["top", "bottom"]),
        ("open", BOOL),
        ("powered", BOOL),
        ("waterlogged", BOOL),
    ],
};
const LADDER: Template = Template {
    id: "minecraft:ladder",
    props: &[("facing", FACING4), ("waterlogged", BOOL)],
};
const SCAFFOLDING: Template = Template {
    id: "minecraft:scaffolding",
    props: &[
        ("bottom", BOOL),
        ("distance", &["0", "1", "2", "3", "4", "5", "6", "7"]),
        ("waterlogged", BOOL),
    ],
};
/// A full block placed along the clicked face's axis.
const PILLAR: Template = Template {
    id: "minecraft:quartz_pillar",
    props: &[("axis", AXIS)],
};
/// A thin block placed along the clicked face's axis that hides nothing.
const AXLE: Template = Template {
    id: "minecraft:iron_chain",
    props: &[("axis", AXIS), ("waterlogged", BOOL)],
};
/// A block facing the clicked face, any of six, that hides nothing.
const DIRECTIONAL: Template = Template {
    id: "minecraft:lightning_rod",
    props: &[("facing", FACING6), ("powered", BOOL), ("waterlogged", BOOL)],
};
/// A full block facing the placer, one of four.
const HORIZONTAL: Template = Template {
    id: "minecraft:white_glazed_terracotta",
    props: &[("facing", FACING4)],
};
const CARPET: Template = Template {
    id: "minecraft:white_carpet",
    props: &[],
};
/// A template without properties: its states are all the block declares.
const fn plain(id: &'static str) -> Template {
    Template { id, props: &[] }
}

/// Vanilla blocks without state properties a Java `initialProperties` may copy.
const PLAIN_BLOCKS: &[&str] = &[
    "minecraft:andesite",
    "minecraft:stone",
    "minecraft:gold_block",
    "minecraft:iron_block",
    "minecraft:copper_block",
    "minecraft:netherite_block",
    "minecraft:spruce_planks",
    "minecraft:glass",
    "minecraft:glowstone",
    "minecraft:obsidian",
    "minecraft:dirt",
    "minecraft:clay",
    "minecraft:white_wool",
    "minecraft:calcite",
    "minecraft:tuff",
    "minecraft:dripstone_block",
    "minecraft:polished_deepslate",
    "minecraft:quartz_block",
    "minecraft:cut_copper",
    "minecraft:iron_ore",
    "minecraft:deepslate_iron_ore",
    "minecraft:raw_iron_block",
    "minecraft:sculk",
];

/// Hardness, blast resistance, whether it needs a tool, and its sounds, for the vanilla
/// blocks Create's `SharedProperties` copy.
fn material(copies: &str) -> (f64, f64, bool, &'static str) {
    match copies {
        "minecraft:stripped_spruce_wood" => (2.0, 2.0, false, "minecraft:stripped_spruce_wood"),
        "minecraft:gold_block" => (3.0, 6.0, true, "minecraft:gold_block"),
        "minecraft:copper_block" => (3.0, 6.0, true, "minecraft:copper_block"),
        "minecraft:netherite_block" => (50.0, 1200.0, true, "minecraft:netherite_block"),
        "minecraft:iron_block" => (5.0, 6.0, true, "minecraft:iron_block"),
        "minecraft:glass" => (0.3, 0.3, false, "minecraft:glass"),
        _ => (1.5, 6.0, true, "minecraft:andesite"),
    }
}

/// What the converter knows of one block.
pub struct Input<'a> {
    pub name: &'a str,
    pub props: &'a Properties,
    pub java: Option<&'a JavaBlock>,
    /// Its default model is a full cube (from the model files).
    pub full_model: bool,
    pub has_item: bool,
    /// Drawn by the native mod's block entity (`--animated`).
    pub animated: bool,
}

pub struct Declared {
    pub json: Value,
    /// The template picked, for the report.
    pub template: &'static str,
    /// State properties declared beside the template's.
    pub declared: Vec<String>,
}

pub fn declare(b: &Input) -> Declared {
    let names: BTreeSet<&str> = b.props.keys().map(String::as_str).collect();
    let has = |p: &str| names.contains(p);
    let only = |ps: &[&str]| names.iter().all(|n| ps.contains(n));
    let values = |p: &str| b.props.get(p).cloned().unwrap_or_default();
    let ends = |s: &str| b.name.ends_with(s);
    let occludes = b.full_model && !b.java.is_some_and(|j| j.no_occlusion);
    let facing = values("facing");
    let facing6 = facing.iter().any(|f| f == "up") && facing.iter().any(|f| f == "down");
    let facing4 = !facing.is_empty() && facing.iter().all(|f| FACING4.contains(&f.as_str()));
    let axis3 = values("axis").len() == 3;

    let chosen = if has("type") && only(&["type", "waterlogged"]) && ends("_slab") {
        SLAB
    } else if ends("_stairs") && only(&["facing", "half", "shape", "waterlogged"]) {
        STAIRS
    } else if ends("_wall") && has("up") {
        WALL
    } else if only(&["north", "east", "south", "west", "waterlogged"]) && has("north") {
        if ends("_bars") { BARS } else { PANE }
    } else if ends("_door") && has("hinge") {
        DOOR
    } else if ends("_trapdoor") {
        TRAPDOOR
    } else if ends("_ladder") && only(&["facing", "waterlogged"]) {
        LADDER
    } else if ends("_scaffolding") && only(&["bottom", "waterlogged"]) {
        SCAFFOLDING
    } else if ends("_seat") || ends("_table_cloth") {
        CARPET
    } else if axis3 && !has("facing") {
        if occludes { PILLAR } else { AXLE }
    } else if facing6 {
        DIRECTIONAL
    } else if facing4 && occludes && !has("face") {
        HORIZONTAL
    } else if names.is_empty() {
        plain(plain_template(b, occludes))
    } else {
        // Nothing vanilla has these states: a plain block declares them all. Its facing
        // or axis is then set on placement by the native mod (`state_for_placement`).
        plain(plain_template(b, occludes))
    };

    // A template whose property lacks a value the blockstate tests can't hold the block's
    // states: a plain one declares them all instead.
    let fits = |t: &Template| {
        b.props.iter().all(|(name, vals)| {
            t.props
                .iter()
                .find(|(p, _)| p == name)
                .is_none_or(|(_, have)| vals.iter().all(|v| have.contains(&v.as_str())))
        })
    };
    let template = if fits(&chosen) {
        chosen
    } else {
        plain(plain_template(b, occludes))
    };
    let mut declared_props = Map::new();
    let mut declared = Vec::new();
    for (name, vals) in b.props {
        if template.props.iter().any(|(p, _)| p == name) {
            continue;
        }
        declared.push(name.clone());
        // Vanilla's first value is the default: `ordered` puts the sensible one first.
        let spec = if vals.iter().all(|v| v == "true" || v == "false") {
            json!({"type": "boolean"})
        } else {
            json!({"type": "enum", "values": ordered(name, vals)})
        };
        declared_props.insert(name.clone(), spec);
    }

    let mut properties = Map::new();
    let copies = b
        .java
        .and_then(|j| j.copies.clone())
        .unwrap_or_else(|| guess_material(b.name).to_owned());
    let (hardness, resistance, requires_tool, sounds) = material(&copies);
    let java = b.java.cloned().unwrap_or_default();
    // Templates picked for their states take the block's own numbers; the palette's
    // stone and metal templates keep theirs.
    let behaviour_template = !template.props.is_empty() || template.id == CARPET.id;
    let glass_machine = template.id == "minecraft:glass" && guess_material(b.name) != "minecraft:glass";
    if behaviour_template || glass_machine || b.java.is_some() {
        let (h, r) = java
            .strength
            .map(|(h, r)| (h, r.unwrap_or(h)))
            .unwrap_or((hardness, resistance));
        properties.insert("hardness".into(), json!(h));
        properties.insert("explosion_resistance".into(), json!(r));
        properties.insert(
            "requires_tool".into(),
            json!(java.requires_tool || (requires_tool && b.java.is_some())),
        );
        let sounds = java.sounds.clone().unwrap_or_else(|| sounds.to_owned());
        properties.insert("sounds".into(), json!(sounds));
    }
    if let Some(c) = java.map_color {
        properties.insert("map_color".into(), json!(c));
    }
    if let Some(l) = java.light {
        properties.insert("light".into(), json!(l));
    } else if template.id == DIRECTIONAL.id {
        properties.insert("light".into(), json!(0));
    }
    if weathers(template.id) {
        properties.insert("random_ticks".into(), json!(false));
    }
    if b.animated {
        properties.insert("render_shape".into(), json!("ENTITYBLOCK_ANIMATED"));
    }

    let mut v = json!({ "template": template.id });
    if !properties.is_empty() {
        v["properties"] = Value::Object(properties);
    }
    if let Some(shapes) = shapes(b.name, &template) {
        v["shapes"] = shapes;
    }
    if !declared_props.is_empty() {
        v["state_properties"] = Value::Object(declared_props);
    }
    if !b.has_item {
        v["item"] = json!(false);
    }
    Declared {
        json: v,
        template: template.id,
        declared,
    }
}

/// A full plain block of the right material, or glass when it must not occlude.
fn plain_template(b: &Input, occludes: bool) -> &'static str {
    let n = b.name;
    if !occludes {
        return "minecraft:glass";
    }
    if n.ends_with("_window") || n.contains("framed_glass") || n == "tiled_glass" {
        return "minecraft:glass";
    }
    if n == "zinc_ore" {
        return "minecraft:iron_ore";
    }
    if n == "deepslate_zinc_ore" {
        return "minecraft:deepslate_iron_ore";
    }
    if n == "raw_zinc_block" {
        return "minecraft:raw_iron_block";
    }
    if n.contains("copper_shingles") || n.contains("copper_tiles") {
        return "minecraft:cut_copper";
    }
    if n.ends_with("_block") && ["zinc", "brass", "andesite_alloy", "industrial_iron", "weathered_iron"]
        .iter()
        .any(|m| n.starts_with(m))
    {
        return "minecraft:iron_block";
    }
    if n.contains("deepslate") {
        return "minecraft:polished_deepslate";
    }
    if n.contains("tuff") {
        return "minecraft:tuff";
    }
    if n.contains("calcite") {
        return "minecraft:calcite";
    }
    if n.contains("dripstone") {
        return "minecraft:dripstone_block";
    }
    if n.contains("rose_quartz") {
        return "minecraft:quartz_block";
    }
    if let Some(copies) = b.java.and_then(|j| j.copies.as_deref()) {
        if copies == "minecraft:stripped_spruce_wood" {
            return "minecraft:spruce_planks";
        }
        if let Some(p) = PLAIN_BLOCKS.iter().find(|p| **p == copies) {
            return p;
        }
    }
    "minecraft:stone"
}

/// The material of a block registered in a loop, by its name.
fn guess_material(name: &str) -> &'static str {
    if name.contains("copper") {
        "minecraft:copper_block"
    } else if name.ends_with("_window") || name.ends_with("_pane") || name.contains("glass") {
        "minecraft:glass"
    } else if name.ends_with("_seat") || name.ends_with("_table_cloth") {
        "minecraft:stripped_spruce_wood"
    } else if name.ends_with("_bars") || name.ends_with("_ladder") || name.ends_with("_scaffolding")
    {
        "minecraft:iron_block"
    } else {
        "minecraft:andesite"
    }
}

/// Copper templates oxidise on random ticks; a copy of one has nothing to turn into.
fn weathers(template: &str) -> bool {
    template.contains("copper") || template == DIRECTIONAL.id
}

/// An enum's values with its default first: vanilla's first value is the default.
fn ordered(name: &str, values: &[String]) -> Vec<String> {
    let preferred: &[&str] = match name {
        // A block that faces down but never up (a basin, a chute) rests facing down.
        "facing" if values.iter().any(|v| v == "down") && !values.iter().any(|v| v == "up") => {
            &["down"]
        }
        "facing" => &["north", "up"],
        "axis" => &["y", "x"],
        _ => &[
            "none", "normal", "plain", "single", "straight", "regular", "floor", "bottom", "0",
            "small", "start",
        ],
    };
    let mut out = values.to_vec();
    if let Some(i) = preferred
        .iter()
        .find_map(|p| out.iter().position(|v| v == p))
    {
        let first = out.remove(i);
        out.insert(0, first);
    }
    out
}

/// A box in pixels (0..16), as Create's `AllShapes` writes them.
type Px = [f64; 6];

/// Boxes along the Y axis turned to `axis`, in block units.
fn for_axis(boxes: &[Px], axis: &str) -> Value {
    let turned: Vec<Value> = boxes
        .iter()
        .map(|&[x1, y1, z1, x2, y2, z2]| {
            let b = match axis {
                "x" => [y1, x1, z1, y2, x2, z2],
                "z" => [x1, z1, y1, x2, z2, y2],
                _ => [x1, y1, z1, x2, y2, z2],
            };
            json!(b.map(|v| v / 16.0))
        })
        .collect();
    Value::Array(turned)
}

/// Boxes for a block facing up turned to `facing`, in block units (`VoxelShaper.forDirectional`).
fn for_direction(boxes: &[Px], facing: &str) -> Value {
    let turned: Vec<Value> = boxes
        .iter()
        .map(|&[x1, y1, z1, x2, y2, z2]| {
            let b = match facing {
                "down" => [x1, 16.0 - y2, z1, x2, 16.0 - y1, z2],
                "north" => [x1, z1, 16.0 - y2, x2, z2, 16.0 - y1],
                "south" => [x1, z1, y1, x2, z2, y2],
                "west" => [16.0 - y2, x1, z1, 16.0 - y1, x2, z2],
                "east" => [y1, x1, z1, y2, x2, z2],
                _ => [x1, y1, z1, x2, y2, z2],
            };
            json!(b.map(|v| v / 16.0))
        })
        .collect();
    Value::Array(turned)
}

// Create's shapes (`AllShapes.java`, MIT), in pixels, facing up / along Y.
const SIX_VOXEL_POLE: Px = [5.0, 0.0, 5.0, 11.0, 16.0, 11.0];
const SMALL_GEAR: [Px; 2] = [[2.0, 6.0, 2.0, 14.0, 10.0, 14.0], SIX_VOXEL_POLE];
const LARGE_GEAR: [Px; 2] = [[0.0, 6.0, 0.0, 16.0, 10.0, 16.0], SIX_VOXEL_POLE];
const MOTOR_BLOCK: [Px; 1] = [[3.0, 0.0, 3.0, 13.0, 14.0, 13.0]];
const CRANK: [Px; 2] = [
    [5.0, 0.0, 5.0, 11.0, 6.0, 11.0],
    [1.0, 3.0, 1.0, 15.0, 8.0, 15.0],
];
const VALVE_HANDLE: [Px; 2] = [
    [5.0, 0.0, 5.0, 11.0, 4.0, 11.0],
    [1.0, 3.0, 1.0, 15.0, 8.0, 15.0],
];
const MILLSTONE: [Px; 2] = [
    [0.0, 0.0, 0.0, 16.0, 6.0, 16.0],
    [2.0, 6.0, 2.0, 14.0, 16.0, 14.0],
];
const SEAT: [Px; 1] = [[0.0, 0.0, 0.0, 16.0, 8.0, 16.0]];

fn shapes(name: &str, template: &Template) -> Option<Value> {
    let by_axis = |boxes: &[Px]| {
        let values: Map<String, Value> = AXIS
            .iter()
            .map(|a| (a.to_string(), for_axis(boxes, a)))
            .collect();
        json!({"property": "axis", "values": values})
    };
    let by_facing = |boxes: &[Px]| {
        let values: Map<String, Value> = FACING6
            .iter()
            .map(|f| (f.to_string(), for_direction(boxes, f)))
            .collect();
        json!({"property": "facing", "values": values})
    };
    let full = || json!({"default": [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]});
    Some(match name {
        "shaft" | "powered_shaft" => by_axis(&[SIX_VOXEL_POLE]),
        "cogwheel" => by_axis(&SMALL_GEAR),
        "large_cogwheel" | "flywheel" => by_axis(&LARGE_GEAR),
        "creative_motor" => by_facing(&MOTOR_BLOCK),
        "hand_crank" => by_facing(&CRANK),
        n if n.ends_with("_valve_handle") => by_facing(&VALVE_HANDLE),
        "millstone" => json!({"default": for_axis(&MILLSTONE, "y")}),
        n if n.ends_with("_seat") => json!({"default": for_axis(&SEAT, "y")}),
        // Templates picked for their states keep a full outline unless a shape is known:
        // a chain's or a lightning rod's thin one would be wrong for a machine.
        _ if template.id == AXLE.id || template.id == DIRECTIONAL.id => full(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pairs: &[(&str, &[&str])]) -> Properties {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    fn input<'a>(name: &'a str, props: &'a Properties, full: bool) -> Input<'a> {
        Input {
            name,
            props,
            java: None,
            full_model: full,
            has_item: true,
            animated: false,
        }
    }

    #[test]
    fn reads_variants_and_multipart() {
        let bs = json!({
            "variants": {"axis=x": {}, "axis=y": {}},
            "multipart": [{"when": {"OR": [{"north": "true"}, {"facing": "up|down"}]}}]
        });
        let p = blockstate_properties(&bs);
        assert_eq!(p["axis"], ["x", "y"]);
        assert_eq!(p["north"], ["true"]);
        assert_eq!(p["facing"], ["up", "down"]);
    }

    #[test]
    fn a_shaft_is_a_thin_axle_with_its_pole_shape() {
        let p = props(&[("axis", &["x", "y", "z"])]);
        let d = declare(&input("shaft", &p, false));
        assert_eq!(d.template, "minecraft:iron_chain");
        assert!(d.declared.is_empty());
        let x = &d.json["shapes"]["values"]["x"][0];
        assert_eq!(*x, json!([0.0, 0.3125, 0.3125, 1.0, 0.6875, 0.6875]));
    }

    #[test]
    fn a_gearbox_is_a_full_pillar() {
        let p = props(&[("axis", &["x", "y", "z"])]);
        assert_eq!(declare(&input("gearbox", &p, true)).template, "minecraft:quartz_pillar");
    }

    #[test]
    fn templates_without_a_property_declare_it() {
        let p = props(&[
            ("axis", &["x", "y", "z"]),
            ("bottom_shaft", &["false", "true"]),
            ("top_shaft", &["false", "true"]),
        ]);
        let d = declare(&input("andesite_encased_cogwheel", &p, true));
        assert_eq!(d.template, "minecraft:quartz_pillar");
        assert_eq!(d.declared, ["bottom_shaft", "top_shaft"]);
        assert_eq!(d.json["state_properties"]["top_shaft"]["type"], "boolean");
    }

    #[test]
    fn six_way_blocks_take_the_lightning_rod_and_a_full_outline() {
        let p = props(&[("facing", FACING6)]);
        let d = declare(&input("encased_fan", &p, false));
        assert_eq!(d.template, "minecraft:lightning_rod");
        assert_eq!(d.json["shapes"]["default"][0], json!([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        assert_eq!(d.json["properties"]["light"], 0);
        assert_eq!(d.json["properties"]["random_ticks"], false);
    }

    #[test]
    fn horizontal_blocks_that_dont_occlude_declare_their_facing() {
        let p = props(&[("facing", FACING4)]);
        let d = declare(&input("mechanical_press", &p, false));
        assert_eq!(d.template, "minecraft:glass");
        assert_eq!(d.declared, ["facing"]);
        let full = declare(&input("schematic_table", &p, true));
        assert_eq!(full.template, "minecraft:white_glazed_terracotta");
    }

    #[test]
    fn a_facing_without_north_defaults_to_its_first_value() {
        let p = props(&[("facing", &["down", "north", "south", "west", "east"])]);
        let d = declare(&input("basin", &p, false));
        assert_eq!(d.json["state_properties"]["facing"]["values"][0], "down");
    }

    #[test]
    fn palettes_keep_vanilla_behaviour() {
        let slab = props(&[("type", &["bottom", "double", "top"])]);
        assert_eq!(declare(&input("cut_andesite_slab", &slab, false)).template, "minecraft:stone_slab");
        let none = Properties::new();
        assert_eq!(declare(&input("cut_tuff", &none, true)).template, "minecraft:tuff");
        assert_eq!(declare(&input("oak_window", &none, true)).template, "minecraft:glass");
        assert_eq!(declare(&input("zinc_ore", &none, true)).template, "minecraft:iron_ore");
    }

    #[test]
    fn shapes_turn_to_every_direction() {
        let down = for_direction(&[[5.0, 0.0, 5.0, 11.0, 6.0, 11.0]], "down");
        assert_eq!(down[0], json!([0.3125, 0.625, 0.3125, 0.6875, 1.0, 0.6875]));
        let north = for_direction(&[[5.0, 0.0, 5.0, 11.0, 6.0, 11.0]], "north");
        assert_eq!(north[0], json!([0.3125, 0.3125, 0.625, 0.6875, 0.6875, 1.0]));
    }
}
