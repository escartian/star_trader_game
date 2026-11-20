//! Secondary fleets run themselves: only the flagship takes direct orders, and
//! the contacts other fleets meet are settled here without stopping an Advance.
//! Raider fights auto-resolve (each bounty is paid once); salvage is taken when
//! a hold has room; a distress call is answered when medicine is aboard;
//! anything else is bypassed.
use super::Session;
use game_core::{
    economy::{Commodity, Credits, PRICE_LEVEL_UNIT},
    fleet::FleetId,
    navigation::{EncounterChoice, EncounterKind, NavigationError, NavigationEvent},
    travel::{PlayerId, ShipId, TravelError},
};
use game_protocol::CommandError;

impl Session {
    /// Direct orders (courses, landing, lift-off, cancelling) go to the flagship only.
    pub(super) fn require_flagship(
        &self,
        player: PlayerId,
        fleet: FleetId,
    ) -> Result<(), CommandError> {
        if self.state.flagship(player) != Some(fleet) {
            return Err(NavigationError::NotFlagship.into());
        }
        Ok(())
    }

    /// The fleet `ship` belongs to.
    pub(super) fn fleet_holding(&self, ship: ShipId) -> Result<FleetId, CommandError> {
        // An unknown ship reports as such, before any fleet lookup.
        self.state.ship(ship)?;
        Ok(self.state.fleet_for_ship(ship)?.id)
    }

    /// Pays the bounty of a battle that ended since `paid`, once, at today's
    /// price level (the report records what was paid).
    pub(super) fn pay_new_bounty(&mut self, paid: &mut Option<u64>) -> Result<(), CommandError> {
        let level = self.economy.price_level_ppm;
        let Some(report) = self.state.navigation.last_combat.as_mut() else {
            return Ok(());
        };
        if Some(report.encounter) == *paid {
            return Ok(());
        }
        *paid = Some(report.encounter);
        let bounty = u128::from(report.bounty.0) * u128::from(level) / u128::from(PRICE_LEVEL_UNIT);
        report.bounty = Credits(bounty.min(u128::from(u64::MAX)) as u64);
        self.economy.credits = self
            .economy
            .credits
            .0
            .checked_add(report.bounty.0)
            .map(Credits)
            .ok_or(TravelError::TimeOverflow)?;
        Ok(())
    }

    /// Settles every pending contact of a secondary fleet; returns the outcomes.
    pub(super) fn settle_secondary_contacts(
        &mut self,
        player: PlayerId,
        paid: &mut Option<u64>,
    ) -> Result<Vec<NavigationEvent>, CommandError> {
        let flagship = self.state.flagship(player);
        let pending: Vec<_> = self
            .state
            .navigation
            .encounters
            .iter()
            .filter(|encounter| Some(encounter.fleet) != flagship)
            .cloned()
            .collect();
        let mut events = Vec::new();
        for encounter in pending {
            let choice = self.automatic_choice(encounter.fleet, encounter.kind);
            let event =
                self.state
                    .resolve_encounter(player, encounter.id, choice, &self.economy.rules)?;
            self.pay_new_bounty(paid)?;
            events.push(event);
        }
        Ok(events)
    }

    fn automatic_choice(&self, fleet: FleetId, kind: EncounterKind) -> EncounterChoice {
        let ships: Vec<_> = self
            .state
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .map(|record| {
                record
                    .ships
                    .iter()
                    .filter_map(|id| self.state.ship(*id).ok())
                    .collect()
            })
            .unwrap_or_default();
        match kind {
            EncounterKind::Raiders => EncounterChoice::AutoResolve,
            EncounterKind::DebrisField
                if ships
                    .iter()
                    .any(|ship| ship.cargo_used() < u64::from(ship.cargo_capacity)) =>
            {
                EncounterChoice::Investigate
            }
            EncounterKind::DistressSignal
                if ships
                    .iter()
                    .any(|ship| ship.cargo[Commodity::Medicine.index()] > 0) =>
            {
                EncounterChoice::Assist
            }
            _ => EncounterChoice::Bypass,
        }
    }
}
