//! Host movement resolution. A future turn resolver uses this same planning path.
use super::Session;
use game_core::{
    movement::{MoveTarget, MovementPlan, MovementRules},
    travel::{PlayerId, ShipId},
};
use game_protocol::CommandError;

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
}
