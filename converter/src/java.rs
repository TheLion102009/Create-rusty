//! Reads what Create's Java registrations say about its blocks and items (`AllBlocks.java`,
//! `AllItems.java`): the things its data files don't carry, such as a block's material,
//! sounds, map colour, light, whether it occludes, and its stress values.
//!
//! This is a pattern reader, not a Java parser: each registration chain runs from
//! `.block("name"` / `.item("name"` to its `.register()`. Blocks registered in loops
//! (colours, stone palettes) have computed names and aren't found here; the converter
//! falls back to their names and blockstates for those.
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JavaBlock {
    /// `SharedProperties::stone` and friends, as the vanilla block they copy
    /// (`minecraft:andesite`), or `() -> Blocks.X`'s block.
    pub copies: Option<String>,
    /// `SoundType.X`, as the vanilla block whose sounds those are.
    pub sounds: Option<String>,
    /// `MapColor.X`, as its id.
    pub map_color: Option<u8>,
    pub no_occlusion: bool,
    pub light: Option<u8>,
    /// `.strength(hardness[, resistance])`.
    pub strength: Option<(f64, Option<f64>)>,
    pub requires_tool: bool,
    pub stress: Stress,
    /// The Java class its factory makes (`ShaftBlock::new`, `p -> new GearboxBlock(p)`).
    pub class: Option<String>,
}

/// `CStress.setImpact(x)` / `setCapacity(x)` / `setNoImpact()`, in stress units per RPM.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stress {
    pub impact: Option<f64>,
    pub capacity: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JavaItem {
    pub stacks_to: Option<u32>,
    pub rarity: Option<String>,
    /// Food: nutrition and saturation modifier.
    pub food: Option<(u32, f64)>,
    pub durability: Option<u32>,
    pub fire_resistant: bool,
}

/// Every `.block("name"` chain in `src`, by name.
pub fn blocks(src: &str) -> BTreeMap<String, JavaBlock> {
    chains(src, ".block(")
        .into_iter()
        .map(|(name, chain)| (name, block(&chain)))
        .collect()
}

/// Every `.item("name"` chain in `src`, by name.
pub fn items(src: &str) -> BTreeMap<String, JavaItem> {
    chains(src, ".item(")
        .into_iter()
        .map(|(name, chain)| (name, item(&chain)))
        .collect()
}

/// Registration chains: from each literal-named `<call>"name"` to the next `.register()`.
fn chains(src: &str, call: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(call) {
        let after = &rest[at + call.len()..];
        let end = after.find(".register()").unwrap_or(after.len());
        let chain = &after[..end];
        if let Some(name) = after.strip_prefix('"').and_then(|s| s.split('"').next())
            && is_id(name)
        {
            out.push((name.to_owned(), chain.to_owned()));
        }
        rest = &after[end.min(after.len())..];
        if rest.is_empty() {
            break;
        }
    }
    out
}

fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn block(chain: &str) -> JavaBlock {
    let copies = if let Some(kind) = after(chain, "initialProperties(SharedProperties::") {
        shared_properties(&word(kind))
    } else {
        after(chain, "initialProperties(() -> Blocks.")
            .map(|b| format!("minecraft:{}", word(b).to_lowercase()))
    };
    JavaBlock {
        copies,
        sounds: after(chain, "SoundType.").and_then(|s| sound_block(&word(s))),
        map_color: after(chain, "MapColor.").and_then(|s| map_color(&word(s))),
        no_occlusion: chain.contains("noOcclusion()"),
        light: light(chain),
        strength: after(chain, ".strength(").and_then(strength),
        requires_tool: chain.contains("requiresCorrectToolForDrops"),
        stress: Stress {
            impact: after(chain, "CStress.setImpact(").and_then(number),
            capacity: after(chain, "CStress.setCapacity(").and_then(number),
        },
        class: factory_class(chain),
    }
}

/// The class a registration's factory makes: the chain starts `"name", Factory::new)`.
fn factory_class(chain: &str) -> Option<String> {
    let factory = chain.split_once(',')?.1.trim_start();
    let class = if let Some(i) = factory.find("::").filter(|&i| !factory[..i].contains('(')) {
        word(&factory[..i])
    } else {
        word(after(factory, "new ")?)
    };
    (!class.is_empty()).then_some(class)
}

