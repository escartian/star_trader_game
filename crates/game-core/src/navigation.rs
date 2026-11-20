//! Plot fleet courses, then advance all fleets through one shared navigation run.
mod hostile;
mod orders;
mod position;
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
}
impl EncounterKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::DebrisField => "Salvage contact",
            Self::DistressSignal => "Distress signal",
            Self::Raiders => "Raider contact",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterChoice {
    Investigate,
    Assist,
    Bypass,
    /// Raiders only: resolve a full engagement.
    Fight,
    /// Raiders only: one engine-based escape roll; failure forces a fight.
    Evade,
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
        }
    }
}
impl NavigationState {
    pub fn order(&self, fleet: FleetId) -> Option<&NavigationOrder> {
        self.orders.iter().find(|order| order.plan.fleet == fleet)
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
                f.write_str("Cancel the plotted course before changing this fleet's ships.")
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
        }
    }
}
