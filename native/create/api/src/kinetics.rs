//! Create's rotation: which kinetic blocks turn each other, at what ratio, and how much
//! stress a network carries. Ported from Create's `RotationPropagator.java`,
//! `KineticBlockEntity.java` and `KineticBlockEntityVisual.java` (MIT, The Create Team).
//!
//! Create spreads speed block entity by block entity as blocks change. This solves a whole
//! snapshot instead: every kinetic block the world has, its part and what it generates, in;
//! every block's speed and every network's stress, out. Same rules, simpler to test and to
//! drive from Horizon's hooks.
use std::collections::{HashMap, VecDeque};

pub type Pos = [i32; 3];

/// Create's speed limit (`maxRotationSpeed`, 256 RPM by default).
pub const MAX_SPEED: f32 = 256.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn index(self) -> usize {
        self as usize
    }
    /// `Axis.choose(x, y, z)`.
    pub fn choose(self, v: Pos) -> i32 {
        v[self.index()]
    }
    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }
    pub fn parse(s: &str) -> Option<Axis> {
        match s {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        }
    }
    pub fn unit(self) -> [f32; 3] {
        let mut v = [0.0; 3];
        v[self.index()] = 1.0;
        v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    pub const ALL: [Dir; 6] = [Dir::Down, Dir::Up, Dir::North, Dir::South, Dir::West, Dir::East];

    pub fn normal(self) -> Pos {
        match self {
            Dir::Down => [0, -1, 0],
            Dir::Up => [0, 1, 0],
            Dir::North => [0, 0, -1],
            Dir::South => [0, 0, 1],
            Dir::West => [-1, 0, 0],
            Dir::East => [1, 0, 0],
        }
    }
    pub fn axis(self) -> Axis {
        match self {
            Dir::Down | Dir::Up => Axis::Y,
            Dir::North | Dir::South => Axis::Z,
            Dir::West | Dir::East => Axis::X,
        }
    }
    pub fn positive(self) -> bool {
        matches!(self, Dir::Up | Dir::South | Dir::East)
    }
    pub fn opposite(self) -> Dir {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Dir::Down => "down",
            Dir::Up => "up",
            Dir::North => "north",
            Dir::South => "south",
            Dir::West => "west",
            Dir::East => "east",
        }
    }
    pub fn parse(s: &str) -> Option<Dir> {
        Dir::ALL.into_iter().find(|d| d.name() == s)
    }
    /// `Direction.getNearest(x, y, z)`: the direction closest to a non-zero offset.
    pub fn nearest(v: Pos) -> Dir {
        let mut best = Dir::North;
        let mut best_dot = i32::MIN;
        for d in Dir::ALL {
            let n = d.normal();
            let dot = n[0] * v[0] + n[1] * v[1] + n[2] * v[2];
            if dot > best_dot {
                best_dot = dot;
                best = d;
            }
        }
        best
    }
}

/// `KineticBlockEntity.convertToDirection`: a speed along a facing's axis.
pub fn convert_to_direction(speed: f32, facing: Dir) -> f32 {
    if facing.positive() { speed } else { -speed }
}


/// `Direction.getClockWise(Axis)`: a quarter turn about `axis`; directions along it stay.
pub fn clockwise(d: Dir, axis: Axis) -> Dir {
    if d.axis() == axis {
        return d;
    }
    match axis {
        Axis::Y => match d {
            Dir::North => Dir::East,
            Dir::East => Dir::South,
            Dir::South => Dir::West,
            _ => Dir::North,
        },
        Axis::X => match d {
            Dir::Down => Dir::South,
            Dir::Up => Dir::North,
            Dir::North => Dir::Down,
            _ => Dir::Up,
        },
        Axis::Z => match d {
            Dir::Down => Dir::West,
            Dir::Up => Dir::East,
            Dir::West => Dir::Up,
            _ => Dir::Down,
        },
    }
}

