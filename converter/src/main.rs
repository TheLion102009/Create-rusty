//! `create-horizon`: turns the Create mod's source tree (NeoForge, Minecraft 1.21.1) into a
//! Horizon content mod for Minecraft 26.3 (https://playhorizon.dev/docs/content-mods).
//!
//! ```text
//! create-horizon --create <Create checkout> [--out <dir>] [--animated] [--native-table <file>]
//! ```
//!
//! It writes `<out>/create/` (a folder mod: `horizon-mod.toml`, `assets/`, `data/`) and
//! `<out>/create-report.md`. Pack it with `cargo horizon-mod build` inside that folder.
//!
//! What carries over is Create's data and assets: every block and item, their models,
//! textures, names, recipes, loot and tags, and the zinc ore. What Create *does* is Java
//! code and is the native mod's job (`native/create`): `--animated` leaves the kinetic
//! blocks' models to its block entities, `--native-table` writes its stress table.
//!
//! Create's assets (`src/main/resources/assets/`) are All Rights Reserved by the Create
//! team: the folder this writes is for your own use, not for publishing.
mod blocks;
mod classes;
mod data;
mod items;
mod java;
mod upgrade;

use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const NS: &str = "create";
/// Blocks drawn by the native mod when `--animated` is given: their model turns.
const ANIMATED: &[&str] = &["shaft", "cogwheel", "large_cogwheel"];
/// Fluid blocks: Horizon has no mod fluids yet.
const FLUID_BLOCKS: &[&str] = &["honey", "chocolate"];

struct Args {
    create: PathBuf,
    out: PathBuf,
    animated: bool,
    native_table: Option<PathBuf>,
}

fn args() -> Result<Args, String> {
    const USAGE: &str = "usage: create-horizon --create <Create checkout> [--out <dir>] [--animated] [--native-table <file>]";
    let mut create = None;
    let mut out = PathBuf::from("out");
    let mut animated = false;
    let mut native_table = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--create" => create = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from).ok_or(USAGE)?,
            "--animated" => animated = true,
            "--native-table" => native_table = it.next().map(PathBuf::from),
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other:?}\n{USAGE}")),
        }
    }
    let create = create.ok_or(USAGE)?;
    if !create.join("src/main/java/com/simibubi/create").is_dir() {
        return Err(format!(
            "{} is not a Create source checkout (no src/main/java/com/simibubi/create)",
            create.display()
        ));
    }
    Ok(Args {
        create,
        out,
        animated,
        native_table,
    })
}

