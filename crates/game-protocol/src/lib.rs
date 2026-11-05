//! Transport-independent messages. Serialization and networking come later.
mod error;
pub use error::CommandError;
use game_core::economy::{Commodity, Economy, TradeQuote, TradeSide};
use game_core::fleet::{FleetId, ShipKind};
use game_core::{
    StarSystem, SystemId,
    travel::{ShipId, TravelQuote, TravelState},
};

pub const PROTOCOL_VERSION: u32 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientCommand {
    GetGalaxy,
    GetSession,
    BuyShip {
        ship: ShipId,
        kind: ShipKind,
        expected_revision: u64,
    },
    SplitFleet {
        fleet: FleetId,
        ships: Vec<ShipId>,
        expected_revision: u64,
    },
    MergeFleets {
        source: FleetId,
        target: FleetId,
        expected_revision: u64,
    },
    TransferCargo {
        source: ShipId,
        target: ShipId,
        commodity: Commodity,
        quantity: u32,
        expected_revision: u64,
    },
    PreviewTravel {
        ship: ShipId,
        destination: SystemId,
    },
    PreviewTrade {
        ship: ShipId,
        commodity: Commodity,
        quantity: u32,
        side: TradeSide,
    },
    Trade {
        ship: ShipId,
        commodity: Commodity,
        quantity: u32,
        side: TradeSide,
        expected_revision: u64,
    },
    Refuel {
        ship: ShipId,
        units: u32,
        expected_revision: u64,
    },
    UpgradeHold {
        ship: ShipId,
        expected_revision: u64,
    },
    ContinueTravel {
        ship: ShipId,
        expected_revision: u64,
    },
    Travel {
        ship: ShipId,
        destination: SystemId,
        expected_revision: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    Departed {
        destination: SystemId,
    },
    Arrived {
        system: SystemId,
        ship: ShipId,
    },
    ShipPurchased {
        ship: ShipId,
    },
    FleetSplit {
        fleet: FleetId,
        new_fleet: FleetId,
    },
    FleetsMerged {
        source: FleetId,
        target: FleetId,
    },
    CargoTransferred {
        source: ShipId,
        target: ShipId,
        quantity: u32,
    },
    Traded(TradeQuote),
    Refuelled {
        units: u32,
    },
    HoldUpgraded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    GalaxySnapshot {
        protocol_version: u32,
        systems: Vec<StarSystem>,
    },
    SessionSnapshot {
        state: TravelState,
        economy: Economy,
        events: Vec<SessionEvent>,
    },
    TravelPreview(TravelQuote),
    TradePreview(TradeQuote),
    CommandRejected(CommandError),
}
