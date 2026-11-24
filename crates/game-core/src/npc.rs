//! NPC commerce uses the same routes, hulls, cargo and finite-money rules as players.
mod band_raids;
mod decisions;
mod outfit;
mod population;
mod raids;
use crate::{
    StarSystem,
    economy::{Credits, Economy, MarketId},
    factions::Faction,
    fleet::FleetId,
    travel::{Location, PlayerId, ShipId, TravelState},
};
pub use population::{MAX_TRADERS, trader_count};
pub use raids::REFIT_RUNS;
use serde::{Deserialize, Serialize};

pub const NPC_ID_BASE: u64 = 1 << 63;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Personality {
    Cautious,
    Opportunist,
}
impl Personality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cautious => "Cautious trader",
            Self::Opportunist => "Opportunist",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trader {
    pub id: u64,
    pub name: String,
    pub personality: Personality,
    pub credits: Credits,
    pub state: TravelState,
    pub last_action: String,
    /// Runs left being refitted after raiders destroyed the ship (0 = trading).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub refit_runs: u32,
    /// Runs left on a hired escort contract (0: none; see `combat::gear`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub escort_runs: u32,
    /// The planet market it was commissioned at and is refitted at. Saves
    /// from before it was stored use the `id`-th planet market.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<MarketId>,
    /// The faction it flies for: the holder of its home port when factions
    /// were set up, or none (independent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<Faction>,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

impl Economy {
    pub fn validate_traders(&self, systems: &[StarSystem], run: u64) -> Result<(), String> {
        if self.traders.len() > MAX_TRADERS {
            return Err("Too many NPC traders".into());
        }
        let mut ids = std::collections::HashSet::new();
        for trader in &self.traders {
            let identity = NPC_ID_BASE
                .checked_add(trader.id)
                .ok_or("Invalid trader ID")?;
            if trader.id == 0
                || trader.id > MAX_TRADERS as u64
                || trader
                    .affiliation
                    .is_some_and(|faction| !self.factions.in_play.contains(&faction))
                || trader.home.is_some_and(|home| {
                    !self
                        .markets
                        .iter()
                        .any(|market| market.id == home && !market.is_station)
                })
                || !ids.insert(trader.id)
                || trader.credits.0 > 1_000_000_000_000_000
                || trader.name.is_empty()
                || trader.name.len() > 128
                || trader.last_action.len() > 256
                || trader.state.ships.len() != 1
                || trader.state.fleets.len() != 1
                || trader.state.navigation.run != run
                || trader.refit_runs > REFIT_RUNS
                || trader.escort_runs > crate::combat::gear::ESCORT_RUNS
            {
                return Err("Invalid NPC trader".into());
            }
            let ship = &trader.state.ships[0];
            let fleet = &trader.state.fleets[0];
            if ship.id != ShipId(identity)
                || ship.owner != PlayerId(identity)
                || fleet.id != FleetId(identity)
                || fleet.owner != ship.owner
                || fleet.ships != [ship.id]
                || ship.fuel_capacity == 0
                || ship.fuel > ship.fuel_capacity
                || ship.fuel_capacity > 1_000_000
                || ship.range_ly == 0
                || ship.range_ly > 1_000_000
                || !(1..=10_000).contains(&ship.cargo_capacity)
                || ship.cargo_used() > u64::from(ship.cargo_capacity)
                || ship.upgraded
                || !crate::combat::gear::valid_fit(&ship.gear)
                || ship.cargo_capacity != ship.base_cargo_capacity
                || !trader.state.navigation.encounters.is_empty()
                || self
                    .rules
                    .ship_catalog
                    .spec(ship.kind)
                    .is_none_or(|spec| ship.hull_damage >= spec.combat.hull)
            {
                return Err("Invalid NPC hull or fleet".into());
            }
            trader
                .state
                .movement_rules
                .validate_world(systems)
                .map_err(|error| error.to_string())?;
            trader
                .state
                .movement_rules
                .validate_location(&ship.location, systems)
                .map_err(|error| error.to_string())?;
            trader.state.validate_navigation(systems)?;
            if ship.location.stationary() && trader.state.navigation.orders.is_empty() {
                self.current_market(ship)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
}

pub fn market_location(market: MarketId, systems: &[StarSystem]) -> Location {
    if systems
        .iter()
        .find(|system| system.id == market.system)
        .and_then(|system| system.bodies.iter().find(|body| body.id == market.body))
        .is_some_and(|body| body.kind == crate::BodyKind::Station)
    {
        Location::Docked(market.system)
    } else {
        Location::Orbiting {
            system: market.system,
            body: market.body,
        }
    }
}
