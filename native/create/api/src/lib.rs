//! Create for Horizon: the native half. The content mod (`create.hmod`, written by the
//! `create-horizon` converter) brings Create's blocks, items and recipes; this trait mod makes
//! them work, piece by piece:
//!
//! - **Rotation.** Shafts, cogwheels, encased shafts and cogwheels, gearboxes, clutches,
//!   gearshifts and gauges pass rotation with Create's ratios; a creative motor drives them at
//!   16 RPM, a hand crank at 32 RPM while it is turned. Networks stop when two sources fight,
//!   when something would spin past 256 RPM, or when they take more stress than their sources
//!   give (`kinetics`, `parts`).
//! - **Redstone.** A powered clutch cuts its far side off, a powered gearshift turns it back.
//! - **Drawing.** The turning parts are drawn by their block entities, as Create's renderers
//!   draw them; the content mod must be built with `--animated` so the chunk mesh leaves the
//!   shafts and cogwheels out.
//! - **Placement and the wrench.** Blocks whose Horizon template doesn't orient them face the
//!   way Create places them (`placement`), and Create's wrench turns what Create's turns.
//!
//! Ported from Create (MIT, The Create Team) against Horizon's trait mod API.
mod kinetics;
mod parts;
mod placement;
mod tables;

use horizon_client::mod_api::{
    BlockChangedEvent, BlockEntityDraw, BlockEntityEvent, BlockEntitySubmitEvent,
    BlockEntityTickEvent, BlockEntityTicksEvent, BlockEntityUpdateTagEvent, BlockStateId,
    Compound, CubeFace, Flow, Hooks, Host, LevelDimension, LevelHost, ModsEvent, NativeMod,
    RightClickBlockEvent, StateForPlacementEvent, Tag, UseItemOnEvent,
};
use kinetics::{Dir, Node, Pos};
use parts::{Draw, Generator};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Create::default())
}

/// `CreativeMotorBlockEntity.DEFAULT_SPEED`.
const MOTOR_RPM: f32 = 16.0;
/// `HandCrankBlock.getRotationSpeed`.
const CRANK_RPM: f32 = 32.0;
/// `HandCrankBlockEntity.turn`: a click keeps it turning this many ticks.
const CRANK_TICKS: u32 = 10;
/// The block entity's saved speed, which clients draw with (`KineticBlockEntity`'s `Speed`).
const SPEED: &str = "Speed";
/// The side its rotation comes in by, which a gearbox's and a split shaft's halves turn by.
const SOURCE: &str = "Source";

#[derive(Default)]
struct Create {
    /// What the server knows of each dimension's kinetic blocks.
    levels: Mutex<HashMap<&'static str, Level>>,
}

#[derive(Default)]
struct Level {
    /// Where kinetic block entities ticked: the blocks to solve.
    known: HashSet<Pos>,
    /// Hand cranks being turned, with the ticks they have left.
    cranks: HashMap<Pos, u32>,
    /// The last solution.
    solution: kinetics::Solution,
    /// Something changed since the last solve.
    dirty: bool,
}

fn dimension_name(d: LevelDimension) -> &'static str {
    match d {
        LevelDimension::Overworld => "minecraft:overworld",
        LevelDimension::Nether => "minecraft:the_nether",
        LevelDimension::End => "minecraft:the_end",
    }
}

fn get<'a>(c: &'a Compound, key: &str) -> Option<&'a Tag> {
    c.iter().find(|(k, _)| **k == key.into()).map(|(_, v)| v)
}

fn float(c: &Compound, key: &str) -> Option<f32> {
    match get(c, key)? {
        Tag::Float(bits) => Some(f32::from_bits(*bits)),
        _ => None,
    }
}

fn set_float(c: &mut Compound, key: &str, value: f32) {
    c.insert(key.into(), Tag::Float(value.to_bits()));
}

