//! Offline movement resolves a whole journey as one atomic player action.
use super::{LOCAL_PLAYER, Session};
use game_core::travel::{Location, PlayerId, TravelError};
use game_protocol::{ClientCommand, ServerMessage};

impl Session {
    pub fn handle_offline(&mut self, command: ClientCommand) -> ServerMessage {
        self.handle_offline_as(LOCAL_PLAYER, command)
    }

    pub fn handle_offline_as(&mut self, player: PlayerId, command: ClientCommand) -> ServerMessage {
        let (ship, revision) = match command {
            ClientCommand::Move {
                ship,
                target,
                expected_revision,
            } => return self.resolve_movement_as(player, ship, &target, expected_revision),
            ClientCommand::ContinueTravel {
                ship,
                expected_revision,
            } => (ship, expected_revision),
            _ => return self.handle_as(player, command),
        };
        if let Err(error) = self.state.authorize(player, ship) {
            return ServerMessage::CommandRejected(error.into());
        }
        if revision != self.state.revision {
            return ServerMessage::CommandRejected(TravelError::StaleQuote.into());
        }
        let mut candidate = self.clone();
        let Ok(moving_ship) = candidate.state.ship(ship) else {
            return ServerMessage::CommandRejected(TravelError::UnknownShip.into());
        };
        let Location::Travelling(journey) = &moving_ship.location else {
            return ServerMessage::CommandRejected(TravelError::NotTravelling.into());
        };
        let remaining = journey.arrival_tick.saturating_sub(candidate.state.tick);
        let mut response = candidate.advance(remaining);
        if let ServerMessage::SessionSnapshot { .. } = &mut response {
            *self = candidate;
        }
        response
    }
}
