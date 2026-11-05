//! Simulation types and rules. No graphics, networking, or filesystem access.

/// Stable identity; never use a display name or array index as an entity ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SystemId(pub u64);

/// Absolute galaxy coordinates. System-local coordinates will be a separate type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalaxyPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Display classification; rendering sizes are symbolic, not physical scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarKind {
    YellowDwarf,
    RedDwarf,
    BlueGiant,
    RedGiant,
    WhiteDwarf,
}

impl StarKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::YellowDwarf => "Yellow dwarf",
            Self::RedDwarf => "Red dwarf",
            Self::BlueGiant => "Blue giant",
            Self::RedGiant => "Red giant",
            Self::WhiteDwarf => "White dwarf",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StarSystem {
    pub id: SystemId,
    pub name: String,
    pub star_kind: StarKind,
    pub position: GalaxyPosition,
}

/// A fixed fixture for the first map prototype, not procedural generation.
pub fn demo_galaxy() -> Vec<StarSystem> {
    vec![
        StarSystem {
            id: SystemId(1),
            name: "Sol".into(),
            star_kind: StarKind::YellowDwarf,
            position: GalaxyPosition { x: 0, y: 0, z: 0 },
        },
        StarSystem {
            id: SystemId(2),
            name: "Meridian".into(),
            star_kind: StarKind::BlueGiant,
            position: GalaxyPosition { x: 8, y: 4, z: 12 },
        },
        StarSystem {
            id: SystemId(3),
            name: "Far Reach".into(),
            star_kind: StarKind::RedDwarf,
            position: GalaxyPosition {
                x: -6,
                y: 10,
                z: -9,
            },
        },
    ]
}