fn direction(c: &Compound, key: &str) -> Option<Dir> {
    match get(c, key)? {
        Tag::String(s) => Dir::parse(&String::from_utf16_lossy(&s.0)),
        _ => None,
    }
}

fn set_direction(c: &mut Compound, key: &str, value: Option<Dir>) {
    match value {
        Some(d) => {
            c.insert(key.into(), Tag::String(d.name().into()));
        }
        None => {
            c.retain(|k, _| *k != key.into());
        }
    }
}

/// (impact, capacity) per RPM, from Create's `AllBlocks.java`.
fn stress(block: &str) -> (f64, f64) {
    let name = block.strip_prefix("create:").unwrap_or(block);
    tables::STRESS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, impact, capacity)| (*impact, *capacity))
        .unwrap_or((0.0, 0.0))
}

/// The kinetic node of the block at `pos`, read from the world; None for anything else.
fn read_node(host: &dyn Host, pos: Pos, cranking: bool) -> Option<Node> {
    let block = host.block(pos);
    let k = parts::kinetic(&block, &|key| host.block_property(pos, key))?;
    let (impact, capacity) = stress(&block);
    let generated = match (k.generator, k.part) {
        (Some(Generator::Motor), kinetics::Part::OneShaft { shaft }) => {
            kinetics::convert_to_direction(MOTOR_RPM, shaft)
        }
        (Some(Generator::Crank), kinetics::Part::OneShaft { shaft }) if cranking => {
            // The crank faces away from its shaft.
            kinetics::convert_to_direction(CRANK_RPM, shaft.opposite())
        }
        _ => 0.0,
    };
    let mut node = Node::new(k.part);
    node.generated = generated;
    node.impact = impact;
    node.capacity = capacity;
    Some(node)
}

fn lock<'a>(m: &'a Mutex<HashMap<&'static str, Level>>) -> std::sync::MutexGuard<'a, HashMap<&'static str, Level>> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Create {
    /// Reads every known kinetic block of this dimension and solves its networks.
    fn solve(&self, host: &dyn Host, dim: &'static str) {
        let (known, cranks) = {
            let levels = lock(&self.levels);
            let Some(level) = levels.get(dim) else { return };
            (level.known.clone(), level.cranks.clone())
        };
        let mut nodes = HashMap::new();
        for pos in known {
            if let Some(node) = read_node(host, pos, cranks.contains_key(&pos)) {
                nodes.insert(pos, node);
            }
        }
        let solution = kinetics::solve(&nodes);
        let mut levels = lock(&self.levels);
        let level = levels.entry(dim).or_default();
        level.known.retain(|p| nodes.contains_key(p));
        level.solution = solution;
    }

    /// Marks a block known and its dimension to solve again.
    fn touch(&self, dim: &'static str, pos: Pos) {
        let mut levels = lock(&self.levels);
        let level = levels.entry(dim).or_default();
        level.known.insert(pos);
        level.dirty = true;
    }
}

