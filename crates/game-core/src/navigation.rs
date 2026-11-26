//! Plot fleet courses, then advance all fleets through one shared navigation run.
pub(crate) mod band_motion;
mod band_roaming;
mod battle;
pub mod chase;
mod contacts;
mod errors;
mod haven;
mod hostile;
mod landing;
mod next_leg;
mod raiders;
pub(crate) use band_motion::{distance, local_in, near, step_toward};
pub use band_roaming::{BandTransit, MAX_BANDS_PER_SYSTEM};
pub use raiders::{BAND_KM_PER_RUN, NPC_STRIKE_KM, STRIKE_KM};
pub use raiders::{COOLDOWN_RUNS, LOOT_COOLDOWN_RUNS};
pub use raiders::{Prey, RaiderBand};
mod orders;
mod position;
pub use battle::{ActiveBattle, BattleAction};
pub use errors::NavigationError;
pub use haven::{GARRISON, RaiderHaven, haven_site};
pub(crate) use hostile::escort_setup;
pub use hostile::{MAX_SAVED_RAIDERS, evade_chance_percent};
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
    /// A faction patrol pursuing you (see `police`): comply and pay the
    /// fine, fight, or try to outrun it.
    Police,
    /// An NPC trader that came alongside offers to sell you its hold below
    /// list price (see `approach`): trade or let it go.
    TraderOffer,
    /// A corsair that outguns you demands half your cargo (see `approach`):
    /// comply, fight, or try to outrun it.
    Shakedown,
    /// An NPC trader stalked by a raider band calls for help (see
    /// `approach`): fight the band or leave the trader to it.
    TraderDistress,
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
            Self::Police => "Police patrol",
            Self::TraderOffer => "Trader alongside",
            Self::Shakedown => "Corsair shakedown",
            Self::TraderDistress => "Trader in distress",
        }
    }

    /// Contacts with an NPC trader, whose ship (and escort) are the hostiles.
    pub fn with_trader(self) -> bool {
        matches!(self, Self::Trader | Self::TraderOffer | Self::Shakedown)
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
    /// The faction patrol that stopped you, for a `Police` contact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub police: Option<crate::police::PoliceContact>,
    /// The NPC trader (`npc::Trader::id`) calling for help against the band
    /// in `band`, for a `TraderDistress` contact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rescue: Option<u64>,
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
    /// NPC traders the player robbed or attacked (see `piracy`).
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub piracies: u64,
    /// Successful cargo seizures and units taken from NPC traders.
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub pirate_seizures: u64,
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub pirate_cargo: u64,
    /// Successful escapes from faction police contacts.
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub pirate_escapes: u64,
    /// Hired escorts, at most one per fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escorts: Vec<Escort>,
    /// Faction police patrols (see `police`).
    #[serde(default, skip_serializing_if = "crate::police::PoliceState::is_empty")]
    pub police: crate::police::PoliceState,
    /// The flagship's chase of a trader or raider band (see `pursuit`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pursuit: Option<crate::pursuit::Pursuit>,
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
            pirate_seizures: 0,
            pirate_cargo: 0,
            pirate_escapes: 0,
            escorts: Vec::new(),
            police: Default::default(),
            pursuit: None,
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
    /// The flagship's pursuit ended without a contact (see `pursuit`).
    PursuitNote {
        fleet: FleetId,
        note: String,
    },
    /// The player took direct command of a fleet.
    FlagshipChanged(FleetId),
    /// News of faction police in a system where you have a fleet.
    PoliceNote(String),
}
