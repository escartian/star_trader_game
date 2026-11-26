//! Who trades: the NPC population a new galaxy starts with.
//!
//! - One trader per [`SYSTEMS_PER_TRADER`] market systems, at least
//!   [`MIN_TRADERS`] and at most [`MAX_TRADERS`] (never more than the galaxy
//!   has planet markets times two, so ports are not crowded).
//! - Home ports are spread evenly over the planet markets in id order; the
//!   first trader is commissioned at the first planet market.
//! - Names come from fixed word lists, so the first two are always Aster Haul
//!   and Helix Courier, as in earlier versions.
//! - Personalities are dealt from the catalog's `spawn_cycle` (cautious, then
//!   opportunist first).
//! - A trader flies for the faction holding its home port, once factions are
//!   set up ([`Economy::affiliate_traders`]); in independent space it has none,
//!   and a loyalist there trades as a smuggler instead.
use super::{NPC_ID_BASE, PersonalityCatalog, Trader, market_location};
use crate::{
    StarSystem,
    economy::{Economy, MarketId},
    fleet::{Fleet, FleetId},
    movement::MovementRules,
    travel::{PlayerId, ShipId, TravelState},
};

/// Most NPC traders a galaxy (and a save) may hold.
pub const MAX_TRADERS: usize = 32;
/// Fewest NPC traders a galaxy with planet markets starts with.
pub const MIN_TRADERS: usize = 4;
/// Market systems per NPC trader in a new galaxy.
pub const SYSTEMS_PER_TRADER: usize = 3;

const NAME_FIRST: [&str; 16] = [
    "Aster", "Helix", "Corvid", "Lumen", "Tessera", "Vagrant", "Orrery", "Kestrel", "Saffron",
    "Meridian", "Quill", "Basalt", "Halcyon", "Ember", "Juniper", "Nadir",
];
const NAME_SECOND: [&str; 8] = [
    "Haul", "Courier", "Freight", "Venture", "Carrier", "Drifter", "Packet", "Line",
];

/// The `index`-th trader's name; unique for every index below [`MAX_TRADERS`].
pub fn trader_name(index: usize) -> String {
    let first = NAME_FIRST[index % NAME_FIRST.len()];
    // Each lap over the first names shifts the second names by one.
    let second = NAME_SECOND[(index + index / NAME_FIRST.len()) % NAME_SECOND.len()];
    format!("{first} {second}")
}

/// How many traders a galaxy starts with, given its market systems and
/// planet markets.
pub fn trader_count(market_systems: usize, planet_markets: usize) -> usize {
    if planet_markets == 0 {
        return 0;
    }
    (market_systems / SYSTEMS_PER_TRADER)
        .clamp(MIN_TRADERS, MAX_TRADERS)
        .min(planet_markets * 2)
}

impl Economy {
    pub fn spawn_traders(&mut self, systems: &[StarSystem], movement: &MovementRules) {
        self.traders.clear();
        let ports: Vec<MarketId> = self
            .markets
            .iter()
            .filter(|market| !market.is_station)
            .map(|market| market.id)
            .collect();
        let market_systems: std::collections::HashSet<_> = self
            .markets
            .iter()
            .map(|market| market.id.system.0)
            .collect();
        let count = trader_count(market_systems.len(), ports.len());
        for index in 0..count {
            let home = ports[index * ports.len() / count];
            let personality = self.rules.personality_catalog.dealt(index);
            let id = index as u64 + 1;
            let name = trader_name(index);
            let species = crate::species::Species::ALL[index % 3];
            let mut state = commissioned(id, &name, home, systems, movement);
            state.player_species = species;
            state.fleets[0].captain.species = species;
            self.traders.push(Trader {
                id,
                name,
                species,
                personality,
                credits: self.rules.npc_starting_credits,
                state,
                last_action: "Awaiting the next navigation run".into(),
                refit_runs: 0,
                escort_runs: 0,
                home: Some(home),
                affiliation: None,
                last_approach: 0,
            });
        }
    }

    /// Each trader flies for the faction holding its home port (independent
    /// space: none, and loyalists there turn smuggler). Run when factions are
    /// set up.
    pub fn affiliate_traders(&mut self) {
        for index in 0..self.traders.len() {
            let home = self.home_market(&self.traders[index]);
            let affiliation = home.and_then(|home| self.factions.owner(home.system));
            let trader = &mut self.traders[index];
            trader.affiliation = affiliation;
            if affiliation.is_none() {
                trader.personality = PersonalityCatalog::for_independent(trader.personality);
            }
        }
    }

    /// The planet market a trader is refitted at (see [`Trader::home`]).
    pub(crate) fn home_market(&self, trader: &Trader) -> Option<MarketId> {
        if trader.home.is_some() {
            return trader.home;
        }
        let index = usize::try_from(trader.id.checked_sub(1)?).ok()?;
        self.markets
            .iter()
            .filter(|market| !market.is_station)
            .nth(index)
            .map(|market| market.id)
    }
}

/// A trader's single-ship fleet, docked or in orbit at `home`, with its own
/// identity and random stream. NPC fleets never roll encounters.
fn commissioned(
    id: u64,
    name: &str,
    home: MarketId,
    systems: &[StarSystem],
    movement: &MovementRules,
) -> TravelState {
    let identity = NPC_ID_BASE + id;
    let mut state = TravelState {
        movement_rules: movement.clone(),
        ..TravelState::default()
    };
    state.movement_rules.encounter_chance_percent = 0;
    state.ships[0].id = ShipId(identity);
    state.ships[0].owner = PlayerId(identity);
    state.ships[0].name = name.into();
    state.ships[0].location = market_location(home, systems);
    state.fleets = vec![Fleet {
        id: FleetId(identity),
        name: name.into(),
        owner: PlayerId(identity),
        ships: vec![ShipId(identity)],
        captain: crate::species::FleetCaptain::default(),
    }];
    state.next_ship_id = identity + 1;
    state.next_fleet_id = identity + 1;
    state.navigation.rng_state = id;
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_keep_the_original_pair_first() {
        let names: std::collections::HashSet<_> = (0..MAX_TRADERS).map(trader_name).collect();
        assert_eq!(names.len(), MAX_TRADERS);
        assert_eq!(trader_name(0), "Aster Haul");
        assert_eq!(trader_name(1), "Helix Courier");
    }

    #[test]
    fn population_scales_with_markets_within_its_bounds() {
        assert_eq!(trader_count(3, 3), MIN_TRADERS);
        assert_eq!(trader_count(3, 1), 2);
        assert_eq!(trader_count(30, 40), 10);
        assert_eq!(trader_count(150, 200), MAX_TRADERS);
        assert_eq!(trader_count(5, 0), 0);
    }
}
