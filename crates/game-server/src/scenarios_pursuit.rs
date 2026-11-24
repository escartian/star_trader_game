//! Dev fixture for the pursuit capture: something to chase in the flagship's
//! system. Not part of play.
use super::{LOCAL_PLAYER, Session};
use game_core::{LocalPosition, travel::Location};

impl Session {
    /// Puts the first trader and the first raider band (backing off, so it
    /// does not strike first) in open space in the flagship's system, out of
    /// reach, and unlocks battles so Pursue shows.
    pub fn stage_pursuit_quarry(&mut self) -> Result<(), String> {
        let fleet = self.state.flagship(LOCAL_PLAYER).ok_or("No flagship")?;
        let record = self
            .state
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or("No flagship fleet")?;
        let lead = self
            .state
            .ship(*record.ships.first().ok_or("Empty flagship")?)
            .map_err(|error| error.to_string())?;
        let system = lead
            .location
            .system()
            .ok_or("The flagship is between systems")?;
        let spot = |x_km| LocalPosition {
            x_km,
            y_km: 0,
            z_km: 120_000_000,
        };
        let npc = self.economy.traders.first_mut().ok_or("No trader")?;
        npc.state.navigation.orders.clear();
        npc.state.ships[0].location = Location::InSystem {
            system,
            position: spot(-120_000_000),
        };
        let band = self
            .state
            .navigation
            .bands
            .first_mut()
            .ok_or("No raider band")?;
        band.system = system;
        band.transit = None;
        band.position = spot(120_000_000);
        band.heading = band.position;
        band.hunting = None;
        band.stalking = None;
        band.cooldown = game_core::navigation::LOOT_COOLDOWN_RUNS;
        self.state
            .progress
            .unlocked
            .push(game_core::progress::Feature::Battles);
        Ok(())
    }
}
