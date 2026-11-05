//! Local authority shared by the offline client and dedicated executable.
use game_core::StarSystem;
use game_protocol::{ClientCommand, PROTOCOL_VERSION, ServerMessage};

pub struct Session {
    systems: Vec<StarSystem>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            systems: game_core::demo_galaxy(),
        }
    }
}

impl Session {
    pub fn handle(&mut self, command: ClientCommand) -> ServerMessage {
        match command {
            ClientCommand::GetGalaxy => ServerMessage::GalaxySnapshot {
                protocol_version: PROTOCOL_VERSION,
                systems: self.systems.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_are_independent_of_the_authority() {
        let mut session = Session::default();
        let original = session.handle(ClientCommand::GetGalaxy);
        let ServerMessage::GalaxySnapshot { mut systems, .. } = original.clone();
        systems.clear();
        assert_eq!(session.handle(ClientCommand::GetGalaxy), original);
    }
}