fn item(chain: &str) -> JavaItem {
    let food = after(chain, ".nutrition(").and_then(number).map(|n| {
        let saturation = after(chain, ".saturationModifier(")
            .and_then(number)
            .unwrap_or(0.6);
        (n as u32, saturation)
    });
    let durability = if let Some(armor) = after(chain, ".getDurability(") {
        // `Type.HELMET.getDurability(k)`: vanilla's per-slot base times k.
        let base = chain
            .split(".getDurability(")
            .next()
            .and_then(|s| s.rsplit("Type.").next())
            .map(word)
            .and_then(|slot| match slot.as_str() {
                "HELMET" => Some(11),
                "CHESTPLATE" => Some(16),
                "LEGGINGS" => Some(15),
                "BOOTS" => Some(13),
                _ => None,
            });
        base.zip(number(armor)).map(|(b, k)| b * k as u32)
    } else {
        after(chain, ".durability(").and_then(number).map(|n| n as u32)
    };
    JavaItem {
        stacks_to: after(chain, ".stacksTo(")
            .and_then(number)
            .map(|n| n as u32),
        rarity: after(chain, "Rarity.").map(|r| word(r).to_lowercase()),
        food,
        durability,
        fire_resistant: chain.contains("fireResistant()"),
    }
}

/// What follows the first `needle` in `s`.
fn after<'a>(s: &'a str, needle: &str) -> Option<&'a str> {
    s.find(needle).map(|i| &s[i + needle.len()..])
}

