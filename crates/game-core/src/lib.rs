//! Simulation types and rules. No graphics, networking, or filesystem access.
use serde::{Deserialize, Serialize};

pub mod approach;
pub mod campaign;
pub mod combat;
pub mod economy;
pub mod factions;
pub mod fleet;
pub mod fleet_trade;
pub mod generation;
pub mod jobs;
pub mod movement;
pub mod navigation;
pub mod npc;
pub mod orbits;
pub mod patrols;
pub mod piracy;
pub mod planet;
pub mod police;
pub mod progress;
pub mod pursuit;
pub mod routes;
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
    /// Where the body is now (placed for the current tick; see `orbits`).
    pub position: LocalPosition,
    pub planet: Option<planet::PlanetProfile>,
    /// Circular orbit; `None` keeps the body fixed (older saves).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit: Option<orbits::Orbit>,
    /// A moon's or asteroid's surface: biome, danger and ore richness, with
    /// no market (see `planet::SurfaceProfile`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<planet::SurfaceProfile>,
}

impl LocalBody {
    /// The body's biome, from its planet or surface profile.
    pub fn biome(&self) -> Option<planet::Biome> {
        self.planet
            .as_ref()
            .map(|profile| profile.biome)
            .or(self.surface.as_ref().map(|surface| surface.biome))
    }

    /// The body's danger rating, from its planet or surface profile.
    pub fn danger(&self) -> Option<planet::Danger> {
        self.planet
            .as_ref()
            .map(|profile| profile.danger)
            .or(self.surface.as_ref().map(|surface| surface.danger))
    }
}

/// Display classification; rendering sizes are symbolic, not physical scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StarKind {
    YellowDwarf,
    RedDwarf,
    BlueGiant,
    RedGiant,
    WhiteDwarf,
    OrangeDwarf,
    BlueWhiteStar,
    BrownDwarf,
    NeutronStar,
}

impl StarKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::YellowDwarf => "Yellow dwarf",
            Self::RedDwarf => "Red dwarf",
            Self::BlueGiant => "Blue giant",
            Self::RedGiant => "Red giant",
            Self::WhiteDwarf => "White dwarf",
            Self::OrangeDwarf => "Orange dwarf",
            Self::BlueWhiteStar => "Blue-white star",
            Self::BrownDwarf => "Brown dwarf",
            Self::NeutronStar => "Neutron star",
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
/// Space is flat: every system and body lies on the plane `y = 0`.
pub fn demo_galaxy() -> Vec<StarSystem> {
    let mut systems = vec![
        StarSystem {
            id: SystemId(1),
            name: "Sol".into(),
            star_kind: StarKind::YellowDwarf,
            bodies: demo_bodies(["Terra", "Jove", "Gateway Station", "Luna", "Ceres"]),
            position: GalaxyPosition { x: 0, y: 0, z: 0 },
        },
        StarSystem {
            id: SystemId(2),
            name: "Meridian".into(),
            star_kind: StarKind::BlueGiant,
            bodies: demo_bodies(["Calyx", "Thalassa", "Halden Exchange", "Pell", "Vosk"]),
            position: GalaxyPosition { x: 9, y: 0, z: 12 },
        },
        StarSystem {
            id: SystemId(3),
            name: "Far Reach".into(),
            star_kind: StarKind::RedDwarf,
            bodies: demo_bodies(["Brisa", "Tolmar", "Port Avar", "Kesh", "Saska"]),
            position: GalaxyPosition {
                x: -7,
                y: 0,
                z: -13,
            },
        },
    ];
    planet::PlanetCatalog::default().populate(&mut systems);
    systems
}

/// Fixed tick-0 destinations on circular orbits (see `orbits`), named in body
/// order: rocky planet, gas giant, station, moon (circling the rocky planet)
/// and asteroid; everything but the moon circles the star.
fn demo_bodies(names: [&str; 5]) -> Vec<LocalBody> {
    // IDs remain fixed even if display ordering changes or destinations are added.
    let mut bodies: Vec<LocalBody> = [
        (
            BodyId(1),
            names[0],
            BodyKind::RockyPlanet,
            45_000_000,
            0,
            -20_000_000,
        ),
        (
            BodyId(2),
            names[1],
            BodyKind::GasGiant,
            -100_000_000,
            0,
            80_000_000,
        ),
        (
            BodyId(3),
            names[2],
            BodyKind::Station,
            30_000_000,
            0,
            35_000_000,
        ),
        (
            BodyId(4),
            names[3],
            BodyKind::Moon,
            46_000_000,
            0,
            -20_000_000,
        ),
        (
            BodyId(5),
            names[4],
            BodyKind::Asteroid,
            -60_000_000,
            0,
            -25_000_000,
        ),
    ]
    .into_iter()
    .map(|(id, name, kind, x_km, y_km, z_km)| LocalBody {
        planet: None,
        orbit: None,
        surface: None,
        id,
        name: name.into(),
        kind,
        position: LocalPosition { x_km, y_km, z_km },
    })
    .collect();
    orbits::assign_orbits(&mut bodies, &[(BodyId(4), BodyId(1))]);
    bodies
}

#[cfg(test)]
mod planet_tests;
