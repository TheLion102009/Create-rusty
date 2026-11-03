//! Create's kinetic blocks as this mod knows them: which `kinetics::Part` each block is in a
//! given state, how its block entity draws it, and how Create's wrench turns blocks.
//! Pure: a block's state comes in as a property lookup, so the server (reading the world)
//! and the client (reading a drawn state) share it.
use crate::kinetics::{Axis, Dir, Part, Split, clockwise};

/// How a kinetic block's block entity draws its turning parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    /// The whole block model turns (shaft, cogwheel): the content mod leaves it out of the
    /// chunk mesh (`--animated`).
    Model,
    /// The block's model stands (a casing, a gauge); a whole shaft turns through it.
    Shaft,
    /// A cogwheel turns inside a casing.
    Cog { large: bool },
    /// Half shafts turn out of the part's shaft sides, each by its axis modifier (a gearbox,
    /// a clutch, a motor).
    Halves,
}

/// What generates rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generator {
    /// `CreativeMotorBlockEntity`: 16 RPM out of its front.
    Motor,
    /// `HandCrankBlockEntity`: 32 RPM while turned, out of its back.
    Crank,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kinetic {
    pub part: Part,
    pub draw: Draw,
    pub generator: Option<Generator>,
}

/// The blocks this mod turns, by id: they get block entities.
pub const BLOCKS: &[&str] = &[
    "create:shaft",
    "create:cogwheel",
    "create:large_cogwheel",
    "create:andesite_encased_shaft",
    "create:brass_encased_shaft",
    "create:andesite_encased_cogwheel",
    "create:brass_encased_cogwheel",
    "create:andesite_encased_large_cogwheel",
    "create:brass_encased_large_cogwheel",
    "create:gearbox",
    "create:clutch",
    "create:gearshift",
    "create:speedometer",
    "create:stressometer",
    "create:creative_motor",
    "create:hand_crank",
];

pub fn is_kinetic(block: &str) -> bool {
    BLOCKS.contains(&block)
}

/// `DirectionalAxisKineticBlock.getRotationAxis`: a gauge's shaft runs across its facing.
pub fn directional_axis(facing: Dir, along_first: bool) -> Axis {
    match (facing.axis(), along_first) {
        (Axis::X, true) => Axis::Y,
        (Axis::X, false) => Axis::Z,
        (Axis::Y, true) => Axis::X,
        (Axis::Y, false) => Axis::Z,
        (Axis::Z, true) => Axis::X,
        (Axis::Z, false) => Axis::Y,
    }
}

/// The kinetic part of `block` in the state `prop` reads; None for a block this mod doesn't
/// turn or a state it can't read.
pub fn kinetic(block: &str, prop: &dyn Fn(&str) -> Option<String>) -> Option<Kinetic> {
    let axis = || prop("axis").as_deref().and_then(Axis::parse);
    let facing = || prop("facing").as_deref().and_then(Dir::parse);
    let flag = |key: &str| prop(key).as_deref() == Some("true");
    let k = |part, draw| Some(Kinetic { part, draw, generator: None });
    let name = block.strip_prefix("create:")?;
    match name {
        "shaft" => k(Part::Shaft { axis: axis()? }, Draw::Model),
        "cogwheel" => k(Part::cog(axis()?, false), Draw::Model),
        "large_cogwheel" => k(Part::cog(axis()?, true), Draw::Model),
        "andesite_encased_shaft" | "brass_encased_shaft" => k(Part::Shaft { axis: axis()? }, Draw::Shaft),
        "andesite_encased_cogwheel" | "brass_encased_cogwheel" => k(
            Part::Cog { axis: axis()?, large: false, shafts: [flag("bottom_shaft"), flag("top_shaft")] },
            Draw::Cog { large: false },
        ),
        "andesite_encased_large_cogwheel" | "brass_encased_large_cogwheel" => k(
            Part::Cog { axis: axis()?, large: true, shafts: [flag("bottom_shaft"), flag("top_shaft")] },
            Draw::Cog { large: true },
        ),
        "gearbox" => k(Part::Gearbox { axis: axis()? }, Draw::Halves),
        "clutch" | "gearshift" => {
            let kind = if name == "clutch" { Split::Clutch } else { Split::Gearshift };
            k(Part::Split { axis: axis()?, kind, powered: flag("powered") }, Draw::Halves)
        }
        "speedometer" | "stressometer" => {
            let axis = directional_axis(facing()?, flag("axis_along_first"));
            k(Part::Shaft { axis }, Draw::Shaft)
        }
        "creative_motor" => Some(Kinetic {
            part: Part::OneShaft { shaft: facing()? },
            draw: Draw::Halves,
            generator: Some(Generator::Motor),
        }),
        "hand_crank" => Some(Kinetic {
            part: Part::OneShaft { shaft: facing()?.opposite() },
            draw: Draw::Halves,
            generator: Some(Generator::Crank),
        }),
        _ => None,
    }
}