/// A split shaft's kind (`SplitShaftBlockEntity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
    /// Powered, it lets nothing through (`ClutchBlockEntity`).
    Clutch,
    /// Powered, it turns the far side the other way (`GearshiftBlockEntity`).
    Gearshift,
}

/// What a kinetic block is, as far as passing rotation goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part {
    /// A shaft, or any block with a shaft through both ends of its axis (an encased shaft, a
    /// gauge).
    Shaft { axis: Axis },
    /// A cogwheel, with a shaft out of its negative and positive ends (an encased cogwheel
    /// may lack either).
    Cog { axis: Axis, large: bool, shafts: [bool; 2] },
    /// A block with one shaft, out of `shaft` (a motor out of its front, a crank out of its back).
    OneShaft { shaft: Dir },
    /// A gearbox: shafts out of the four sides around `axis` (`GearboxBlock`).
    Gearbox { axis: Axis },
    /// A clutch or gearshift: a shaft in two halves along `axis`.
    Split { axis: Axis, kind: Split, powered: bool },
}

impl Part {
    /// A plain cogwheel, with its shaft through both ends.
    pub fn cog(axis: Axis, large: bool) -> Part {
        Part::Cog { axis, large, shafts: [true, true] }
    }
    /// `IRotate.getRotationAxis`.
    pub fn axis(&self) -> Axis {
        match *self {
            Part::Shaft { axis }
            | Part::Cog { axis, .. }
            | Part::Gearbox { axis }
            | Part::Split { axis, .. } => axis,
            Part::OneShaft { shaft } => shaft.axis(),
        }
    }
    /// `IRotate.hasShaftTowards`.
    pub fn has_shaft_towards(&self, face: Dir) -> bool {
        match *self {
            Part::Shaft { axis } | Part::Split { axis, .. } => face.axis() == axis,
            Part::Cog { axis, shafts, .. } => face.axis() == axis && shafts[usize::from(face.positive())],
            Part::OneShaft { shaft } => face == shaft,
            Part::Gearbox { axis } => face.axis() != axis,
        }
    }
    pub fn small_cog(&self) -> bool {
        matches!(self, Part::Cog { large: false, .. })
    }
    pub fn large_cog(&self) -> bool {
        matches!(self, Part::Cog { large: true, .. })
    }
    /// `RotationPropagator.getAxisModifier`: how a block driven from `source` (the side its
    /// rotation comes in by) turns what leaves by `direction`. 1 for a block without halves.
    pub fn axis_modifier(&self, source: Option<Dir>, direction: Dir) -> f32 {
        let Some(source) = source else { return 1.0 };
        match *self {
            Part::Gearbox { .. } => {
                if direction.axis() == source.axis() {
                    if direction == source { 1.0 } else { -1.0 }
                } else if direction.positive() == source.positive() {
                    -1.0
                } else {
                    1.0
                }
            }
            Part::Split { kind, powered, .. } if powered && direction != source => match kind {
                Split::Clutch => 0.0,
                Split::Gearshift => -1.0,
            },
            _ => 1.0,
        }
    }
    /// Offsets this part may pass rotation to: its six sides, and for cogwheels the
    /// diagonals (`getPotentialNeighbourLocations`, `addPropagationLocations`).
    pub fn neighbours(&self) -> Vec<Pos> {
        let mut out: Vec<Pos> = Dir::ALL.iter().map(|d| d.normal()).collect();
        if self.small_cog() || self.large_cog() {
            let axis = self.axis();
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let o = [x, y, z];
                        if x * x + y * y + z * z != 2 {
                            continue;
                        }
                        // A small cog meshes in its own plane; a large one also with
                        // large cogs turned the other way.
                        if self.small_cog() && axis.choose(o) != 0 {
                            continue;
                        }
                        out.push(o);
                    }
                }
            }
        }
        out
    }
}

