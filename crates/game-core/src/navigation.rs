//! Plot fleet courses, then advance all fleets through one shared navigation run.
mod orders;
mod position;
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
    pub plan: MovementPlan,
    pub remaining_ticks: u64,
    pub started: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncounterKind {
    DebrisField,
    DistressSignal,
}
impl EncounterKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::DebrisField => "Salvage contact",
            Self::DistressSignal => "Distress signal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterChoice {
    Investigate,
    Assist,
    Bypass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub id: u64,
    pub fleet: FleetId,
    pub kind: EncounterKind,
    pub run: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationState {
    pub run: u64,
    pub orders: Vec<NavigationOrder>,
    pub holds: Vec<FleetId>,
    pub encounters: Vec<Encounter>,
    pub rng_state: u64,
    pub next_encounter_id: u64,
    pub rescues: u64,
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
    EncounterResolved { fleet: FleetId, outcome: String },
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
}
impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsOrders(fleet) => write!(
                f,
                "Fleet {} needs a course or Hold position before executing this navigation run.",
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
        }
    }
}
