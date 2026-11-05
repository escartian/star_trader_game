use super::Session;
use game_core::{
    fleet::FleetError,
    travel::{PlayerId, TravelError},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn fleet_command(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let expected = match &command {
            ClientCommand::BuyShip {
                expected_revision, ..
            }
            | ClientCommand::SplitFleet {
                expected_revision, ..
            }
            | ClientCommand::MergeFleets {
                expected_revision, ..
            }
            | ClientCommand::TransferCargo {
                expected_revision, ..
            } => *expected_revision,
            _ => unreachable!("only fleet commands reach this handler"),
        };
        if expected != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let mut candidate = self.clone();
        let event = match command {
            ClientCommand::BuyShip { ship, kind, .. } => {
                candidate.state.authorize(player, ship)?;
                let spec = candidate
                    .economy
                    .rules
                    .ship_catalog
                    .ships
                    .iter()
                    .find(|spec| spec.kind == kind)
                    .ok_or(FleetError::UnknownClass)?
                    .clone();
                let credits = candidate
                    .economy
                    .credits
                    .0
                    .checked_sub(spec.price.0)
                    .ok_or(FleetError::InsufficientCredits)?;
                let fleet = candidate.state.fleet_for_ship(ship)?.id;
                let id = candidate.state.add_ship(player, fleet, &spec)?;
                candidate.economy.credits.0 = credits;
                SessionEvent::ShipPurchased { ship: id }
            }
            ClientCommand::SplitFleet { fleet, ships, .. } => {
                let new_fleet = candidate.state.split_fleet(player, fleet, &ships)?;
                SessionEvent::FleetSplit { fleet, new_fleet }
            }
            ClientCommand::MergeFleets { source, target, .. } => {
                candidate.state.merge_fleets(player, source, target)?;
                SessionEvent::FleetsMerged { source, target }
            }
            ClientCommand::TransferCargo {
                source,
                target,
                commodity,
                quantity,
                ..
            } => {
                candidate
                    .state
                    .transfer_cargo(player, source, target, commodity, quantity)?;
                SessionEvent::CargoTransferred {
                    source,
                    target,
                    quantity,
                }
            }
            _ => unreachable!("command was checked above"),
        };
        candidate.state.revision = revision;
        *self = candidate;
        Ok(self.snapshot(vec![event]))
    }
}