/// `RotationPropagator.getRotationSpeedModifier`: the ratio `to` turns at for each turn of
/// `from`, `diff` being `to`'s position minus `from`'s, each driven from its `source` side
/// (None for a source, or a block not reached yet); 0 when they don't connect.
pub fn modifier(from: &Part, from_source: Option<Dir>, to: &Part, to_source: Option<Dir>, diff: Pos) -> f32 {
    let direction = Dir::nearest(diff);
    let aligned = [Axis::X, Axis::Y, Axis::Z]
        .iter()
        .all(|&a| a == direction.axis() || a.choose(diff) == 0);
    let connected_by_axis = aligned
        && from.has_shaft_towards(direction)
        && to.has_shaft_towards(direction.opposite());
    let connected_by_gears = from.small_cog() && to.small_cog();

    // Axis <-> Axis
    if connected_by_axis {
        let mut back = to.axis_modifier(to_source, direction.opposite());
        if back != 0.0 {
            back = 1.0 / back;
        }
        return from.axis_modifier(from_source, direction) * back;
    }
    // Large Gear <-> Large Gear
    if large_to_large(from, to, diff) {
        let source = from.axis().choose(diff);
        let target = to.axis().choose(diff);
        return if (source > 0) ^ (target > 0) { -1.0 } else { 1.0 };
    }
    // Gear <-> Large Gear
    if from.large_cog() && to.small_cog() && large_to_small(from, to, diff) {
        return -2.0;
    }
    if to.large_cog() && from.small_cog() && large_to_small(to, from, diff) {
        return -0.5;
    }
    // Gear <-> Gear
    if connected_by_gears {
        let manhattan: i32 = diff.iter().map(|v| v.abs()).sum();
        if manhattan != 1 || to.large_cog() || direction.axis() == from.axis() {
            return 0.0;
        }
        if from.axis() == to.axis() {
            return -1.0;
        }
    }
    0.0
}

fn large_to_large(from: &Part, to: &Part, diff: Pos) -> bool {
    if !from.large_cog() || !to.large_cog() || from.axis() == to.axis() {
        return false;
    }
    [Axis::X, Axis::Y, Axis::Z].iter().all(|&a| {
        let d = a.choose(diff);
        if a == from.axis() || a == to.axis() { d != 0 } else { d == 0 }
    })
}

fn large_to_small(large: &Part, small: &Part, diff: Pos) -> bool {
    let axis = large.axis();
    if axis != small.axis() || axis.choose(diff) != 0 {
        return false;
    }
    [Axis::X, Axis::Y, Axis::Z]
        .iter()
        .filter(|&&a| a != axis)
        .all(|&a| a.choose(diff).abs() == 1)
}

/// One kinetic block of a snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub part: Part,
    /// The speed it generates along its axis (`getGeneratedSpeed`), 0 for none.
    pub generated: f32,
    /// Stress per RPM it takes (`BlockStressValues.getImpact`).
    pub impact: f64,
    /// Stress per RPM it provides when it generates (`getCapacity`).
    pub capacity: f64,
}

impl Node {
    pub fn new(part: Part) -> Node {
        Node {
            part,
            generated: 0.0,
            impact: 0.0,
            capacity: 0.0,
        }
    }
}

/// Why a network stands still.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Two sources turn it opposite ways, or a loop of gears disagrees with itself: Create
    /// breaks one of the blocks; here the network stops until it is fixed.
    Conflict,
    /// A block would turn faster than [`MAX_SPEED`].
    Overspeed,
    /// It takes more stress than its sources give (`isOverStressed`).
    Overstressed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub members: Vec<Pos>,
    /// Stress units taken: impact times speed, summed.
    pub stress: f64,
    /// Stress units given: capacity times generated speed, summed.
    pub capacity: f64,
    pub stopped: Option<Stop>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solution {
    /// Every node's speed in RPM along its axis; 0 for a stopped or sourceless network.
    pub speed: HashMap<Pos, f32>,
    /// The side each driven block's rotation comes in by (`getSourceFacing`).
    pub source: HashMap<Pos, Dir>,
    /// Each driven block's network, an index into `networks`.
    pub network_of: HashMap<Pos, usize>,
    pub networks: Vec<Network>,
}

