//! Raider bands strike the NPC traders they stalk (see `navigation::raiders`).
//!
//! After a trader's own run, a band stalking it strikes if it is within strike
//! range of where the trader stood before the run (its heading: it caught the
//! trader before it got away) or of where the trader is now, unless the
//! trader has docked at a station.
//! A trader the band outguns as the player would need to (`piracy::would_yield`
//! thresholds) hands over half its cargo, rounded up, without a fight.
//! Otherwise the fight is the same auto-resolved battle as a random raid,
//! against the band's real ships: destroyed raiders leave the band, which then lies low
//! for `LOOT_COOLDOWN_RUNS` (longer than after fighting you). A struck trader is not also raided at
//! random that run. In a peaceful game (contact chance 0) bands never strike.
use super::Trader;
use crate::{
    StarSystem,
    combat::gear::ESCORT_KIND,
    economy::Economy,
    fleet::FleetMovementError,
    navigation::{Prey, STRIKE_KM, distance, local_in},
    piracy,
    travel::{Location, TravelState},
};

impl Economy {
    /// Traders bands may stalk: in service, carrying cargo (raiders want loot)
    /// and out in the open, where they stand now (`systems` placed at the
    /// current tick).
    pub fn prey(&self, systems: &[StarSystem]) -> Vec<Prey> {
        self.traders
            .iter()
            .filter(|trader| trader.refit_runs == 0)
            .filter_map(|trader| {
                let ship = &trader.state.ships[0];
                if ship.cargo_used() == 0 || matches!(ship.location, Location::Docked(_)) {
                    return None;
                }
                let position = trader.state.navigation_position(ship.id, systems)?;
                Some(Prey {
                    trader: trader.id,
                    position,
                })
            })
            .collect()
    }

    /// Lets a band stalking `trader` strike it; returns whether one did.
    pub(super) fn band_raid(
        &mut self,
        trader: &mut Trader,
        hunters: &mut TravelState,
        systems: &[StarSystem],
    ) -> Result<bool, FleetMovementError> {
        let ship = &trader.state.ships[0];
        if hunters.movement_rules.encounter_chance_percent == 0
            || matches!(ship.location, Location::Docked(_))
        {
            return Ok(false);
        }
        let Some(at) = trader.state.navigation_position(ship.id, systems) else {
            return Ok(false);
        };
        let half = hunters.movement_rules.system_half_extent_km;
        let striking = hunters.navigation.bands.iter().find(|band| {
            band.stalking == Some(trader.id)
                && systems
                    .iter()
                    .find(|world| world.id == band.system)
                    .and_then(|world| local_in(world, at, half))
                    .is_some_and(|local| distance(band.position, local) <= STRIKE_KM)
                || (band.stalking == Some(trader.id)
                    && distance(band.position, band.heading) <= STRIKE_KM)
        });
        let Some(band) = striking.cloned() else {
            return Ok(false);
        };
        let catalog = &self.rules.ship_catalog;
        let ours = piracy::strength(band.ships.iter().map(|kind| (*kind, 0)), catalog);
        let escort = (trader.escort_runs > 0).then_some((ESCORT_KIND, 0));
        let theirs = piracy::strength(
            std::iter::once((ship.kind, ship.hull_damage)).chain(escort),
            catalog,
        );
        let threshold = piracy::yield_percent(&self.rules.personality_catalog, trader.personality);
        if u128::from(ours) * 100 >= u128::from(theirs) * u128::from(threshold) {
            let cargo = &mut trader.state.ships[0].cargo;
            let mut left = cargo.iter().sum::<u32>().div_ceil(2);
            let handed = left;
            for units in cargo.iter_mut() {
                let taken = (*units).min(left);
                *units -= taken;
                left -= taken;
            }
            trader.last_action = format!("Handed {handed} cargo to a raider band");
            hunters.band_after_raid(band.id, &[]);
            return Ok(true);
        }
        let seed = trader.state.navigation.rolls();
        let destroyed = self.fight_raiders(trader, &band.ships, seed, "a raider band", systems)?;
        hunters.band_after_raid(band.id, &destroyed);
        Ok(true)
    }
}
