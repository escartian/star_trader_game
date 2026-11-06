//! Transport-independent messages. Serialization and networking come later.
mod error;
pub use error::CommandError;
use game_core::economy::{Commodity, Economy, TradeQuote, TradeSide};
use game_core::fleet::{FleetId, ShipKind};
use game_core::{
    StarSystem, SystemId,
    travel::{ShipId, TravelState},
};

pub const PROTOCOL_VERSION: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientCommand {
    GetGalaxy,
    GetSession,
    BuyShip {
        ship: ShipId,
        kind: ShipKind,
        expected_revision: u64,
    },
    SellShip {
        ship: ShipId,
        expected_revision: u64,
    },
    RenameFleet {
        fleet: FleetId,
        name: String,
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
    PreviewMove {
        ship: ShipId,
        target: game_core::movement::MoveTarget,
    },
    Move {
        ship: ShipId,
        target: game_core::movement::MoveTarget,
        expected_revision: u64,
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
    ExecuteNavigationRun {
        expected_revision: u64,
    },
    HoldFleet {
        fleet: FleetId,
        expected_revision: u64,
    },
    CancelCourse {
        fleet: FleetId,
        expected_revision: u64,
    },
    ResolveEncounter {
        encounter: u64,
        choice: game_core::navigation::EncounterChoice,
        expected_revision: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    Navigation(game_core::navigation::NavigationEvent),
    Arrived {
        system: SystemId,
        ship: ShipId,
    },
    ShipSold {
        ship: ShipId,
        price: game_core::economy::Credits,
    },
    FleetRenamed {
        fleet: FleetId,
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
        economy: Box<Economy>,
        events: Vec<SessionEvent>,
    },
    MovementPreview(game_core::movement::MovementPlan),
    TradePreview(TradeQuote),
    CommandRejected(CommandError),
}
