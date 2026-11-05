//! Transport-independent messages. Serialization and networking come later.
use game_core::{
    StarSystem, SystemId,
    travel::{ShipId, TravelError, TravelQuote, TravelState},
};

pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientCommand {
    GetGalaxy,
    GetSession,
    PreviewTravel {
        ship: ShipId,
        destination: SystemId,
    },
    Travel {
        ship: ShipId,
        destination: SystemId,
        expected_revision: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    Departed { destination: SystemId },
    Arrived { system: SystemId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    GalaxySnapshot {
        protocol_version: u32,
        systems: Vec<StarSystem>,
    },
    SessionSnapshot {
        state: TravelState,
        events: Vec<SessionEvent>,
    },
    TravelPreview(TravelQuote),
    CommandRejected(TravelError),
}
