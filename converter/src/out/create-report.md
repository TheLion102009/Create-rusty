# Create for Horizon: conversion report

8464 files written to `create/`.

| What | Count |
|---|---|
| asset files | 4812 |
| asset files left out (ponder scenes, Flywheel shaders, particles) | 205 |
| blocks | 641 |
| blocks with state properties of their own | 153 |
| data files left out (advancements, enchantments, data maps, damage types) | 1896 |
| files of other namespaces left out (Curios, Quark, NeoForge) | 63 |
| items | 99 |
| loot tables | 640 |
| recipes for other mods (left out) | 485 |
| recipes of Create's own types (data for the native mod) | 391 |
| recipes the game crafts | 1005 |
| tags | 183 |
| world generation (zinc ore) | 1 |

## Create's recipe types

Loaded as data: the game crafts none of them until the native mod runs the machine.

| Type | Recipes |
|---|---|
| `create:compacting` | 7 |
| `create:crushing` | 82 |
| `create:cutting` | 2 |
| `create:deploying` | 167 |
| `create:emptying` | 2 |
| `create:filling` | 10 |
| `create:haunting` | 20 |
| `create:item_application` | 8 |
| `create:item_copying` | 1 |
| `create:mechanical_crafting` | 4 |
| `create:milling` | 47 |
| `create:mixing` | 11 |
| `create:pressing` | 8 |
| `create:sandpaper_polishing` | 1 |
| `create:sequenced_assembly` | 3 |
| `create:splashing` | 17 |
| `create:toolbox_dyeing` | 1 |

## Block templates

| Template | Blocks |
|---|---|
| `minecraft:glass` | 149 |
| `minecraft:stone_slab` | 72 |
| `minecraft:stone_stairs` | 72 |
| `minecraft:lightning_rod` | 71 |
| `minecraft:stone` | 65 |
| `minecraft:cobblestone_wall` | 56 |
| `minecraft:white_carpet` | 35 |
| `minecraft:iron_chain` | 19 |
| `minecraft:glass_pane` | 18 |
| `minecraft:quartz_pillar` | 18 |
| `minecraft:cut_copper` | 16 |
| `minecraft:calcite` | 5 |
| `minecraft:copper_door` | 5 |
| `minecraft:dripstone_block` | 5 |
| `minecraft:iron_block` | 5 |
| `minecraft:polished_deepslate` | 5 |
| `minecraft:tuff` | 5 |
| `minecraft:gold_block` | 3 |
| `minecraft:iron_bars` | 3 |
| `minecraft:ladder` | 3 |
| `minecraft:quartz_block` | 3 |
| `minecraft:scaffolding` | 3 |
| `minecraft:copper_trapdoor` | 2 |
| `minecraft:deepslate_iron_ore` | 1 |
| `minecraft:iron_ore` | 1 |
| `minecraft:raw_iron_block` | 1 |

## Item templates

| Template | Items |
|---|---|
| `minecraft:apple` | 1 |
| `minecraft:bread` | 1 |
| `minecraft:cookie` | 2 |
| `minecraft:copper_boots` | 1 |
| `minecraft:copper_chestplate` | 1 |
| `minecraft:copper_helmet` | 1 |
| `minecraft:honey_bottle` | 1 |
| `minecraft:leather_boots` | 1 |
| `minecraft:leather_chestplate` | 1 |
| `minecraft:leather_helmet` | 1 |
| `minecraft:leather_leggings` | 1 |
| `minecraft:netherite_boots` | 1 |
| `minecraft:netherite_chestplate` | 1 |
| `minecraft:netherite_helmet` | 1 |
| `minecraft:stick` | 83 |
| `minecraft:wooden_sword` | 1 |

`--animated`: shaft, cogwheel, large_cogwheel are drawn by the native mod's block entities and are invisible without it.

## Items without a model

- `package`: its 1.21.1 model is drawn in code
- `rare_package`: its 1.21.1 model is drawn in code

## Left out

- block `chocolate`: a fluid; Horizon has no mod fluids yet
- block `honey`: a fluid; Horizon has no mod fluids yet
- `META-INF/accesstransformer.cfg`: not a path Horizon loads (upper case or a space)
- `assets/create/lang/README.md`: not a path Horizon loads (upper case or a space)
- feature `create:striated_ores_nether`: type `create:layered_ore` is Java code
- feature `create:striated_ores_overworld`: type `create:layered_ore` is Java code
