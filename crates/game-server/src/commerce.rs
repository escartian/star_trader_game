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
            }
            | ClientCommand::RepairShip {
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
            } => {
                let quote = economy.trade(
                    state.ship_mut(ship)?,
                    commodity,
                    quantity,
                    side,
                    expected_revision,
                )?;
                // Sales count toward unlocking trade routes.
                if side == game_core::economy::TradeSide::Sell {
                    state.progress.deliveries = state.progress.deliveries.saturating_add(1);
                }
                SessionEvent::Traded(quote)
            }
            ClientCommand::Refuel { units, .. } => {
                economy.refuel(state.ship_mut(ship)?, units)?;
                SessionEvent::Refuelled { units }
            }
            ClientCommand::UpgradeHold { .. } => {
                economy.upgrade_hold(state.ship_mut(ship)?)?;
                SessionEvent::HoldUpgraded
            }
            ClientCommand::RepairShip { .. } => {
                let cost = economy.repair(state.ship_mut(ship)?)?;
                SessionEvent::Repaired { ship, cost }
            }
            _ => unreachable!("command was validated above"),
        };
        state.revision = revision;
        self.state = state;
        self.economy = economy;
        Ok(self.snapshot(vec![event]))
    }
}