impl Hooks for Create {
    fn mods(&self, _host: &mut dyn Host, event: &mut ModsEvent<'_>) {
        event.mods.push(NativeMod {
            id: "create".into(),
            title: "Create".into(),
            description: "Create's shafts, cogwheels, gearboxes, clutches, motors and cranks (port in progress)".into(),
            keys: Vec::new(),
        });
    }

    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent<'_>) -> Option<(String, Compound)> {
        parts::is_kinetic(event.block).then(|| (event.block.to_owned(), Compound::new()))
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        if parts::is_kinetic(event.block) {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        let id = event.entity.id.clone();
        if !parts::is_kinetic(&id) {
            return;
        }
        let pos = event.pos;
        let dim = host.dimension();
        // `SplitShaftBlock.neighborChanged`: a clutch or gearshift follows its redstone.
        if id == "create:clutch" || id == "create:gearshift" {
            let want = if host.neighbor_signal(dim, pos) > 0 { "true" } else { "false" };
            if host.block_property(pos, "powered").as_deref() != Some(want)
                && host.set_block_property(pos, "powered", want)
            {
                self.touch(dim, pos);
            }
        }
        let solve = {
            let mut levels = lock(&self.levels);
            let level = levels.entry(dim).or_default();
            if level.known.insert(pos) {
                level.dirty = true;
            }
            if let Some(left) = level.cranks.get_mut(&pos) {
                *left = left.saturating_sub(1);
                if *left == 0 {
                    level.cranks.remove(&pos);
                    level.dirty = true;
                }
            }
            std::mem::take(&mut level.dirty)
        };
        if solve {
            self.solve(host, dim);
        }
        let (speed, source) = {
            let levels = lock(&self.levels);
            let s = levels.get(dim).map(|l| &l.solution);
            (
                s.and_then(|s| s.speed.get(&pos).copied()).unwrap_or(0.0),
                s.and_then(|s| s.source.get(&pos).copied()),
            )
        };
        let nbt = &event.entity.nbt;
        if float(nbt, SPEED) != Some(speed) || direction(nbt, SOURCE) != source {
            let nbt = &mut event.entity.nbt;
            set_float(nbt, SPEED, speed);
            set_direction(nbt, SOURCE, source);
            host.block_entity_sync(pos);
        }
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        if !parts::is_kinetic(&event.entity.id) {
            return None;
        }
        let mut tag = Compound::new();
        set_float(&mut tag, SPEED, float(&event.entity.nbt, SPEED).unwrap_or(0.0));
        set_direction(&mut tag, SOURCE, direction(&event.entity.nbt, SOURCE));
        Some(tag)
    }

    fn block_changed(&self, _level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let mut levels = lock(&self.levels);
        if let Some(level) = levels.get_mut(dimension_name(event.dimension))
            && level.known.remove(&pos)
        {
            // A kinetic block went or changed: a block entity placed in its stead
            // registers again on its first tick.
            level.cranks.remove(&pos);
            level.dirty = true;
        }
    }

    fn right_click_block(&self, host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        if host.block(event.pos) != "create:hand_crank" {
            return Flow::Continue;
        }
        let mut levels = lock(&self.levels);
        let level = levels.entry(host.dimension()).or_default();
        if level.cranks.insert(event.pos, CRANK_TICKS).is_none() {
            level.dirty = true;
        }
        Flow::Cancel
    }

    /// Create's wrench (`WrenchItem.useOn`, `IWrenchable.onWrenched`) turns the blocks Create
    /// turns. Taking a block up with a sneaking click needs the player's sneaking, which the
    /// hook doesn't tell yet.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if &*event.stack.item != "create:wrench" {
            return Flow::Continue;
        }
        let pos = event.pos;
        let block = host.block(pos);
        let Some(orientation) = parts::orientation(&block) else {
            return Flow::Continue;
        };
        let Some(face) = Dir::ALL.into_iter().find(|d| d.normal() == event.face) else {
            return Flow::Continue;
        };
        let changes = parts::wrenched(orientation, face, &|key| host.block_property(pos, key));
        let mut changed = false;
        for (key, value) in changes {
            changed |= host.set_block_property(pos, key, &value);
        }
        if changed && parts::is_kinetic(&block) {
            self.touch(host.dimension(), pos);
        }
        Flow::Cancel
    }

    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockStateId>> {
        let rule = placement::rule(&event.context.item)?;
        let n = event.context.face.normal();
        let clicked = Dir::ALL.into_iter().find(|d| d.normal() == n)?;
        let mut state = event.state;
        for (key, value) in placement::properties(rule, clicked, event.context.yaw, event.context.pitch) {
            if let Some(s) = level.state_with_property(state, key, value) {
                state = s;
            }
        }
        (state != event.state).then_some(Some(state))
    }

    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
        if !parts::is_kinetic(event.block) {
            return;
        }
        let state = event.state;
        let Some(k) = parts::kinetic(event.block, &|key| {
            host.block_state_property(state, key).map(str::to_owned)
        }) else {
            return;
        };
        let speed = float(&event.entity.nbt, SPEED).unwrap_or(0.0);
        let source = direction(&event.entity.nbt, SOURCE);
        let time = event.draw.game_time as f64 + event.draw.partial as f64;
        let pos = event.pos;
        let axis = k.part.axis();
        let draw = &mut *event.draw;
        let turned = |draw: &mut BlockEntityDraw, block: &str, large: bool| {
            let angle = kinetics::angle(time, speed, axis, pos, large);
            draw.push();
            draw.rotate_about(axis.unit(), angle, [0.5, 0.5, 0.5]);
            host.draw_block(draw, block, &[("axis", axis.name())]);
            draw.pop();
        };
        match k.draw {
            Draw::Model => turned(draw, event.block, k.part.large_cog()),
            Draw::Shaft => turned(draw, "create:shaft", false),
            Draw::Cog { large } => {
                let cog = if large { "create:large_cogwheel" } else { "create:cogwheel" };
                turned(draw, cog, large);
            }
            Draw::Halves => {
                for side in parts::halves(&k.part) {
                    // `GearboxRenderer`, `SplitShaftRenderer`: each half by its modifier.
                    let factor = k.part.axis_modifier(source, side);
                    let angle = kinetics::angle(time, speed * factor, side.axis(), pos, false);
                    half_shaft(host, draw, side, angle);
                }
            }
        }
    }
}

