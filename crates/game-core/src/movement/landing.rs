//! Landing rules: which bodies a fleet may set down on, what landing and lifting
//! off cost, and what a landed fleet mines while it waits.
//!
//! - Rocky planets, moons and asteroids can be landed on; gas giants and stations
//!   cannot.
//! - Landing costs each ship the body's fuel (by gravity); lifting off costs the
//!   same again. A course plotted from the surface includes the lift-off.
//! - A fleet landed on a moon or asteroid with a Wait order mines ore every run:
//!   the yield per ship, limited by each ship's free cargo space.
use crate::BodyKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandingRules {
    /// Fuel per ship to land on, or lift off from, a rocky planet.
    pub rocky_planet_fuel: u32,
    pub moon_fuel: u32,
    pub asteroid_fuel: u32,
    /// Ore each landed ship mines per navigation run.
    pub moon_ore_per_ship: u32,
    pub asteroid_ore_per_ship: u32,
}

impl Default for LandingRules {
    fn default() -> Self {
        Self {
            rocky_planet_fuel: 2,
            moon_fuel: 1,
            asteroid_fuel: 0,
            moon_ore_per_ship: 1,
            asteroid_ore_per_ship: 2,
        }
    }
}

impl LandingRules {
    /// Fuel per ship to land on (or lift off from) `kind`; `None` when a fleet
    /// cannot land there.
    pub fn fuel(&self, kind: BodyKind) -> Option<u32> {
        match kind {
            BodyKind::RockyPlanet => Some(self.rocky_planet_fuel),
            BodyKind::Moon => Some(self.moon_fuel),
            BodyKind::Asteroid => Some(self.asteroid_fuel),
            BodyKind::GasGiant | BodyKind::Station => None,
        }
    }

    /// Ore per ship per run when landed on `kind` (0 where nothing is mined).
    pub fn ore_per_ship(&self, kind: BodyKind) -> u32 {
        match kind {
            BodyKind::Moon => self.moon_ore_per_ship,
            BodyKind::Asteroid => self.asteroid_ore_per_ship,
            _ => 0,
        }
    }

    pub(super) fn valid(&self) -> bool {
        [
            self.rocky_planet_fuel,
            self.moon_fuel,
            self.asteroid_fuel,
            self.moon_ore_per_ship,
            self.asteroid_ore_per_ship,
        ]
        .iter()
        .all(|value| *value <= 100)
    }
}