fn add(a: Pos, b: Pos) -> Pos {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Every node's speed and every network's stress. Each network is driven from its strongest
/// source outwards, as Create's propagation settles: each block remembers the side its
/// rotation came in by, which gearboxes and split shafts turn by.
pub fn solve(nodes: &HashMap<Pos, Node>) -> Solution {
    let mut solution = Solution::default();
    let mut sources: Vec<Pos> = nodes
        .iter()
        .filter(|(_, n)| n.generated != 0.0)
        .map(|(p, _)| *p)
        .collect();
    sources.sort_by(|a, b| {
        nodes[b].generated.abs().total_cmp(&nodes[a].generated.abs()).then(a.cmp(b))
    });
    // Speed and the side it came in by, per reached block.
    let mut reached: HashMap<Pos, (f32, Option<Dir>)> = HashMap::new();
    for &first in &sources {
        if reached.contains_key(&first) {
            continue;
        }
        let index = solution.networks.len();
        let mut members = vec![first];
        let mut stopped = None;
        reached.insert(first, (nodes[&first].generated, None));
        let mut queue = VecDeque::from([first]);
        while let Some(pos) = queue.pop_front() {
            let (speed, source) = reached[&pos];
            let part = nodes[&pos].part;
            for offset in part.neighbours() {
                let other = add(pos, offset);
                let Some(n) = nodes.get(&other) else { continue };
                let other_source = reached.get(&other).and_then(|r| r.1);
                let ratio = modifier(&part, source, &n.part, other_source, offset);
                if ratio == 0.0 {
                    continue;
                }
                let want = speed * ratio;
                match reached.get(&other) {
                    None => {
                        let back = Dir::nearest([-offset[0], -offset[1], -offset[2]]);
                        reached.insert(other, (want, Some(back)));
                        members.push(other);
                        queue.push_back(other);
                    }
                    Some(&(have, _)) if (have - want).abs() > 1e-3 => stopped = Some(Stop::Conflict),
                    Some(_) => {}
                }
            }
        }
        // A weaker source turning against the network fights it (`SpeedConflict`).
        for p in &members {
            let generated = nodes[p].generated;
            let have = reached[p].0;
            if generated != 0.0 && have != 0.0 && (have > 0.0) != (generated > 0.0) {
                stopped = Some(Stop::Conflict);
            }
        }
        if stopped.is_none() && members.iter().any(|p| reached[p].0.abs() > MAX_SPEED + 1e-3) {
            stopped = Some(Stop::Overspeed);
        }
        let capacity: f64 = members
            .iter()
            .map(|p| nodes[p].capacity * nodes[p].generated.abs() as f64)
            .sum();
        let stress: f64 = members
            .iter()
            .map(|p| nodes[p].impact * reached[p].0.abs() as f64)
            .sum();
        if stopped.is_none() && stress > capacity + 1e-9 {
            stopped = Some(Stop::Overstressed);
        }
        members.sort();
        for p in &members {
            let (speed, source) = reached[p];
            solution.speed.insert(*p, if stopped.is_some() { 0.0 } else { speed });
            if let Some(s) = source {
                solution.source.insert(*p, s);
            }
            solution.network_of.insert(*p, index);
        }
        solution.networks.push(Network {
            members,
            stress,
            capacity,
            stopped,
        });
    }
    for p in nodes.keys() {
        solution.speed.entry(*p).or_insert(0.0);
    }
    solution
}

/// `KineticBlockEntityVisual.rotationOffset`: neighbouring cogs turn half a tooth apart so
/// their teeth mesh.
pub fn rotation_offset(axis: Axis, pos: Pos, large: bool) -> f32 {
    let sum: i32 = [Axis::X, Axis::Y, Axis::Z]
        .iter()
        .filter(|&&a| a != axis)
        .map(|&a| a.choose(pos))
        .sum();
    if sum.rem_euclid(2) == 0 {
        22.5
    } else if large {
        11.25
    } else {
        0.0
    }
}

/// `KineticBlockEntityRenderer.getAngleForBe`, in degrees: `time` in ticks with the partial.
pub fn angle(time: f64, speed: f32, axis: Axis, pos: Pos, large: bool) -> f32 {
    let turned = (time * speed as f64 * 3.0 / 10.0).rem_euclid(360.0) as f32;
    (turned + rotation_offset(axis, pos, large)).rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaft(axis: Axis) -> Node {
        Node::new(Part::Shaft { axis })
    }
    fn cog(axis: Axis, large: bool) -> Node {
        Node::new(Part::cog(axis, large))
    }
    /// A creative motor facing `facing` at `rpm` (`CreativeMotorBlockEntity`).
    fn motor(facing: Dir, rpm: f32) -> Node {
        Node {
            part: Part::OneShaft { shaft: facing },
            generated: convert_to_direction(rpm, facing),
            impact: 0.0,
            capacity: 16384.0,
        }
    }

    #[test]
    fn a_motor_turns_the_shafts_in_front_of_it() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::East, 16.0)),
            ([1, 0, 0], shaft(Axis::X)),
            ([2, 0, 0], shaft(Axis::X)),
            // Not on the motor's axis: no shaft towards it.
            ([0, 1, 0], shaft(Axis::X)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.speed[&[1, 0, 0]], 16.0);
        assert_eq!(s.speed[&[2, 0, 0]], 16.0);
        assert_eq!(s.speed[&[0, 1, 0]], 0.0);
        // Only driven blocks form networks: the stray shaft has none.
        assert_eq!(s.networks.len(), 1);
        assert!(!s.network_of.contains_key(&[0, 1, 0]));
    }

    #[test]
    fn a_motor_facing_a_negative_direction_turns_negative() {
        let nodes = HashMap::from([([0, 0, 0], motor(Dir::West, 16.0)), ([-1, 0, 0], shaft(Axis::X))]);
        assert_eq!(solve(&nodes).speed[&[-1, 0, 0]], -16.0);
    }

    #[test]
    fn small_cogs_side_by_side_turn_opposite_ways() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::Up, 32.0)),
            ([0, 1, 0], cog(Axis::Y, false)),
            ([1, 1, 0], cog(Axis::Y, false)),
            ([2, 1, 0], cog(Axis::Y, false)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.speed[&[0, 1, 0]], 32.0);
        assert_eq!(s.speed[&[1, 1, 0]], -32.0);
        assert_eq!(s.speed[&[2, 1, 0]], 32.0);
    }

    #[test]
    fn a_large_cog_turns_a_small_one_twice_as_fast_and_back() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::Up, 16.0)),
            ([0, 1, 0], cog(Axis::Y, true)),
            ([1, 1, 1], cog(Axis::Y, false)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.speed[&[0, 1, 0]], 16.0);
        assert_eq!(s.speed[&[1, 1, 1]], -32.0);

        // Driven from the small side, the large one turns half as fast.
        let nodes = HashMap::from([
            ([1, 0, 1], motor(Dir::Up, 16.0)),
            ([1, 1, 1], cog(Axis::Y, false)),
            ([0, 1, 0], cog(Axis::Y, true)),
        ]);
        assert_eq!(solve(&nodes).speed[&[0, 1, 0]], -8.0);
    }

    #[test]
    fn large_cogs_mesh_at_right_angles() {
        let from = Part::cog(Axis::Y, true);
        let to = Part::cog(Axis::X, true);
        assert_eq!(modifier(&from, None, &to, None, [1, 1, 0]), 1.0);
        assert_eq!(modifier(&from, None, &to, None, [1, -1, 0]), -1.0);
        assert_eq!(modifier(&from, None, &to, None, [0, 1, 1]), 0.0);
    }

    #[test]
    fn small_cogs_dont_mesh_along_their_axis_or_across_axes() {
        let a = Part::cog(Axis::Y, false);
        // Stacked on the axis they connect by their shafts, not their teeth.
        assert_eq!(modifier(&a, None, &a, None, [0, 1, 0]), 1.0);
        let b = Part::cog(Axis::X, false);
        assert_eq!(modifier(&a, None, &b, None, [1, 0, 0]), 0.0);
    }

    #[test]
    fn opposing_motors_conflict() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::East, 16.0)),
            ([1, 0, 0], shaft(Axis::X)),
            ([2, 0, 0], motor(Dir::West, 16.0)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.networks[0].stopped, Some(Stop::Conflict));
        assert!(s.speed.values().all(|v| *v == 0.0));
    }

    #[test]
    fn motors_turning_the_same_way_share_a_network() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::East, 16.0)),
            ([1, 0, 0], shaft(Axis::X)),
            // Facing west, generating -16 along x... which is +16 seen from the east.
            ([2, 0, 0], Node { generated: 16.0, ..motor(Dir::West, 16.0) }),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.networks[0].stopped, None);
        assert_eq!(s.speed[&[1, 0, 0]], 16.0);
    }

    #[test]
    fn a_crank_overstressed_by_two_presses() {
        // A hand crank: 8 SU per RPM at 32 RPM is 256 SU; a press takes 8 per RPM.
        let crank = Node {
            part: Part::OneShaft { shaft: Dir::Down },
            generated: convert_to_direction(32.0, Dir::Up),
            impact: 0.0,
            capacity: 8.0,
        };
        let press = Node { impact: 8.0, ..shaft(Axis::Y) };
        let one = HashMap::from([([0, 1, 0], crank), ([0, 0, 0], press)]);
        let s = solve(&one);
        assert_eq!(s.networks[0].capacity, 256.0);
        assert_eq!(s.networks[0].stress, 256.0);
        assert_eq!(s.networks[0].stopped, None);
        let two = HashMap::from([([0, 1, 0], crank), ([0, 0, 0], press), ([0, -1, 0], press)]);
        let s = solve(&two);
        assert_eq!(s.networks[0].stopped, Some(Stop::Overstressed));
        assert_eq!(s.speed[&[0, 0, 0]], 0.0);
    }

    #[test]
    fn too_fast_stops_the_network() {
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::Up, 256.0)),
            ([0, 1, 0], cog(Axis::Y, true)),
            ([1, 1, 1], cog(Axis::Y, false)),
        ]);
        assert_eq!(solve(&nodes).networks[0].stopped, Some(Stop::Overspeed));
    }

    #[test]
    fn neighbouring_cogs_turn_half_a_tooth_apart() {
        assert_eq!(rotation_offset(Axis::Y, [0, 0, 0], false), 22.5);
        assert_eq!(rotation_offset(Axis::Y, [1, 0, 0], false), 0.0);
        assert_eq!(rotation_offset(Axis::Y, [1, 7, 0], true), 11.25);
        // The axis's own coordinate doesn't count.
        assert_eq!(rotation_offset(Axis::Y, [0, 1, 0], false), 22.5);
    }

    #[test]
    fn angle_turns_with_time_and_speed() {
        // 16 RPM: 16 * 360 degrees a minute, 1200 ticks: 4.8 degrees a tick.
        let a = angle(10.0, 16.0, Axis::Y, [1, 0, 0], false);
        assert!((a - 48.0).abs() < 1e-3);
        assert!(angle(1e9, -256.0, Axis::Y, [1, 0, 0], false) >= 0.0);
    }

    fn gearbox(axis: Axis) -> Node {
        Node::new(Part::Gearbox { axis })
    }
    fn split(axis: Axis, kind: Split, powered: bool) -> Node {
        Node::new(Part::Split { axis, kind, powered })
    }

    #[test]
    fn a_gearbox_turns_straight_through_backwards_and_round_corners_by_side() {
        // Driven from the west along x; the gearbox's own axis is y (no shafts up or down).
        let nodes = HashMap::from([
            ([-2, 0, 0], motor(Dir::East, 16.0)),
            ([-1, 0, 0], shaft(Axis::X)),
            ([0, 0, 0], gearbox(Axis::Y)),
            ([1, 0, 0], shaft(Axis::X)),
            ([0, 0, 1], shaft(Axis::Z)),
            ([0, 0, -1], shaft(Axis::Z)),
            ([0, 1, 0], shaft(Axis::Y)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.source[&[0, 0, 0]], Dir::West);
        assert_eq!(s.speed[&[1, 0, 0]], -16.0, "straight through turns back");
        // Round a corner: the same sign of direction as the source turns back (north, like
        // west, is negative), the other keeps (south).
        assert_eq!(s.speed[&[0, 0, 1]], 16.0);
        assert_eq!(s.speed[&[0, 0, -1]], -16.0);
        assert_eq!(s.speed[&[0, 1, 0]], 0.0, "no shaft out of its axis");
    }

    #[test]
    fn a_powered_clutch_cuts_and_a_powered_gearshift_reverses() {
        let line = |middle: Node| {
            HashMap::from([
                ([0, 0, 0], motor(Dir::East, 16.0)),
                ([1, 0, 0], middle),
                ([2, 0, 0], shaft(Axis::X)),
            ])
        };
        let off = solve(&line(split(Axis::X, Split::Clutch, false)));
        assert_eq!(off.speed[&[2, 0, 0]], 16.0);
        let on = solve(&line(split(Axis::X, Split::Clutch, true)));
        assert_eq!(on.speed[&[1, 0, 0]], 16.0, "the driven half still turns");
        assert_eq!(on.speed[&[2, 0, 0]], 0.0);
        let shifted = solve(&line(split(Axis::X, Split::Gearshift, true)));
        assert_eq!(shifted.speed[&[2, 0, 0]], -16.0);
    }

    #[test]
    fn an_encased_cog_takes_shafts_only_on_its_open_ends() {
        let encased = Node::new(Part::Cog { axis: Axis::X, large: false, shafts: [false, true] });
        let nodes = HashMap::from([
            ([0, 0, 0], motor(Dir::East, 16.0)),
            ([1, 0, 0], encased),
            ([2, 0, 0], shaft(Axis::X)),
        ]);
        let s = solve(&nodes);
        assert_eq!(s.speed[&[1, 0, 0]], 0.0, "closed on the motor's side");
        let open = Node::new(Part::Cog { axis: Axis::X, large: false, shafts: [true, false] });
        let nodes = HashMap::from([([0, 0, 0], motor(Dir::East, 16.0)), ([1, 0, 0], open), ([1, 1, 0], cog(Axis::X, false))]);
        let s = solve(&nodes);
        assert_eq!(s.speed[&[1, 0, 0]], 16.0);
        assert_eq!(s.speed[&[1, 1, 0]], -16.0, "it still meshes with cogs");
    }

    #[test]
    fn quarter_turns_follow_vanilla() {
        assert_eq!(clockwise(Dir::North, Axis::Y), Dir::East);
        assert_eq!(clockwise(Dir::Up, Axis::X), Dir::North);
        assert_eq!(clockwise(Dir::Up, Axis::Z), Dir::East);
        assert_eq!(clockwise(Dir::East, Axis::X), Dir::East);
        // Four quarter turns come back round.
        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for d in Dir::ALL {
                let mut r = d;
                for _ in 0..4 {
                    r = clockwise(r, axis);
                }
                assert_eq!(r, d);
            }
        }
    }
}
