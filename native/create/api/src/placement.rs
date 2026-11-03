//! How Create orients a block when it is placed, for the blocks whose Horizon template
//! doesn't (a machine on a glass template declares its own `facing`). The rules follow
//! Create's `getStateForPlacement` methods (MIT): `DirectionalKineticBlock` faces against
//! the look, `HorizontalKineticBlock` towards the placer, a pillar along the clicked face.
use crate::kinetics::Dir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Six ways, facing the placer (`getNearestLookingDirection().getOpposite()`).
    Facing,
    /// Four ways, facing the placer (`getHorizontalDirection().getOpposite()`).
    HorizontalFacing,
    /// Along the clicked face's axis.
    Axis,
    /// Along the placer's horizontal look.
    HorizontalAxis,
    /// On the floor, a wall or the ceiling, as a lever.
    FaceFacing,
}

impl Rule {
    pub fn parse(s: &str) -> Option<Rule> {
        Some(match s {
            "facing" => Rule::Facing,
            "horizontal_facing" => Rule::HorizontalFacing,
            "axis" => Rule::Axis,
            "horizontal_axis" => Rule::HorizontalAxis,
            "face_facing" => Rule::FaceFacing,
            _ => return None,
        })
    }
}

/// The rule for a Create block (`create:mechanical_press`), from the generated table.
pub fn rule(block: &str) -> Option<Rule> {
    let name = block.strip_prefix("create:")?;
    crate::tables::PLACEMENT
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, r)| Rule::parse(r))
}

/// `Direction.fromYRot`: the horizontal direction a yaw looks along.
pub fn horizontal(yaw: f32) -> Dir {
    const BY_DATA: [Dir; 4] = [Dir::South, Dir::West, Dir::North, Dir::East];
    let i = ((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4) as usize;
    BY_DATA[i]
}

/// `Direction.orderedByNearest(entity)[0]`: the direction a look is closest to.
pub fn looking(yaw: f32, pitch: f32) -> Dir {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    let look = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
    let (i, v) = look
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, v)| (i, *v))
        .unwrap_or((2, 1.0));
    match (i, v >= 0.0) {
        (0, true) => Dir::East,
        (0, false) => Dir::West,
        (1, true) => Dir::Up,
        (1, false) => Dir::Down,
        (_, true) => Dir::South,
        (_, false) => Dir::North,
    }
}

/// The properties to set on a placed block: `(name, value)` pairs.
pub fn properties(rule: Rule, clicked: Dir, yaw: f32, pitch: f32) -> Vec<(&'static str, &'static str)> {
    match rule {
        Rule::Facing => vec![("facing", looking(yaw, pitch).opposite().name())],
        Rule::HorizontalFacing => vec![("facing", horizontal(yaw).opposite().name())],
        Rule::Axis => vec![("axis", clicked.axis().name())],
        Rule::HorizontalAxis => vec![("axis", horizontal(yaw).axis().name())],
        Rule::FaceFacing => match clicked {
            Dir::Up => vec![("face", "floor"), ("facing", horizontal(yaw).name())],
            Dir::Down => vec![("face", "ceiling"), ("facing", horizontal(yaw).name())],
            side => vec![("face", "wall"), ("facing", side.name())],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_names_vanillas_directions() {
        assert_eq!(horizontal(0.0), Dir::South);
        assert_eq!(horizontal(90.0), Dir::West);
        assert_eq!(horizontal(180.0), Dir::North);
        assert_eq!(horizontal(-90.0), Dir::East);
        assert_eq!(horizontal(44.0), Dir::South);
        assert_eq!(horizontal(46.0), Dir::West);
    }

    #[test]
    fn looking_down_or_ahead() {
        assert_eq!(looking(0.0, 80.0), Dir::Down);
        assert_eq!(looking(0.0, -80.0), Dir::Up);
        assert_eq!(looking(180.0, 10.0), Dir::North);
    }

    #[test]
    fn a_machine_faces_whoever_places_it() {
        // Looking north at the spot: the press faces south, back at the player.
        let p = properties(Rule::HorizontalFacing, Dir::Up, 180.0, 30.0);
        assert_eq!(p, [("facing", "south")]);
        let p = properties(Rule::Facing, Dir::Up, 0.0, 80.0);
        assert_eq!(p, [("facing", "up")]);
    }

    #[test]
    fn pillars_follow_the_clicked_face_and_levers_the_wall() {
        assert_eq!(properties(Rule::Axis, Dir::East, 0.0, 0.0), [("axis", "x")]);
        assert_eq!(
            properties(Rule::FaceFacing, Dir::North, 0.0, 0.0),
            [("face", "wall"), ("facing", "north")]
        );
    }

    #[test]
    fn the_generated_table_names_create_blocks() {
        assert_eq!(rule("create:mechanical_press"), Some(Rule::HorizontalFacing));
        assert_eq!(rule("minecraft:stone"), None);
    }
}