/// The identifier `s` starts with.
fn word(s: &str) -> String {
    s.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The number `s` starts with (`8.0`, `16`, `0.5f`).
fn number(s: &str) -> Option<f64> {
    let n: String = s
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    n.parse().ok()
}

fn strength(s: &str) -> Option<(f64, Option<f64>)> {
    let args = s.split(')').next()?;
    let mut parts = args.split(',').map(|p| number(p.trim()));
    let hardness = parts.next()??;
    let resistance = parts.next().flatten();
    Some((hardness, resistance))
}

/// `lightLevel(s -> 15)`, `lightLevel($ -> 12)`: a constant only.
fn light(chain: &str) -> Option<u8> {
    let rest = after(chain, "lightLevel(")?;
    let body = rest.split("->").nth(1)?;
    let n = number(body)?;
    (0.0..=15.0).contains(&n).then_some(n as u8)
}

/// `SharedProperties.java`: the vanilla block each kind copies.
fn shared_properties(kind: &str) -> Option<String> {
    Some(
        match kind {
            "wooden" => "minecraft:stripped_spruce_wood",
            "stone" => "minecraft:andesite",
            "softMetal" => "minecraft:gold_block",
            "copperMetal" => "minecraft:copper_block",
            "netheriteMetal" => "minecraft:netherite_block",
            _ => return None,
        }
        .to_owned(),
    )
}

/// A `SoundType` constant as a vanilla block that has it.
pub fn sound_block(sound: &str) -> Option<String> {
    Some(
        match sound {
            "WOOD" => "minecraft:oak_planks",
            "STONE" => "minecraft:stone",
            "METAL" => "minecraft:iron_block",
            "COPPER" => "minecraft:copper_block",
            "NETHERITE_BLOCK" => "minecraft:netherite_block",
            "GLASS" => "minecraft:glass",
            "WOOL" => "minecraft:white_wool",
            "SCAFFOLDING" => "minecraft:scaffolding",
            "DEEPSLATE" => "minecraft:deepslate",
            "CHISELED_BOOKSHELF" => "minecraft:chiseled_bookshelf",
            "CHAIN" => "minecraft:iron_chain",
            "LANTERN" => "minecraft:lantern",
            "AMETHYST" => "minecraft:amethyst_block",
            "HONEY_BLOCK" => "minecraft:honey_block",
            "TUFF" => "minecraft:tuff",
            "CALCITE" => "minecraft:calcite",
            "DRIPSTONE_BLOCK" => "minecraft:dripstone_block",
            "NETHER_BRICKS" => "minecraft:nether_bricks",
            "BASALT" => "minecraft:basalt",
            "COPPER_GRATE" => "minecraft:copper_grate",
            _ => return None,
        }
        .to_owned(),
    )
}

/// `MapColor` constants by their id (`MapColor.java`, 0..=61).
pub fn map_color(name: &str) -> Option<u8> {
    const NAMES: [&str; 62] = [
        "NONE",
        "GRASS",
        "SAND",
        "WOOL",
        "FIRE",
        "ICE",
        "METAL",
        "PLANT",
        "SNOW",
        "CLAY",
        "DIRT",
        "STONE",
        "WATER",
        "WOOD",
        "QUARTZ",
        "COLOR_ORANGE",
        "COLOR_MAGENTA",
        "COLOR_LIGHT_BLUE",
        "COLOR_YELLOW",
        "COLOR_LIGHT_GREEN",
        "COLOR_PINK",
        "COLOR_GRAY",
        "COLOR_LIGHT_GRAY",
        "COLOR_CYAN",
        "COLOR_PURPLE",
        "COLOR_BLUE",
        "COLOR_BROWN",
        "COLOR_GREEN",
        "COLOR_RED",
        "COLOR_BLACK",
        "GOLD",
        "DIAMOND",
        "LAPIS",
        "EMERALD",
        "PODZOL",
        "NETHER",
        "TERRACOTTA_WHITE",
        "TERRACOTTA_ORANGE",
        "TERRACOTTA_MAGENTA",
        "TERRACOTTA_LIGHT_BLUE",
        "TERRACOTTA_YELLOW",
        "TERRACOTTA_LIGHT_GREEN",
        "TERRACOTTA_PINK",
        "TERRACOTTA_GRAY",
        "TERRACOTTA_LIGHT_GRAY",
        "TERRACOTTA_CYAN",
        "TERRACOTTA_PURPLE",
        "TERRACOTTA_BLUE",
        "TERRACOTTA_BROWN",
        "TERRACOTTA_GREEN",
        "TERRACOTTA_RED",
        "TERRACOTTA_BLACK",
        "CRIMSON_NYLIUM",
        "CRIMSON_STEM",
        "CRIMSON_HYPHAE",
        "WARPED_NYLIUM",
        "WARPED_STEM",
        "WARPED_HYPHAE",
        "WARPED_WART_BLOCK",
        "DEEPSLATE",
        "RAW_IRON",
        "GLOW_LICHEN",
    ];
    NAMES.iter().position(|n| *n == name).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
	public static final BlockEntry<ShaftBlock> SHAFT = REGISTRATE.block("shaft", ShaftBlock::new)
		.initialProperties(SharedProperties::stone)
		.properties(p -> p.mapColor(MapColor.METAL).forceSolidOff())
		.transform(CStress.setNoImpact())
		.simpleItem()
		.register();
	public static final BlockEntry<MechanicalPressBlock> MECHANICAL_PRESS =
		REGISTRATE.block("mechanical_press", MechanicalPressBlock::new)
			.initialProperties(SharedProperties::stone)
			.properties(p -> p.noOcclusion()
				.mapColor(MapColor.PODZOL))
			.transform(CStress.setImpact(8.0))
			.register();
	public static final BlockEntry<Block> LAMP = REGISTRATE.block("lamp", Block::new)
		.initialProperties(() -> Blocks.GLOWSTONE)
		.properties(p -> p.sound(SoundType.GLASS).lightLevel(s -> 15).strength(0.5f, 6f))
		.register();
	public static final BlockEntry<Block> LOOPED = REGISTRATE.block(colour + "_seat", Block::new)
		.register();
"#;

    #[test]
    fn reads_block_chains() {
        let b = blocks(SRC);
        assert_eq!(b.len(), 3, "the looped registration has no literal name");
        let shaft = &b["shaft"];
        assert_eq!(shaft.copies.as_deref(), Some("minecraft:andesite"));
        assert_eq!(shaft.map_color, Some(6));
        assert!(!shaft.no_occlusion);
        let press = &b["mechanical_press"];
        assert!(press.no_occlusion);
        assert_eq!(press.map_color, Some(34));
        assert_eq!(press.stress.impact, Some(8.0));
        let lamp = &b["lamp"];
        assert_eq!(lamp.copies.as_deref(), Some("minecraft:glowstone"));
        assert_eq!(lamp.sounds.as_deref(), Some("minecraft:glass"));
        assert_eq!(lamp.light, Some(15));
        assert_eq!(lamp.strength, Some((0.5, Some(6.0))));
        assert_eq!(shaft.class.as_deref(), Some("ShaftBlock"));
        assert_eq!(press.class.as_deref(), Some("MechanicalPressBlock"));
    }

    #[test]
    fn reads_item_chains() {
        let src = r#"
	public static final ItemEntry<Item> SWEET_ROLL = REGISTRATE.item("sweet_roll", Item::new)
		.properties(p -> p.food(new FoodProperties.Builder().nutrition(6)
			.saturationModifier(0.8F).build()))
		.register();
	public static final ItemEntry<Item> HELMET = REGISTRATE.item("copper_diving_helmet", Item::new)
		.properties(p -> p.durability(Type.HELMET.getDurability(7)))
		.register();
	public static final ItemEntry<Item> WRENCH = REGISTRATE.item("wrench", Item::new)
		.properties(p -> p.stacksTo(1).rarity(Rarity.UNCOMMON))
		.register();
"#;
        let i = items(src);
        assert_eq!(i["sweet_roll"].food, Some((6, 0.8)));
        assert_eq!(i["copper_diving_helmet"].durability, Some(77));
        assert_eq!(i["wrench"].stacks_to, Some(1));
        assert_eq!(i["wrench"].rarity.as_deref(), Some("uncommon"));
    }
}
