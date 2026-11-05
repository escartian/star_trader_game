//! Transport-independent messages. Serialization and networking come later.

use game_core::StarSystem;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientCommand {
    GetGalaxy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    GalaxySnapshot {
        protocol_version: u32,
        systems: Vec<StarSystem>,
    },
}
