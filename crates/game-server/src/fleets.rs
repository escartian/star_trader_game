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
            | ClientCommand::SellShip {
                expected_revision, ..
            }
            | ClientCommand::RenameFleet {
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
        let structural = matches!(
            command,
            ClientCommand::BuyShip { .. }
                | ClientCommand::SellShip { .. }
                | ClientCommand::SplitFleet { .. }
                | ClientCommand::MergeFleets { .. }
        );
        match &command {
            ClientCommand::BuyShip { ship, .. } | ClientCommand::SellShip { ship, .. } => {
                candidate.state.authorize(player, *ship)?;
                candidate
                    .state
                    .guard_fleet_changes(player, candidate.state.fleet_for_ship(*ship)?.id)?;
            }
            ClientCommand::SplitFleet { fleet, .. } => {
                candidate.state.guard_fleet_changes(player, *fleet)?
            }
            ClientCommand::MergeFleets { source, target, .. } => {
                candidate.state.guard_fleet_changes(player, *source)?;
                candidate.state.guard_fleet_changes(player, *target)?;
            }
            _ => {}
        }
        let event = match command {
            ClientCommand::BuyShip { ship, kind, .. } => candidate.buy_ship(player, ship, kind)?,
            ClientCommand::SellShip {
                ship,
                discard_overflow,
                ..
            } => candidate.sell_ship(player, ship, discard_overflow)?,
            ClientCommand::RenameFleet { fleet, name, .. } => {
                let name = name.trim();
                if name.is_empty()
                    || name.chars().count() > 64
                    || name.len() > 128
                    || name.chars().any(char::is_control)
                {
                    return Err(FleetError::InvalidName.into());
                }
                let fleet_record = candidate
                    .state
                    .fleets
                    .iter_mut()
                    .find(|record| record.id == fleet)
                    .ok_or(FleetError::UnknownFleet)?;
                if fleet_record.owner != player {
                    return Err(FleetError::NotOwner.into());
                }
                fleet_record.name = name.into();
                SessionEvent::FleetRenamed { fleet }
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
        if structural {
            // Fleets created by a split start without orders; surviving fleets keep theirs.
            candidate.state.prune_standing_orders();
        }
        candidate.state.revision = revision;
        *self = candidate;
        Ok(self.snapshot(vec![event]))
    }
}