/// The sides a part's half shafts come out of (`Draw::Halves`).
pub fn halves(part: &Part) -> Vec<Dir> {
    Dir::ALL
        .into_iter()
        .filter(|d| part.has_shaft_towards(*d))
        .collect()
}

/// How Create's wrench orients a block (`tables::WRENCHABLE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orientation {
    pub facing: Option<Facing>,
    pub axis: Option<Facing>,
    pub along_first: bool,
}

/// Six ways or four (horizontal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    All,
    Horizontal,
}

pub fn orientation(block: &str) -> Option<Orientation> {
    let name = block.strip_prefix("create:")?;
    let (_, facing, axis, along_first) = crate::tables::WRENCHABLE.iter().find(|w| w.0 == name)?;
    let kind = |k: &str| match k {
        "facing" | "axis" => Some(Facing::All),
        "horizontal_facing" | "horizontal_axis" => Some(Facing::Horizontal),
        _ => None,
    };
    Some(Orientation {
        facing: kind(facing),
        axis: kind(axis),
        along_first: *along_first,
    })
}

/// `IWrenchable.getRotatedBlockState`: the properties a wrench click on `face` sets, from the
/// block's current `prop`erties. Empty when it doesn't turn.
pub fn wrenched(o: Orientation, face: Dir, prop: &dyn Fn(&str) -> Option<String>) -> Vec<(&'static str, String)> {
    let turn_axis = |a: Axis| {
        // `VoxelShaper.axisAsFace(axis).getClockWise(face axis).getAxis()`.
        let as_face = match a {
            Axis::X => Dir::East,
            Axis::Y => Dir::Up,
            Axis::Z => Dir::South,
        };
        clockwise(as_face, face.axis()).axis()
    };
    let flip = |v: Option<String>| if v.as_deref() == Some("true") { "false" } else { "true" };
    let axis = prop("axis").as_deref().and_then(Axis::parse);
    let facing = prop("facing").as_deref().and_then(Dir::parse);
    if face.axis() == Axis::Y {
        if let (Some(Facing::Horizontal), Some(a)) = (o.axis, axis) {
            return vec![("axis", turn_axis(a).name().to_owned())];
        }
        if let (Some(Facing::Horizontal), Some(f)) = (o.facing, facing) {
            return vec![("facing", clockwise(f, face.axis()).name().to_owned())];
        }
    }
    if let (Some(Facing::All), Some(a)) = (o.axis, axis) {
        return vec![("axis", turn_axis(a).name().to_owned())];
    }
    let (Some(Facing::All), Some(f)) = (o.facing, facing) else {
        return Vec::new();
    };
    if f.axis() == face.axis() {
        if o.along_first {
            return vec![("axis_along_first", flip(prop("axis_along_first")).to_owned())];
        }
        return Vec::new();
    }
    let mut new = f;
    let mut along = prop("axis_along_first");
    loop {
        new = clockwise(new, face.axis());
        if face.axis() == Axis::Y && o.along_first {
            along = Some(flip(along).to_owned());
        }
        if new.axis() != face.axis() {
            break;
        }
    }
    let mut out = vec![("facing", new.name().to_owned())];
    if o.along_first && face.axis() == Axis::Y {
        out.push(("axis_along_first", along.unwrap_or_default()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(p, _)| *p == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn blocks_become_parts() {
        let shaft = kinetic("create:shaft", &props(&[("axis", "x")])).unwrap();
        assert_eq!(shaft.part, Part::Shaft { axis: Axis::X });
        let cog = kinetic(
            "create:brass_encased_cogwheel",
            &props(&[("axis", "y"), ("top_shaft", "true"), ("bottom_shaft", "false")]),
        )
        .unwrap();
        assert_eq!(cog.part, Part::Cog { axis: Axis::Y, large: false, shafts: [false, true] });
        let crank = kinetic("create:hand_crank", &props(&[("facing", "up")])).unwrap();
        assert_eq!(crank.part, Part::OneShaft { shaft: Dir::Down });
        assert_eq!(crank.generator, Some(Generator::Crank));
        let gauge = kinetic("create:speedometer", &props(&[("facing", "north"), ("axis_along_first", "true")]));
        assert_eq!(gauge.unwrap().part, Part::Shaft { axis: Axis::X });
        assert!(kinetic("create:shaft", &props(&[])).is_none(), "no axis, no part");
        assert!(kinetic("minecraft:stone", &props(&[])).is_none());
        for b in BLOCKS {
            assert!(b.starts_with("create:"));
        }
    }

    #[test]
    fn a_gearbox_has_four_halves_a_motor_one() {
        assert_eq!(halves(&Part::Gearbox { axis: Axis::Y }).len(), 4);
        assert_eq!(halves(&Part::OneShaft { shaft: Dir::East }), [Dir::East]);
    }

    #[test]
    fn the_wrench_turns_axes_around_the_clicked_face() {
        let o = Orientation { facing: None, axis: Some(Facing::All), along_first: false };
        // A y shaft clicked on its side (north, z axis) lies down along x.
        let w = wrenched(o, Dir::North, &props(&[("axis", "y")]));
        assert_eq!(w, [("axis", "x".to_owned())]);
        // Clicked on its end it stays.
        assert_eq!(wrenched(o, Dir::Up, &props(&[("axis", "y")])), [("axis", "y".to_owned())]);
    }

    #[test]
    fn the_wrench_turns_horizontal_blocks_only_from_above() {
        let o = Orientation { facing: Some(Facing::Horizontal), axis: None, along_first: false };
        assert_eq!(wrenched(o, Dir::Up, &props(&[("facing", "north")])), [("facing", "east".to_owned())]);
        assert!(wrenched(o, Dir::North, &props(&[("facing", "north")])).is_empty());
    }

    #[test]
    fn the_wrench_skips_facing_along_the_clicked_axis() {
        let o = Orientation { facing: Some(Facing::All), axis: None, along_first: true };
        // Facing up, clicked on its north side: a quarter turn about z takes up to east.
        let w = wrenched(o, Dir::North, &props(&[("facing", "up"), ("axis_along_first", "false")]));
        assert_eq!(w, [("facing", "east".to_owned())]);
        // Clicked on its own axis it flips its shaft instead.
        let w = wrenched(o, Dir::Down, &props(&[("facing", "up"), ("axis_along_first", "false")]));
        assert_eq!(w, [("axis_along_first", "true".to_owned())]);
        // Around y a directional-axis block also flips its shaft each quarter turn.
        let w = wrenched(o, Dir::Up, &props(&[("facing", "north"), ("axis_along_first", "false")]));
        assert_eq!(w, [("facing", "east".to_owned()), ("axis_along_first", "true".to_owned())]);
    }

    #[test]
    fn create_blocks_have_orientations() {
        assert_eq!(orientation("create:gearbox").unwrap().axis, Some(Facing::All));
        assert_eq!(orientation("create:mechanical_press").unwrap().facing, Some(Facing::Horizontal));
        assert!(orientation("create:cut_andesite").is_none());
    }
}