/// Create's `shaft_half`: a 4-pixel shaft from the block's centre out of `towards`, turned
/// by `angle` degrees about its axis.
fn half_shaft(host: &dyn Host, draw: &mut BlockEntityDraw, towards: Dir, angle: f32) {
    let centre = [0.5, 0.5, 0.5];
    draw.push();
    draw.rotate_about(towards.axis().unit(), angle, centre);
    // The part is modelled pointing up; turn it to `towards`.
    match towards {
        Dir::Up => {}
        Dir::Down => draw.rotate_about([1.0, 0.0, 0.0], 180.0, centre),
        Dir::North => draw.rotate_about([1.0, 0.0, 0.0], -90.0, centre),
        Dir::South => draw.rotate_about([1.0, 0.0, 0.0], 90.0, centre),
        Dir::East => draw.rotate_about([0.0, 0.0, 1.0], -90.0, centre),
        Dir::West => draw.rotate_about([0.0, 0.0, 1.0], 90.0, centre),
    }
    let side = || {
        Some(CubeFace {
            sprite: "create:block/axis",
            uv: Some([6.0, 0.0, 10.0, 8.0]),
        })
    };
    let end = Some(CubeFace {
        sprite: "create:block/axis_top",
        uv: Some([6.0, 6.0, 10.0, 10.0]),
    });
    // Faces in `Direction` order: down, up, north, south, west, east.
    host.draw_cube(
        draw,
        [6.0, 8.0, 6.0],
        [10.0, 16.0, 10.0],
        [None, end, side(), side(), side(), side()],
        0xFF_FF_FF,
    );
    draw.pop();
}

/// The hooks driven as the game drives them (`dispatch_*`), on a small world.
#[cfg(test)]
mod hook_tests {
    use super::*;
    use horizon_client::containers::{ModBlockEntity, ModBlockEntityFields};
    use horizon_client::items::ItemStack;
    use horizon_client::mod_api::{
        dispatch_block_entity, dispatch_block_entity_tick, dispatch_block_entity_update_tag,
        dispatch_right_click_block, dispatch_use_item_on,
    };