fn main() -> ExitCode {
    match args().and_then(|a| run(&a)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Create's resources by their path in a jar (`assets/create/...`): the generated ones
/// (datagen output) win over the hand-written ones with the same path.
fn resources(create: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut out = BTreeMap::new();
    for root in ["src/main/resources", "src/generated/resources"] {
        let root = create.join(root);
        if root.is_dir() {
            walk(&root, &root, &mut out)?;
        }
    }
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            walk(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.insert(rel, path);
        }
    }
    Ok(())
}

/// A path Horizon's content loader takes (`horizon_mods::content::check_path`).
fn plain_path(name: &str) -> bool {
    name.split('/').all(|p| {
        !p.is_empty()
            && p != "."
            && p != ".."
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
    })
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

struct Writer {
    root: PathBuf,
    files: usize,
}

impl Writer {
    fn bytes(&mut self, rel: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        self.files += 1;
        Ok(())
    }

    fn json(&mut self, rel: &str, v: &Value) -> Result<(), String> {
        let mut text = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
        text.push(b'\n');
        self.bytes(rel, &text)
    }

    fn copy(&mut self, rel: &str, from: &Path) -> Result<(), String> {
        let bytes = fs::read(from).map_err(|e| format!("{}: {e}", from.display()))?;
        self.bytes(rel, &bytes)
    }
}

/// Counts for the report, by what happened.
#[derive(Default)]
struct Report {
    counts: BTreeMap<&'static str, usize>,
    /// Lines listed under a heading.
    lists: BTreeMap<&'static str, Vec<String>>,
    templates: BTreeMap<&'static str, usize>,
    item_templates: BTreeMap<&'static str, usize>,
    /// Create's own recipe types, by how many recipes have each.
    recipe_types: BTreeMap<String, usize>,
}

impl Report {
    fn count(&mut self, what: &'static str) {
        *self.counts.entry(what).or_default() += 1;
    }
    fn list(&mut self, heading: &'static str, line: String) {
        self.lists.entry(heading).or_default().push(line);
    }
}

fn run(a: &Args) -> Result<(), String> {
    let res = resources(&a.create)?;
    let java_dir = a.create.join("src/main/java/com/simibubi/create");
    let java_blocks = fs::read_to_string(java_dir.join("AllBlocks.java"))
        .map(|s| java::blocks(&s))
        .unwrap_or_default();
    let java_items = fs::read_to_string(java_dir.join("AllItems.java"))
        .map(|s| java::items(&s))
        .unwrap_or_default();
    let version = fs::read_to_string(a.create.join("gradle.properties"))
        .ok()
        .and_then(|p| {
            p.lines()
                .find_map(|l| l.strip_prefix("mod_version").map(|v| v.trim_start_matches([' ', '=']).trim().to_owned()))
        })
        .unwrap_or_else(|| "0".into());

    let mod_dir = a.out.join(NS);
    clean(&mod_dir)?;
    let mut w = Writer {
        root: mod_dir.clone(),
        files: 0,
    };
    let mut report = Report::default();

    // Names the game shows decide what's registered: `block.create.x` and `item.create.x`.
    let lang = res
        .get("assets/create/lang/en_us.json")
        .map(|p| read_json(p))
        .transpose()?
        .unwrap_or(Value::Null);
    let lang_names = |prefix: &str| -> BTreeSet<String> {
        lang.as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, _)| k.strip_prefix(prefix))
            .filter(|rest| !rest.contains('.'))
            .map(str::to_owned)
            .collect()
    };
    let item_names = lang_names("item.create.");
    let has_item_model =
        |name: &str| res.contains_key(&format!("assets/create/models/item/{name}.json"));

    // Blocks: one declaration per blockstate file.
    let mut block_names = Vec::new();
    for rel in res.keys() {
        if let Some(name) = rel
            .strip_prefix("assets/create/blockstates/")
            .and_then(|n| n.strip_suffix(".json"))
        {
            block_names.push(name.to_owned());
        }
    }
    let mut declared_blocks = BTreeSet::new();
    let mut block_items = BTreeSet::new();
    let mut placements: Vec<(String, &'static str)> = Vec::new();
    let mut wrenchable: Vec<Wrenchable> = Vec::new();
    let classes = classes::Classes::read(&a.create.join("src/main/java"));
    for name in &block_names {
        if FLUID_BLOCKS.contains(&name.as_str()) {
            report.list("Left out", format!("block `{name}`: a fluid; Horizon has no mod fluids yet"));
            continue;
        }
        let blockstate = read_json(&res[&format!("assets/create/blockstates/{name}.json")])?;
        let props = blocks::blockstate_properties(&blockstate);
        let has_item = has_item_model(name) && !item_names.contains(name.as_str());
        let input = blocks::Input {
            name,
            props: &props,
            java: java_blocks.get(name),
            full_model: full_model(&res, &blockstate),
            has_item,
            animated: a.animated && ANIMATED.contains(&name.as_str()),
        };
        let d = blocks::declare(&input);
        *report.templates.entry(d.template).or_default() += 1;
        if !d.declared.is_empty() {
            report.count("blocks with state properties of their own");
        }
        if let Some(rule) = placement_rule(&d.declared, &props) {
            placements.push((name.clone(), rule));
        }
        // What Create's wrench turns: blocks whose class is an `IWrenchable`.
        let class = java_blocks.get(name).and_then(|j| j.class.as_deref());
        if class.is_some_and(|c| classes.is(c, "IWrenchable")) {
            wrenchable.push(Wrenchable::of(name, &props));
        }
        w.json(&format!("data/create/horizon/block/{name}.json"), &d.json)?;
        report.count("blocks");
        declared_blocks.insert(name.clone());
        if has_item {
            block_items.insert(name.clone());
        }
    }

    // Items that aren't a block's.
    let mut declared_items = BTreeSet::new();
    for name in &item_names {
        if declared_blocks.contains(name) && block_items.contains(name) {
            continue;
        }
        let d = items::declare(name, java_items.get(name));
        *report.item_templates.entry(d.template).or_default() += 1;
        w.json(&format!("data/create/horizon/item/{name}.json"), &d.json)?;
        report.count("items");
        declared_items.insert(name.clone());
    }

    // 1.21.4 moved item models behind `items/<name>.json` definitions.
    for name in block_items.iter().chain(&declared_items) {
        if has_item_model(name) {
            let def = data::item_definition(&format!("create:item/{name}"));
            w.json(&format!("assets/create/items/{name}.json"), &def)?;
        } else {
            report.list("Items without a model", format!("`{name}`: its 1.21.1 model is drawn in code"));
            let def = data::item_definition("minecraft:item/barrier");
            w.json(&format!("assets/create/items/{name}.json"), &def)?;
        }
    }

    // Creative tabs, in Create's registration order.
    creative_tabs(&mut w, &java_blocks, &java_items, &block_items, &declared_items)?;

    // Data and assets, file by file.
    let mut features_kept = BTreeSet::new();
    let mut placed_kept = BTreeSet::new();
    let mut deferred: Vec<(String, PathBuf)> = Vec::new();
    for (rel, path) in &res {
        if !plain_path(rel) {
            report.list("Left out", format!("`{rel}`: not a path Horizon loads (upper case or a space)"));
            continue;
        }
        let parts: Vec<&str> = rel.splitn(3, '/').collect();
        let [root, ns, rest] = parts[..] else { continue };
        match (root, ns) {
            ("assets", "create") => {
                let keep = rest.starts_with("models/")
                    || rest.starts_with("textures/")
                    || rest.starts_with("lang/")
                    || rest.starts_with("sounds/")
                    || rest.starts_with("particles/")
                    || rest == "sounds.json"
                    || rest
                        .strip_prefix("blockstates/")
                        .and_then(|b| b.strip_suffix(".json"))
                        .is_some_and(|b| declared_blocks.contains(b));
                if keep {
                    w.copy(rel, path)?;
                    report.count("asset files");
                } else {
                    report.count("asset files left out (ponder scenes, Flywheel shaders, particles)");
                }
            }
            ("assets", "minecraft") if rest.starts_with("atlases/") => {
                w.copy(rel, path)?;
                report.count("asset files");
            }
            ("data", "create" | "minecraft" | "c") if rest.starts_with("tags/") => {
                let mut v = read_json(path)?;
                data::tag(&mut v);
                w.json(rel, &v)?;
                report.count("tags");
            }
            ("data", "create") => {
                if rest.starts_with("recipe/") {
                    let mut v = read_json(path)?;
                    let kind = v["type"].as_str().unwrap_or("").to_owned();
                    match data::recipe(&mut v) {
                        data::Recipe::Dropped => report.count("recipes for other mods (left out)"),
                        outcome => {
                            w.json(rel, &v)?;
                            match outcome {
                                data::Recipe::Vanilla => report.count("recipes the game crafts"),
                                data::Recipe::VanillaKept => {
                                    report.list("Recipes kept as they are", format!("`{rel}` ({kind})"))
                                }
                                _ => {
                                    report.count("recipes of Create's own types (data for the native mod)");
                                    *report.recipe_types.entry(kind).or_default() += 1;
                                }
                            }
                        }
                    }
                } else if rest.starts_with("loot_table/") {
                    let mut v = read_json(path)?;
                    data::loot_table(&mut v);
                    w.json(rel, &v)?;
                    report.count("loot tables");
                } else if let Some(f) = rest
                    .strip_prefix("worldgen/configured_feature/")
                    .and_then(|f| f.strip_suffix(".json"))
                {
                    let mut v = read_json(path)?;
                    if data::vanilla_feature(&v) {
                        data::block_states(&mut v);
                        w.json(rel, &v)?;
                        features_kept.insert(format!("create:{f}"));
                    } else {
                        let kind = v["type"].as_str().unwrap_or("?");
                        report.list("Left out", format!("feature `create:{f}`: type `{kind}` is Java code"));
                    }
                } else if rest.starts_with("worldgen/placed_feature/")
                    || rest.starts_with("neoforge/biome_modifier/")
                {
                    deferred.push((rel.clone(), path.clone()));
                } else {
                    report.count("data files left out (advancements, enchantments, data maps, damage types)");
                }
            }
            _ => report.count("files of other namespaces left out (Curios, Quark, NeoForge)"),
        }
    }
    // Placed features and biome modifiers, once the features they name are known.
    for (rel, path) in deferred.iter().filter(|(r, _)| r.contains("placed_feature")) {
        let mut v = read_json(path)?;
        let name = rel
            .trim_start_matches("data/create/worldgen/placed_feature/")
            .trim_end_matches(".json");
        if data::placed_feature_target(&v).is_some_and(|t| features_kept.contains(t)) {
            data::block_states(&mut v);
            w.json(rel, &v)?;
            placed_kept.insert(format!("create:{name}"));
        }
    }
    for (rel, path) in deferred.iter().filter(|(r, _)| r.contains("biome_modifier")) {
        let v = read_json(path)?;
        let features = data::modifier_features(&v);
        if !features.is_empty() && features.iter().all(|f| placed_kept.contains(f)) {
            w.json(rel, &v)?;
            report.count("world generation (zinc ore)");
        }
    }

    let manifest = format!(
        "# Written by create-horizon from the Create source tree; do not edit by hand.\n\
         id = \"create\"\n\
         name = \"Create\"\n\
         version = \"{version}-horizon.1\"\n\
         description = \"Create's blocks, items, recipes and ore for Horizon (content only: the machines need the native mod)\"\n\
         authors = [\"simibubi\", \"The Create Team\"]\n\
         api = \"0.2\"\n\
         side = \"both\"\n\
         capabilities = [\"registries\"]\n"
    );
    w.bytes("horizon-mod.toml", manifest.as_bytes())?;

    if let Some(table) = &a.native_table {
        write_native_table(table, &java_blocks, &placements, &wrenchable)?;
    }

    let text = render_report(&report, w.files, a.animated);
    fs::write(a.out.join("create-report.md"), text).map_err(|e| e.to_string())?;
    println!(
        "wrote {} ({} files): {} blocks, {} items",
        mod_dir.display(),
        w.files,
        report.counts.get("blocks").unwrap_or(&0),
        report.counts.get("items").unwrap_or(&0),
    );
    println!("report: {}", a.out.join("create-report.md").display());
    Ok(())
}

/// Empties the output folder, but only one this tool wrote (it holds our manifest).
fn clean(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let ours = fs::read_to_string(dir.join("horizon-mod.toml"))
        .is_ok_and(|m| m.starts_with("# Written by create-horizon"));
    if !ours {
        return Err(format!(
            "{} exists and wasn't written by create-horizon: pick another --out",
            dir.display()
        ));
    }
    fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

/// Whether a blockstate's first model is a full cube: its elements, or its parents', fill
/// the block. A model drawn by a loader (OBJ, composite) or without elements is not.
fn full_model(res: &BTreeMap<String, PathBuf>, blockstate: &Value) -> bool {
    let first = |v: &Value| -> Option<String> {
        let m = if v.is_array() { &v[0] } else { v };
        m["model"].as_str().map(str::to_owned)
    };
    let model = blockstate["variants"]
        .as_object()
        .and_then(|vs| vs.values().next())
        .and_then(first)
        .or_else(|| {
            blockstate["multipart"]
                .as_array()
                .and_then(|parts| parts.first())
                .and_then(|p| first(&p["apply"]))
        });
    let Some(mut model) = model else { return false };
    for _ in 0..16 {
        let (ns, path) = model.split_once(':').unwrap_or(("minecraft", &model));
        let path = path.trim_start_matches('/');
        if ns == "minecraft" {
            const FULL: &[&str] = &[
                "block/cube",
                "block/cube_all",
                "block/cube_column",
                "block/cube_column_horizontal",
                "block/cube_bottom_top",
                "block/cube_top",
                "block/cube_directional",
                "block/cube_mirrored_all",
                "block/orientable",
                "block/orientable_with_bottom",
                "block/orientable_vertical",
                "block/leaves",
            ];
            return FULL.contains(&path);
        }
        let Some(file) = res.get(&format!("assets/{ns}/models/{path}.json")) else {
            return false;
        };
        let Ok(v) = read_json(file) else { return false };
        if v.get("loader").is_some() {
            return false;
        }
        if let Some(elements) = v["elements"].as_array() {
            return elements.iter().any(|e| {
                let from: Vec<f64> = e["from"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect();
                let to: Vec<f64> = e["to"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect();
                from == [0.0, 0.0, 0.0] && to == [16.0, 16.0, 16.0]
            });
        }
        match v["parent"].as_str() {
            Some(p) => model = p.to_owned(),
            None => return false,
        }
    }
    false
}

/// Whether a block or item belongs on Create's building blocks tab.
fn palette(name: &str) -> bool {
    const STONES: &[&str] = &[
        "asurine", "crimsite", "limestone", "ochrum", "scorchia", "scoria", "veridium",
    ];
    STONES.contains(&name)
        || ["_slab", "_stairs", "_wall", "_pillar", "_window", "_window_pane", "_bricks"]
            .iter()
            .any(|s| name.ends_with(s))
        || ["cut_", "polished_cut_", "layered_", "small_"]
            .iter()
            .any(|p| name.starts_with(p))
        || ["framed_glass", "tiled_glass", "copper_shingle", "copper_tile", "rose_quartz_tiles"]
            .iter()
            .any(|p| name.contains(p))
}

fn creative_tabs(
    w: &mut Writer,
    java_blocks: &BTreeMap<String, java::JavaBlock>,
    java_items: &BTreeMap<String, java::JavaItem>,
    block_items: &BTreeSet<String>,
    items: &BTreeSet<String>,
) -> Result<(), String> {
    // Java's maps are sorted by name: registration order isn't kept, so Create's tab order
    // is approximated with kinetic parts first.
    let _ = (java_blocks, java_items);
    const FIRST: &[&str] = &[
        "wrench", "goggles", "shaft", "cogwheel", "large_cogwheel", "andesite_encased_shaft",
        "gearbox", "vertical_gearbox", "clutch", "gearshift", "encased_chain_drive",
        "adjustable_chain_gearshift", "belt_connector", "creative_motor", "water_wheel",
        "large_water_wheel", "windmill_bearing", "hand_crank", "millstone", "crushing_wheel",
        "mechanical_press", "mechanical_mixer", "basin", "depot", "andesite_alloy", "zinc_ingot",
        "brass_ingot", "andesite_casing", "brass_casing", "copper_casing",
    ];
    let mut all: Vec<String> = FIRST
        .iter()
        .filter(|n| block_items.contains(**n) || items.contains(**n))
        .map(|n| n.to_string())
        .collect();
    let rest = block_items
        .iter()
        .chain(items)
        .filter(|n| !FIRST.contains(&n.as_str()) && !n.ends_with("_placeable") && !n.starts_with("incomplete_"));
    all.extend(rest.cloned());
    let (palettes, base): (Vec<String>, Vec<String>) = all.into_iter().partition(|n| palette(n));
    let ids = |v: &[String]| v.iter().map(|n| format!("create:{n}")).collect::<Vec<_>>();
    w.json(
        "data/create/horizon/creative_tab/base.json",
        &json!({"title": "itemGroup.create.base", "icon": "create:cogwheel", "items": ids(&base)}),
    )?;
    w.json(
        "data/create/horizon/creative_tab/palettes.json",
        &json!({"title": "itemGroup.create.palettes", "icon": "create:polished_cut_andesite", "items": ids(&palettes)}),
    )
}

/// How the native mod places a block whose template doesn't orient it: the declared state
/// property it sets from the placement (`state_for_placement`).
fn placement_rule(declared: &[String], props: &blocks::Properties) -> Option<&'static str> {
    let own = |p: &str| declared.iter().any(|d| d == p);
    let values = |p: &str| props.get(p).cloned().unwrap_or_default();
    let has_value = |p: &str, v: &str| values(p).iter().any(|x| x == v);
    if own("face") && own("facing") {
        Some("face_facing")
    } else if own("facing") && has_value("facing", "up") {
        Some("facing")
    } else if own("facing") && !has_value("facing", "down") {
        Some("horizontal_facing")
    } else if own("axis") && has_value("axis", "y") {
        Some("axis")
    } else if own("axis") {
        Some("horizontal_axis")
    } else {
        // A basin or chute rests facing down until it is turned.
        None
    }
}

/// A block Create's wrench turns, and the orientation properties it turns.
struct Wrenchable {
    name: String,
    /// `facing` (six ways), `horizontal_facing` or empty.
    facing: &'static str,
    /// `axis` (three ways), `horizontal_axis` or empty.
    axis: &'static str,
    /// It has `axis_along_first` (`DirectionalAxisKineticBlock`).
    along_first: bool,
}

impl Wrenchable {
    fn of(name: &str, props: &blocks::Properties) -> Wrenchable {
        let values = |p: &str| props.get(p).cloned().unwrap_or_default();
        let facing = values("facing");
        let axis = values("axis");
        Wrenchable {
            name: name.to_owned(),
            facing: if facing.is_empty() {
                ""
            } else if facing.iter().any(|v| v == "up" || v == "down") {
                "facing"
            } else {
                "horizontal_facing"
            },
            axis: if axis.is_empty() {
                ""
            } else if axis.iter().any(|v| v == "y") {
                "axis"
            } else {
                "horizontal_axis"
            },
            along_first: props.contains_key("axis_along_first"),
        }
    }
}

/// The native mod's tables: every block's stress impact and capacity from `AllBlocks.java`,
/// how the blocks whose templates don't orient them are placed, and what the wrench turns.
fn write_native_table(
    path: &Path,
    java: &BTreeMap<String, java::JavaBlock>,
    placements: &[(String, &str)],
    wrenchable: &[Wrenchable],
) -> Result<(), String> {
    let mut s = String::from(
        "// @generated by create-horizon --native-table from Create's AllBlocks.java (MIT) and\n\
         // blockstates; do not edit by hand.\n\n\
         /// Stress impact and capacity per RPM, by block: (name, impact, capacity).\n\
         pub const STRESS: &[(&str, f64, f64)] = &[\n",
    );
    for (name, b) in java {
        if b.stress.impact.is_some() || b.stress.capacity.is_some() {
            let _ = writeln!(
                s,
                "    (\"{name}\", {:?}, {:?}),",
                b.stress.impact.unwrap_or(0.0),
                b.stress.capacity.unwrap_or(0.0)
            );
        }
    }
    s.push_str("];\n\n");
    s.push_str(
        "/// Blocks the native mod orients on placement, by rule: `facing` (six ways, against the\n\
         /// look), `horizontal_facing` (towards the placer), `axis` (the clicked face's),\n\
         /// `horizontal_axis` (the placer's) and `face_facing` (floor, wall or ceiling).\n\
         pub const PLACEMENT: &[(&str, &str)] = &[\n",
    );
    for (name, rule) in placements {
        let _ = writeln!(s, "    (\"{name}\", \"{rule}\"),");
    }
    s.push_str("];\n\n");
    s.push_str(
        "/// Blocks Create's wrench turns (their class is an `IWrenchable`), with the orientation\n\
         /// properties it turns: (name, facing kind, axis kind, has `axis_along_first`).\n\
         pub const WRENCHABLE: &[(&str, &str, &str, bool)] = &[\n",
    );
    for w in wrenchable {
        let _ = writeln!(
            s,
            "    (\"{}\", \"{}\", \"{}\", {}),",
            w.name, w.facing, w.axis, w.along_first
        );
    }
    s.push_str("];\n");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

fn render_report(r: &Report, files: usize, animated: bool) -> String {
    let mut s = String::from("# Create for Horizon: conversion report\n\n");
    let _ = writeln!(s, "{files} files written to `create/`.\n");
    let _ = writeln!(s, "| What | Count |\n|---|---|");
    for (k, v) in &r.counts {
        let _ = writeln!(s, "| {k} | {v} |");
    }
    let _ = writeln!(
        s,
        "\n## Create's recipe types\n\nLoaded as data: the game crafts none of them until the native mod runs the machine.\n\n| Type | Recipes |\n|---|---|"
    );
    for (k, v) in &r.recipe_types {
        let _ = writeln!(s, "| `{k}` | {v} |");
    }
    let _ = writeln!(s, "\n## Block templates\n\n| Template | Blocks |\n|---|---|");
    let mut t: Vec<_> = r.templates.iter().collect();
    t.sort_by(|a, b| b.1.cmp(a.1));
    for (k, v) in t {
        let _ = writeln!(s, "| `{k}` | {v} |");
    }
    let _ = writeln!(s, "\n## Item templates\n\n| Template | Items |\n|---|---|");
    for (k, v) in &r.item_templates {
        let _ = writeln!(s, "| `{k}` | {v} |");
    }
    if animated {
        let _ = writeln!(
            s,
            "\n`--animated`: {} are drawn by the native mod's block entities and are invisible without it.",
            ANIMATED.join(", ")
        );
    }
    for (heading, lines) in &r.lists {
        let _ = writeln!(s, "\n## {heading}\n");
        for l in lines {
            let _ = writeln!(s, "- {l}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_horizon_loads() {
        assert!(plain_path("assets/create/textures/block/shaft.png"));
        assert!(!plain_path("assets/create/textures/block/Shaft.png"));
        assert!(!plain_path("assets/create/../x"));
    }

    #[test]
    fn palette_blocks_go_on_their_tab() {
        assert!(palette("cut_andesite_slab"));
        assert!(palette("scoria"));
        assert!(palette("oak_window_pane"));
        assert!(!palette("shaft"));
        assert!(!palette("brass_casing"));
    }
}
