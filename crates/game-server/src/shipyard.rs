//! Shipyard operations run on the fleet handler's unpublished candidate session.
use super::Session;
use game_core::{
    fleet::{FleetError, ShipKind},
    travel::{PlayerId, ShipId},
};
use game_protocol::{CommandError, SessionEvent};

impl Session {
    pub(super) fn buy_ship(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        kind: ShipKind,
    ) -> Result<SessionEvent, CommandError> {
        self.state.authorize(player, ship)?;
        let selected = self.state.ship(ship)?.clone();
        let fleet = self.state.fleet_for_ship(ship)?.id;
        let spec = self.economy.buy_hull(&selected, kind)?;
        let id = self.state.add_ship(player, fleet, &spec)?;
        Ok(SessionEvent::ShipPurchased { ship: id })
    }

    pub(super) fn sell_ship(
        &mut self,
        player: PlayerId,
        ship: ShipId,
    ) -> Result<SessionEvent, CommandError> {
        self.state.authorize(player, ship)?;
        if self
            .state
            .ships
            .iter()
            .filter(|ship| ship.owner == player)
            .count()
            <= 1
        {
            return Err(FleetError::LastShip.into());
        }
        let selected = self.state.ship(ship)?.clone();
        let price = self.economy.sell_hull(&selected)?;
        let fleet = self.state.fleet_for_ship(ship)?.id;
        self.state.ships.retain(|record| record.id != ship);
        self.state
            .fleets
            .iter_mut()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?
            .ships
            .retain(|id| *id != ship);
        self.state.fleets.retain(|fleet| !fleet.ships.is_empty());
        Ok(SessionEvent::ShipSold { ship, price })
    }
}
