//! Transport-independent messages. Serialization and networking come later.
mod error;
pub use error::CommandError;
use game_core::economy::{Commodity, Economy, TradeQuote, TradeSide};
use game_core::fleet::{FleetId, ShipKind};
use game_core::{
    StarSystem, SystemId,
    travel::{ShipId, TravelState},
};

pub const PROTOCOL_VERSION: u32 = 14;

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
    PreviewDestination {
        ship: ShipId,
        system: SystemId,
        body: Option<game_core::BodyId>,
    },
    PlotDestination {
        ship: ShipId,
        system: SystemId,
        body: Option<game_core::BodyId>,
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
    /// Restore all lost hull points at the ship's current market.
    RepairShip {
        ship: ShipId,
        expected_revision: u64,
    },
    /// Fit anti-piracy gear to `ship` at its current market.
    FitGear {
        ship: ShipId,
        gear: game_core::combat::gear::Gear,
        expected_revision: u64,
    },
    /// Hire (or renew) an escort for the fleet `ship` leads, at its market.
    HireEscort {
        ship: ShipId,
        expected_revision: u64,
    },
    /// Pay your fine to the faction that holds the port where `ship` is
    /// (see `game_core::police`), clearing your wanted status there.
    PayFine {
        ship: ShipId,
        expected_revision: u64,
    },
    /// Buy the charter of the port where `ship` is (see `economy::charters`).
    BuyCharter {
        ship: ShipId,
        expected_revision: u64,
    },
    /// Assault the Raider Haven with `fleet`, from orbit at its body.
    AssaultHaven {
        fleet: FleetId,
        expected_revision: u64,
    },
    /// Hail the NPC trader `trader` (`npc::Trader::id`) with the flagship `fleet`.
    HailTrader {
        fleet: FleetId,
        trader: u64,
        expected_revision: u64,
    },
    /// The flagship chases a trader or raider band in its own system until it
    /// catches it (the contact opens) or loses it (`game_core::pursuit`).
    Pursue {
        fleet: FleetId,
        quarry: game_core::pursuit::Quarry,
        expected_revision: u64,
    },
    /// Take direct command of `fleet`; every other fleet runs standing orders.
    SetFlagship {
        fleet: FleetId,
        expected_revision: u64,
    },
    /// Order a secondary fleet to guard `system`.
    SetPatrol {
        fleet: FleetId,
        system: game_core::SystemId,
        expected_revision: u64,
    },
    /// Take offer `job` from the board of the port where `ship` is.
    AcceptJob {
        job: u64,
        ship: ShipId,
        expected_revision: u64,
    },
    /// Give up active `job`; it fails with its standing penalty.
    AbandonJob {
        job: u64,
        expected_revision: u64,
    },
    /// End the campaign on score at this run (`None`: open-ended).
    SetRunLimit {
        runs: Option<u64>,
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
    /// Loop the fleet between the market it is at and `destination`, trading the most
    /// profitable cargo at each end.
    SetTradeRoute {
        fleet: FleetId,
        destination: game_core::economy::MarketId,
        expected_revision: u64,
    },
    /// End the fleet's route; a leg already plotted or underway still completes.
    CancelTradeRoute {
        fleet: FleetId,
        expected_revision: u64,
    },
    /// Set an orbiting fleet down on its rocky planet, moon or asteroid.
    Land {
        fleet: FleetId,
        expected_revision: u64,
    },
    /// Lift a landed fleet back into orbit.
    LiftOff {
        fleet: FleetId,
        expected_revision: u64,
    },
    /// Act with the player ship that is up in the open raider battle.
    BattleAction {
        action: game_core::navigation::BattleAction,
        expected_revision: u64,
    },
}

impl ClientCommand {
    /// Read-only requests: they never change the session.
    pub fn is_query(&self) -> bool {
        matches!(
            self,
            Self::GetGalaxy
                | Self::GetSession
                | Self::PreviewDestination { .. }
                | Self::PreviewMove { .. }
                | Self::PreviewTrade { .. }
        )
    }
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
    CharterBought {
        cost: game_core::economy::Credits,
    },
    RunLimitSet {
        runs: Option<u64>,
    },
    /// A feature's controls just became available (see `game_core::progress`).
    Unlocked {
        feature: game_core::progress::Feature,
    },
    Refuelled {
        units: u32,
    },
    HoldUpgraded,
    Repaired {
        ship: ShipId,
        cost: game_core::economy::Credits,
    },
    GearFitted {
        ship: ShipId,
        gear: game_core::combat::gear::Gear,
        cost: game_core::economy::Credits,
    },
    EscortHired {
        cost: game_core::economy::Credits,
    },
    /// Your standing with a faction moved by `change` to `standing`
    /// (see `game_core::factions`).
    /// A job was taken from a port's board (see `game_core::jobs`).
    JobAccepted {
        job: u64,
        title: String,
    },
    /// An active job was completed (paid) or failed.
    JobFinished(game_core::jobs::JobOutcome),
    /// A faction offers its next story quest step (`TravelState::jobs.quest_offers`).
    QuestOffered {
        job: u64,
    },
    StandingChanged {
        faction: game_core::factions::Faction,
        change: i32,
        standing: i32,
    },
    /// A faction now wants you for `runs` runs, or (0) no longer does
    /// (see `game_core::factions::wanted`).
    WantedChanged {
        faction: game_core::factions::Faction,
        runs: u32,
    },
    /// You paid your fine at a faction's port.
    FinePaid {
        faction: game_core::factions::Faction,
        cost: game_core::economy::Credits,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    GalaxySnapshot {
        protocol_version: u32,
        systems: Vec<StarSystem>,
    },
    SessionSnapshot {
        state: Box<TravelState>,
        economy: Box<Economy>,
        events: Vec<SessionEvent>,
    },
    DestinationPreview(game_core::movement::CoursePreview),
    MovementPreview(game_core::movement::MovementPlan),
    TradePreview(TradeQuote),
    CommandRejected(CommandError),
}
