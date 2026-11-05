use super::Session;
use game_core::travel::{PlayerId, TravelError};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn commerce(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let (ship, expected_revision) = match command {
            ClientCommand::Trade {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::Refuel {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::UpgradeHold {
                ship,
                expected_revision,
            } => (ship, expected_revision),
            _ => unreachable!("only commerce commands reach this handler"),
        };
        self.state.authorize(player, ship)?;
        if expected_revision != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        // A candidate transaction spans credits, ship fuel, cargo and market stock.
        // Publish it only after every rule succeeds, preserving failure atomicity.
        let mut state = self.state.clone();
        let mut economy = self.economy.clone();
        let event = match command {
            ClientCommand::Trade {
                commodity,
                quantity,
                side,
                ..
            } => SessionEvent::Traded(economy.trade(
                &state.ship,
                commodity,
                quantity,
                side,
                state.revision,
            )?),
            ClientCommand::Refuel { units, .. } => {
                economy.refuel(&mut state.ship, units)?;
                SessionEvent::Refuelled { units }
            }
            ClientCommand::UpgradeHold { .. } => {
                economy.upgrade_hold(&state.ship)?;
                SessionEvent::HoldUpgraded
            }
            _ => unreachable!("command was validated above"),
        };
        state.revision = revision;
        self.state = state;
        self.economy = economy;
        Ok(self.snapshot(vec![event]))
    }
}
