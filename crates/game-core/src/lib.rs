//! Simulation types and rules. No graphics, networking, or filesystem access.
use serde::{Deserialize, Serialize};

pub mod economy;
pub mod fleet;
pub mod generation;
pub mod movement;
pub mod planet;
pub mod travel;

/// Stable identity; never use a display name or array index as an entity ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemId(pub u64);

/// Absolute galaxy coordinates in whole light-years.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalaxyPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// A destination ID scoped to its parent SystemId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyId(pub u64);

/// Position relative to the system's star, in kilometers; never galaxy units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPosition {
    pub x_km: i64,
    pub y_km: i64,
    pub z_km: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyKind {
    RockyPlanet,
    GasGiant,
    Moon,
    Asteroid,
    Station,
}

impl BodyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::RockyPlanet => "Rocky planet",
            Self::GasGiant => "Gas giant",
            Self::Moon => "Moon",
            Self::Asteroid => "Asteroid",
            Self::Station => "Trade station",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBody {
    pub id: BodyId,
    pub name: String,
    pub kind: BodyKind,
    pub position: LocalPosition,
    pub planet: Option<planet::PlanetProfile>,
}

/// Display classification; rendering sizes are symbolic, not physical scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarSystem {
    pub id: SystemId,
    pub name: String,
    pub star_kind: StarKind,
    pub position: GalaxyPosition,
    pub bodies: Vec<LocalBody>,
}

/// A fixed fixture for the first map prototype, not procedural generation.
pub fn demo_galaxy() -> Vec<StarSystem> {
    let mut systems = vec![
        StarSystem {
            id: SystemId(1),
            name: "Sol".into(),
            star_kind: StarKind::YellowDwarf,
            bodies: demo_bodies("Sol"),
            position: GalaxyPosition { x: 0, y: 0, z: 0 },
        },
        StarSystem {
            id: SystemId(2),
            name: "Meridian".into(),
            star_kind: StarKind::BlueGiant,
            bodies: demo_bodies("Meridian"),
            position: GalaxyPosition { x: 8, y: 4, z: 12 },
        },
        StarSystem {
            id: SystemId(3),
            name: "Far Reach".into(),
            star_kind: StarKind::RedDwarf,
            bodies: demo_bodies("Far Reach"),
            position: GalaxyPosition {
                x: -6,
                y: 10,
                z: -9,
            },
        },
    ];
    planet::PlanetCatalog::default().populate(&mut systems);
    systems
}

/// Fixed navigation destinations, not an orbital simulation or astronomical model.
fn demo_bodies(system_name: &str) -> Vec<LocalBody> {
    // IDs remain fixed even if display ordering changes or destinations are added.
    [
        (
            BodyId(1),
            "Inner world",
            BodyKind::RockyPlanet,
            45_000_000,
            8_000_000,
            -20_000_000,
        ),
        (
            BodyId(2),
            "Outer giant",
            BodyKind::GasGiant,
            -100_000_000,
            -15_000_000,
            80_000_000,
        ),
        (
            BodyId(3),
            "Trade port",
            BodyKind::Station,
            30_000_000,
            10_000_000,
            35_000_000,
        ),
        (
            BodyId(4),
            "Moon",
            BodyKind::Moon,
            46_000_000,
            8_500_000,
            -20_000_000,
        ),
        (
            BodyId(5),
            "Prospector asteroid",
            BodyKind::Asteroid,
            -60_000_000,
            12_000_000,
            -25_000_000,
        ),
    ]
    .into_iter()
    .map(|(id, name, kind, x_km, y_km, z_km)| LocalBody {
        planet: None,
        id,
        name: format!("{system_name} {name}"),
        kind,
        position: LocalPosition { x_km, y_km, z_km },
    })
    .collect()
}

#[cfg(test)]
mod planet_tests;
