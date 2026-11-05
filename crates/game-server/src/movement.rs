//! Host movement resolution. A future turn resolver uses this same planning path.
use super::Session;
use game_core::{
    movement::{MoveTarget, MovementPlan, MovementRules},
    travel::{PlayerId, ShipId, TravelError},
};
use game_protocol::{CommandError, ServerMessage, SessionEvent};

impl Session {
    pub fn set_movement_rules(&mut self, rules: MovementRules) -> Result<(), String> {
        if self.state.revision != 0 || self.state.tick != 0 {
            return Err("Movement configuration is fixed when a new session starts".into());
        }
        rules
            .validate_world(&self.systems)
            .map_err(|error| error.to_string())?;
        for ship in &self.state.ships {
            rules
                .validate_location(&ship.location, &self.systems)
                .map_err(|error| error.to_string())?;
        }
        self.state.movement_rules = rules;
        Ok(())
    }

    pub(super) fn preview_movement(
        &self,
        player: PlayerId,
        ship: ShipId,
        target: &MoveTarget,
    ) -> Result<MovementPlan, CommandError> {
        self.state
            .plan_movement(player, ship, target, &self.systems)
            .map_err(Into::into)
    }

    /// Host-only entry point. Remote clients submit orders to a turn collector,
    /// never call this directly. Offline input invokes it through handle_offline.
    pub fn resolve_movement_as(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        target: &MoveTarget,
        expected_revision: u64,
    ) -> ServerMessage {
        match self.resolve_movement(player, ship, target, expected_revision) {
            Ok(message) => message,
            Err(error) => ServerMessage::CommandRejected(error),
        }
    }

    fn resolve_movement(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        target: &MoveTarget,
        expected_revision: u64,
    ) -> Result<ServerMessage, CommandError> {
        self.state.authorize(player, ship)?;
        if expected_revision != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let plan = self.preview_movement(player, ship, target)?;
        let mut candidate = self.clone();
        candidate.state.resolve_movement(player, &plan)?;
        candidate.economy.advance(plan.duration_ticks);
        *self = candidate;
        Ok(self.snapshot(vec![SessionEvent::Moved(plan)]))
    }
}