    /// A block and its state properties.
    type Block = (&'static str, Vec<(&'static str, String)>);
    /// A block placed in a test world: where, which, and its state properties.
    type Placed<'a> = (Pos, &'static str, &'a [(&'static str, &'static str)]);

    #[derive(Default)]
    struct World {
        blocks: HashMap<Pos, Block>,
        signals: HashMap<Pos, i32>,
        synced: Vec<Pos>,
    }

    impl Host for World {
        fn block(&self, pos: [i32; 3]) -> String {
            self.blocks.get(&pos).map_or("minecraft:air", |b| b.0).to_owned()
        }
        fn block_property(&self, pos: [i32; 3], key: &str) -> Option<String> {
            let (_, props) = self.blocks.get(&pos)?;
            props.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
        }
        fn set_block_property(&mut self, pos: [i32; 3], key: &str, value: &str) -> bool {
            let Some((_, props)) = self.blocks.get_mut(&pos) else { return false };
            let Some(p) = props.iter_mut().find(|(k, _)| *k == key) else { return false };
            if p.1 == value {
                return false;
            }
            p.1 = value.to_owned();
            true
        }
        fn neighbor_signal(&mut self, _dimension: &str, pos: [i32; 3]) -> i32 {
            self.signals.get(&pos).copied().unwrap_or(0)
        }
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }
        fn block_entity_sync(&mut self, pos: [i32; 3]) {
            self.synced.push(pos);
        }
    }

    struct Rig {
        world: World,
        mods: Vec<Box<dyn Hooks>>,
        entities: Vec<(Pos, ModBlockEntity)>,
    }

    impl Rig {
        fn new(blocks: &[Placed]) -> Rig {
            let mut world = World::default();
            let mods = vec![hooks()];
            let mut entities = Vec::new();
            for &(pos, block, props) in blocks {
                let props = props.iter().map(|(k, v)| (*k, v.to_string())).collect();
                world.blocks.insert(pos, (block, props));
                let mut made = None;
                dispatch_block_entity(&mut world, &mods, block, &mut made);
                let (id, nbt) = made.expect("a kinetic block gets a block entity");
                let fields = ModBlockEntityFields { id, nbt, ..Default::default() };
                entities.push((pos, ModBlockEntity::new(fields)));
            }
            Rig { world, mods, entities }
        }

        fn tick(&mut self, times: usize) {
            for _ in 0..times {
                for (pos, entity) in &mut self.entities {
                    dispatch_block_entity_tick(&mut self.world, &self.mods, *pos, entity);
                }
            }
        }

        fn tag(&self, pos: Pos) -> Compound {
            let (_, e) = self.entities.iter().find(|(p, _)| *p == pos).unwrap();
            let mut tag = None;
            dispatch_block_entity_update_tag(&self.mods, pos, e, &mut tag);
            tag.expect("clients are sent the speed")
        }

        fn speed(&self, pos: Pos) -> f32 {
            float(&self.tag(pos), SPEED).unwrap()
        }

        fn wrench(&mut self, pos: Pos, face: Pos) -> bool {
            let stack = ItemStack {
                item: "create:wrench".into(),
                count: 1,
                damage: 0,
                extra: None,
            };
            let mut claimed = false;
            dispatch_use_item_on(&mut self.world, &self.mods, &stack, pos, face, &mut claimed);
            claimed
        }
    }

    #[test]
    fn a_motor_drives_its_line_of_shafts() {
        let mut rig = Rig::new(&[
            ([0, 0, 0], "create:creative_motor", &[("facing", "east")]),
            ([1, 0, 0], "create:shaft", &[("axis", "x")]),
            ([2, 0, 0], "create:cogwheel", &[("axis", "x")]),
            ([2, 1, 0], "create:cogwheel", &[("axis", "x")]),
        ]);
        // The first tick registers each block entity; the next one sees them all.
        rig.tick(2);
        assert_eq!(rig.speed([1, 0, 0]), 16.0);
        assert_eq!(rig.speed([2, 0, 0]), 16.0);
        assert_eq!(rig.speed([2, 1, 0]), -16.0);
        assert!(rig.world.synced.contains(&[2, 1, 0]));
    }

    #[test]
    fn a_gearbox_tells_clients_where_its_rotation_comes_from() {
        let mut rig = Rig::new(&[
            ([0, 0, 0], "create:creative_motor", &[("facing", "east")]),
            ([1, 0, 0], "create:gearbox", &[("axis", "y")]),
            ([2, 0, 0], "create:andesite_encased_shaft", &[("axis", "x")]),
        ]);
        rig.tick(2);
        assert_eq!(direction(&rig.tag([1, 0, 0]), SOURCE), Some(Dir::West));
        assert_eq!(rig.speed([2, 0, 0]), -16.0);
    }

    #[test]
    fn a_clutch_follows_its_redstone() {
        let mut rig = Rig::new(&[
            ([0, 0, 0], "create:creative_motor", &[("facing", "east")]),
            ([1, 0, 0], "create:clutch", &[("axis", "x"), ("powered", "false")]),
            ([2, 0, 0], "create:shaft", &[("axis", "x")]),
        ]);
        rig.tick(2);
        assert_eq!(rig.speed([2, 0, 0]), 16.0);
        rig.world.signals.insert([1, 0, 0], 15);
        rig.tick(2);
        assert_eq!(rig.world.block_property([1, 0, 0], "powered").as_deref(), Some("true"));
        assert_eq!(rig.speed([2, 0, 0]), 0.0);
        rig.world.signals.clear();
        rig.tick(2);
        assert_eq!(rig.speed([2, 0, 0]), 16.0);
    }

    #[test]
    fn the_wrench_turns_a_shaft_out_of_the_line() {
        let mut rig = Rig::new(&[
            ([0, 0, 0], "create:creative_motor", &[("facing", "east")]),
            ([1, 0, 0], "create:shaft", &[("axis", "x")]),
        ]);
        rig.tick(2);
        assert_eq!(rig.speed([1, 0, 0]), 16.0);
        // Clicked on top: about y, x turns to z.
        assert!(rig.wrench([1, 0, 0], [0, 1, 0]));
        assert_eq!(rig.world.block_property([1, 0, 0], "axis").as_deref(), Some("z"));
        rig.tick(2);
        assert_eq!(rig.speed([1, 0, 0]), 0.0);
    }

    #[test]
    fn a_hand_crank_turns_while_it_is_used() {
        let mut rig = Rig::new(&[
            ([0, 1, 0], "create:hand_crank", &[("facing", "up")]),
            ([0, 0, 0], "create:shaft", &[("axis", "y")]),
        ]);
        rig.tick(2);
        assert_eq!(rig.speed([0, 0, 0]), 0.0);
        let mut canceled = false;
        dispatch_right_click_block(&mut rig.world, &rig.mods, [0, 1, 0], [0, 1, 0], &mut canceled);
        assert!(canceled, "the crank takes the click");
        rig.tick(2);
        assert_eq!(rig.speed([0, 0, 0]), 32.0);
        rig.tick(CRANK_TICKS as usize);
        assert_eq!(rig.speed([0, 0, 0]), 0.0);
    }

    #[test]
    fn other_blocks_are_left_alone() {
        let mut world = World::default();
        let mods = vec![hooks()];
        let mut made = None;
        dispatch_block_entity(&mut world, &mods, "minecraft:furnace", &mut made);
        assert!(made.is_none());
        world.blocks.insert([0, 0, 0], ("minecraft:lever", Vec::new()));
        let mut canceled = false;
        dispatch_right_click_block(&mut world, &mods, [0, 0, 0], [0, 1, 0], &mut canceled);
        assert!(!canceled);
        let stack = ItemStack { item: "create:wrench".into(), count: 1, damage: 0, extra: None };
        let mut claimed = false;
        dispatch_use_item_on(&mut world, &mods, &stack, [0, 0, 0], [0, 1, 0], &mut claimed);
        assert!(!claimed, "the wrench leaves vanilla blocks to vanilla");
    }
}
