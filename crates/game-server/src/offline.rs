//! In-process convenience adapter; solo and future connected play share command rules.
use super::{LOCAL_PLAYER, Session};
use game_core::travel::PlayerId;
use game_protocol::{ClientCommand, ServerMessage};

impl Session {
    pub fn handle_offline(&mut self, command: ClientCommand) -> ServerMessage {
        self.handle_as(LOCAL_PLAYER, command)
    }
    pub fn handle_offline_as(&mut self, player: PlayerId, command: ClientCommand) -> ServerMessage {
        self.handle_as(player, command)
    }
}
