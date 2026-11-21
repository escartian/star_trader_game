//! Plot fleet courses, then advance all fleets through one shared navigation run.
mod band_motion;
mod band_roaming;
mod battle;
mod contacts;
mod haven;
mod hostile;
mod landing;
mod next_leg;
mod raiders;
pub(crate) use band_motion::{distance, local_in};
pub use band_roaming::{BandTransit, MAX_BANDS_PER_SYSTEM};
pub use raiders::STRIKE_KM;
pub use raiders::{COOLDOWN_RUNS, LOOT_COOLDOWN_RUNS};
pub use raiders::{Prey, RaiderBand};
mod orders;
mod position;
pub use battle::{ActiveBattle, BattleAction};
pub use haven::{GARRISON, RaiderHaven, haven_site};
pub(crate) use hostile::escort_setup;
pub use hostile::evade_chance_percent;
pub use position::space_position;
mod resolution;
mod validation;
use crate::{
    fleet::FleetId,
    movement::{MoveTarget, MovementPlan},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationOrder {
    pub target: MoveTarget,
    #[serde(default)]
    pub remaining_waypoints: Vec<MoveTarget>,
    pub plan: MovementPlan,
    pub remaining_ticks: u64,
    pub started: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncounterKind {
    DebrisField,
    DistressSignal,
    /// Hostile raiders; resolved by fighting or attempting to evade.
    Raiders,
    DerelictFreighter,
    MerchantConvoy,
    CargoPod,
    CustomsInspection,
    /// A port authority cutter under raider pressure asks for an escort.
    AuthorityCutter,
    Anomaly,
    Comet,
    LostProbe,
    /// An NPC trader the flagship hailed (see `piracy`): let it go, demand
    /// cargo or attack it.
    Trader,
}
impl EncounterKind {
    /// Every contact a run can roll besides raiders (see `contacts`).
    pub const PEACEFUL: [EncounterKind; 10] = [
        Self::DebrisField,
        Self::DistressSignal,
        Self::DerelictFreighter,
        Self::MerchantConvoy,
        Self::CargoPod,
        Self::CustomsInspection,
        Self::AuthorityCutter,
        Self::Anomaly,
        Self::Comet,
        Self::LostProbe,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::DebrisField => "Salvage contact",
            Self::DistressSignal => "Distress signal",
            Self::Raiders => "Raider contact",
            Self::DerelictFreighter => "Derelict freighter",
            Self::MerchantConvoy => "Merchant convoy",
            Self::CargoPod => "Drifting cargo pod",
            Self::CustomsInspection => "Customs inspection",
            Self::AuthorityCutter => "Authority cutter",
            Self::Anomaly => "Sensor anomaly",
            Self::Comet => "Comet",
            Self::LostProbe => "Lost survey probe",
            Self::Trader => "Hailed trader",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterChoice {
    Investigate,
    Assist,
    Bypass,
    /// Raiders only: open a turn-based battle.
    Fight,
    /// Raiders only: one engine-based escape roll; failure opens a battle.
    Evade,
    /// Raiders only: fight the whole battle at once with the AI on both sides.
    AutoResolve,
    /// Customs: submit to the inspection and pay its fee.
    Comply,
    /// Merchant convoy: sell it cargo.
    Trade,
    /// Hailed trader: threaten it into handing over part of its cargo.
    Demand,
}

/// The NPC trader a `Trader` contact stopped, as it stood when hailed. Its
/// ship cannot change while the contact is pending: no run executes then.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraderContact {
    /// `npc::Trader::id`.
    pub id: u64,
    pub name: String,
    /// The trader's hull damage when hailed, in hull points.
    pub hull_damage: u32,
    /// The trader's fitted gear when hailed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gear: Vec<crate::combat::gear::Gear>,
    /// The trader had a hired escort, which fights beside it (`hostiles[1]`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub escort: bool,
}

/// A hired escort contract for one of the player's fleets (`combat::gear`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Escort {
    pub fleet: FleetId,
    /// Navigation runs left on the contract (1 to `ESCORT_RUNS`).
    pub runs_left: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub id: u64,
    pub fleet: FleetId,
    pub kind: EncounterKind,
    pub run: u64,
    /// Hostile hulls for `Raiders`; empty for every other kind.
    #[serde(default)]
    pub hostiles: Vec<crate::fleet::ShipKind>,
    /// The roaming band that struck, if this contact is one (see `raiders`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<u64>,
    /// An assault on the Raider Haven's garrison (see `haven`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub haven: bool,
    /// The hailed NPC trader, for a `Trader` contact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trader: Option<TraderContact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationState {
    pub run: u64,
    pub orders: Vec<NavigationOrder>,
    /// Explicit Wait orders (shown as "Wait" in the client). A Wait is always
    /// player-issued and lasts across runs until the fleet receives a course or
    /// stops existing. A fleet never has both a Wait and a course.
    pub holds: Vec<FleetId>,
    pub encounters: Vec<Encounter>,
    pub rng_state: u64,
    pub next_encounter_id: u64,
    pub rescues: u64,
    /// The most recent engagement, kept readable across saves.
    #[serde(default)]
    pub last_combat: Option<Box<crate::combat::CombatReport>>,
    /// Standing trade routes, at most one per fleet. Omitted from saves when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<crate::routes::TradeRoute>,
    /// The raider battle being played, if any; its encounter stays pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battle: Option<Box<ActiveBattle>>,
    /// Roaming raider bands, one per system but the home system.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bands: Vec<RaiderBand>,
    /// (system, run) a wiped-out band returns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub band_respawns: Vec<(crate::SystemId, u64)>,
    #[serde(default = "first_band_id")]
    pub next_band_id: u64,
    /// The bands' own random stream (other contact rolls never depend on it).
    #[serde(default)]
    pub band_rng: u64,
    /// Credits (in cents, at a new game's price level) a contact just earned
    /// or cost; the host settles it into the wallet at once, so saves hold 0.
    #[serde(default, skip_serializing_if = "is_zero_payout")]
    pub payout: i64,
    /// The Raider Haven; `None` only in a one-system galaxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub haven: Option<RaiderHaven>,
    /// The fleet the player commands directly; `None` means the first owned
    /// fleet (see `TravelState::flagship`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flagship: Option<FleetId>,
    /// Secondary fleets guarding a system (see `patrols`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patrols: Vec<crate::patrols::Patrol>,
    /// NPC traders the player robbed or attacked (see `piracy`); the future
    /// wanted status and pirate road read it.
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub piracies: u64,
    /// Hired escorts, at most one per fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escorts: Vec<Escort>,
}

fn is_zero_count(value: &u64) -> bool {
    *value == 0
}

fn is_zero_payout(value: &i64) -> bool {
    *value == 0
}

fn first_band_id() -> u64 {
    1
}
impl Default for NavigationState {
    fn default() -> Self {
        Self {
            run: 0,
            orders: Vec::new(),
            holds: Vec::new(),
            encounters: Vec::new(),
            rng_state: 0,
            next_encounter_id: 1,
            rescues: 0,
            last_combat: None,
            routes: Vec::new(),
            battle: None,
            bands: Vec::new(),
            band_respawns: Vec::new(),
            next_band_id: 1,
            band_rng: 0,
            payout: 0,
            haven: None,
            flagship: None,
            patrols: Vec::new(),
            piracies: 0,
            escorts: Vec::new(),
        }
    }
}
impl NavigationState {
    pub fn order(&self, fleet: FleetId) -> Option<&NavigationOrder> {
        self.orders.iter().find(|order| order.plan.fleet == fleet)
    }
    /// Runs left on `fleet`'s escort contract (0: none).
    pub fn escort_runs(&self, fleet: FleetId) -> u32 {
        self.escorts
            .iter()
            .find(|escort| escort.fleet == fleet)
            .map_or(0, |escort| escort.runs_left)
    }

    pub fn rolls(&mut self) -> u64 {
        self.rng_state = self.rng_state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.rng_state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationEvent {
    CoursePlotted(FleetId),
    CourseCancelled(FleetId),
    Holding(FleetId),
    Arrived(MovementPlan),
    RunExecuted(u64),
    EncounterFound(Encounter),
    EncounterResolved {
        fleet: FleetId,
        outcome: String,
    },
    /// A battle is open and one of the fleet's ships is up.
    BattleTurn {
        fleet: FleetId,
    },
    /// A body moved across the rest of the course and no clear way around it
    /// was found: the fleet stopped and needs a new course.
    CourseBlocked(FleetId),
    /// The fleet set down on a body's surface.
    Landed {
        fleet: FleetId,
        body: crate::BodyId,
    },
    /// The fleet lifted off back into orbit.
    LiftedOff {
        fleet: FleetId,
        body: crate::BodyId,
    },
    /// A waiting landed fleet mined `ore` units this run (0: holds are full).
    Mined {
        fleet: FleetId,
        body: crate::BodyId,
        ore: u32,
    },
    /// A route fleet traded at a stop and set off for the other one.
    RouteServiced {
        fleet: FleetId,
        summary: String,
    },
    /// A route fleet found nothing profitable to carry and waits at its stop.
    RoutePaused {
        fleet: FleetId,
        reason: String,
    },
    /// A route ended, by the player's orders or because it could not continue.
    RouteEnded {
        fleet: FleetId,
        reason: String,
    },
    /// A patrol was ordered, or could not set off this run.
    PatrolNote {
        fleet: FleetId,
        note: String,
    },
    /// The player took direct command of a fleet.
    FlagshipChanged(FleetId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationError {
    NeedsOrders(FleetId),
    EncounterPending(FleetId),
    Underway,
    NoCourse,
    FleetHasCourse,
    UnknownEncounter,
    InvalidChoice,
    MissingMedicine,
    CargoFull,
    /// Trade routes start from the market where the fleet is.
    NotAtMarket,
    /// The other end of a route must be a different market.
    InvalidRouteStop,
    NoRoute,
    /// A route would leave with an empty hold: nothing aboard, nothing profitable.
    NothingProfitable,
    NoBattle,
    /// Landing starts from orbit around the body.
    NotInOrbit,
    /// Gas giants and stations have no surface to land on.
    CannotLand,
    NotLanded,
    /// Another raider battle must finish first.
    BattleUnderway,
    /// There is no Raider Haven standing to assault.
    NoHaven,
    /// Assaults start from orbit around (or the surface of) the Haven's body.
    NotAtHaven,
    /// The campaign is lost or out of time; only saves and loads remain.
    CampaignOver,
    /// A convoy has nothing to buy from an empty hold.
    NothingToSell,
    /// Direct orders go to the flagship; other fleets run standing orders.
    NotFlagship,
    /// The flagship takes direct orders and cannot run a patrol.
    FlagshipCommanded,
    /// A run limit must lie beyond the current run.
    InvalidRunLimit,
    /// No NPC trader has that ID.
    UnknownTrader,
    /// The flagship must be close to the trader, in open space or orbit.
    TraderOutOfReach,
    /// The trader is docked at a station or being refitted.
    TraderProtected,
    /// The hailed trader will not hand over cargo to a fleet this weak.
    WillNotYield,
    Battle(crate::combat::tactical::BattleError),
}
impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsOrders(fleet) => write!(
                f,
                "Fleet {} needs a course or a Wait order before executing this navigation run.",
                fleet.0
            ),
            Self::EncounterPending(fleet) => write!(
                f,
                "Resolve Fleet {}'s encounter before executing another navigation run.",
                fleet.0
            ),
            Self::Underway => f.write_str(
                "This course is already underway; it continues with each navigation run.",
            ),
            Self::NoCourse => f.write_str("This fleet has no course to cancel."),
            Self::FleetHasCourse => {
                f.write_str("Cancel the plotted course first: a fleet with a course cannot change ships or land.")
            }
            Self::UnknownEncounter => {
                f.write_str("This encounter has already been resolved or is unknown.")
            }
            Self::InvalidChoice => f.write_str("That choice is unavailable for this encounter."),
            Self::MissingMedicine => {
                f.write_str("Assisting requires one medicine in this fleet's cargo.")
            }
            Self::CargoFull => {
                f.write_str("The fleet needs one free cargo space to recover salvage.")
            }
            Self::NotAtMarket => f.write_str("Start a trade route while the fleet is at a market."),
            Self::InvalidRouteStop => {
                f.write_str("Pick a different market as the other end of the route.")
            }
            Self::NoRoute => f.write_str("This fleet has no trade route."),
            Self::NothingProfitable => f.write_str(
                "Nothing profitable to carry to that market right now; the hold would be empty.",
            ),
            Self::NoBattle => f.write_str("There is no battle in progress."),
            Self::NotInOrbit => {
                f.write_str("Enter orbit around a planet, moon or asteroid to land.")
            }
            Self::CannotLand => f.write_str("Gas giants and stations have no surface to land on."),
            Self::NotLanded => f.write_str("This fleet is not on a surface."),
            Self::BattleUnderway => f.write_str("Finish the battle in progress first."),
            Self::NoHaven => f.write_str("There is no Raider Haven left to assault."),
            Self::NotFlagship => f.write_str(
                "Only your flagship takes direct orders. Give this fleet a patrol, a trade \
                 route or Wait, or make it the flagship.",
            ),
            Self::FlagshipCommanded => f.write_str(
                "The flagship takes your direct orders. Make another fleet the flagship first.",
            ),
            Self::NothingToSell => f.write_str("There is no cargo aboard to sell."),
            Self::CampaignOver => {
                f.write_str("The campaign is over. Load a save or start a new game.")
            }
            Self::InvalidRunLimit => {
                f.write_str("A campaign length must end after the current run.")
            }
            Self::NotAtHaven => {
                f.write_str("Bring the fleet into orbit at the Raider Haven's body to assault it.")
            }
            Self::UnknownTrader => f.write_str("There is no such trader."),
            Self::TraderOutOfReach => f.write_str(
                "Bring your flagship within 20 million km of the trader, in open space or \
                 orbit, to hail it.",
            ),
            Self::TraderProtected => f.write_str(
                "That trader is docked under a station's protection or laid up for a refit.",
            ),
            Self::WillNotYield => f.write_str(
                "The trader will not hand over cargo to a fleet this weak; attack or let it go.",
            ),
            Self::Battle(error) => error.fmt(f),
        }
    }
}
