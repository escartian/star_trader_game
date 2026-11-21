//! The Raider Haven: the fortified base the frontier's raider bands come from.
//!
//! - New games place it in the system farthest from home (ties: lowest ID),
//!   beside that system's most dangerous non-station body (ties: lowest ID).
//!   A one-system galaxy has none. Placement uses no randomness, so a save
//!   from before the Haven gets the same one on load.
//! - Its garrison, [`GARRISON`], is the largest raider group a battle holds.
//! - A fleet orbiting or landed at the Haven's body may **assault** it: a raider
//!   contact against the garrison. Fight or Auto-resolve opens the battle;
//!   Evade calls the assault off, always successfully.
//! - Garrison ships destroyed stay lost (no rebuilding). A battle that leaves
//!   none destroys the Haven: the Soldier road's victory.
//! - While the Haven stands, wiped-out bands return (`raiders`); once it
//!   falls they never do.
use super::{Encounter, EncounterKind, NavigationError, NavigationEvent};
use crate::{
    BodyId, BodyKind, StarSystem, SystemId,
    fleet::{FleetError, FleetId, FleetMovementError, ShipKind},
    planet::Danger,
    travel::{Location, PlayerId, TravelError, TravelState},
};
use serde::{Deserialize, Serialize};

/// The Haven's garrison at full strength: three patrol ships and five scouts.
pub const GARRISON: [ShipKind; 8] = [
    ShipKind::Patrol,
    ShipKind::Patrol,
    ShipKind::Patrol,
    ShipKind::Scout,
    ShipKind::Scout,
    ShipKind::Scout,
    ShipKind::Scout,
    ShipKind::Scout,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaiderHaven {
    pub system: SystemId,
    /// The body the Haven is anchored beside; assaults start from it.
    pub body: BodyId,
    /// Garrison ships still standing.
    pub garrison: Vec<ShipKind>,
    /// Garrison ships destroyed so far.
    #[serde(default)]
    pub sunk: u32,
    /// The run on which the Haven fell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroyed_run: Option<u64>,
}

impl RaiderHaven {
    pub fn standing(&self) -> bool {
        self.destroyed_run.is_none()
    }
}

fn danger_rank(body: &crate::LocalBody) -> u8 {
    match body.danger() {
        Some(Danger::Insidious) => 6,
        Some(Danger::VeryDangerous) => 5,
        Some(Danger::Dangerous) => 4,
        Some(Danger::Moderate) => 3,
        Some(Danger::Safe) => 2,
        Some(Danger::VerySafe) => 1,
        None => 0,
    }
}

/// Where a new game puts the Haven, if the galaxy has a system beyond home.
pub fn haven_site(systems: &[StarSystem]) -> Option<(SystemId, BodyId)> {
    let home = systems.first()?.position;
    let gap = |system: &StarSystem| {
        let axis = |a: i64, b: i64| (i128::from(a) - i128::from(b)).pow(2);
        axis(system.position.x, home.x)
            + axis(system.position.y, home.y)
            + axis(system.position.z, home.z)
    };
    let system = systems
        .iter()
        .skip(1)
        .max_by_key(|system| (gap(system), std::cmp::Reverse(system.id.0)))?;
    let body = system
        .bodies
        .iter()
        .filter(|body| body.kind != BodyKind::Station)
        .max_by_key(|body| (danger_rank(body), std::cmp::Reverse(body.id.0)))?;
    Some((system.id, body.id))
}

impl TravelState {
    /// Places the Haven unless the game already has one.
    pub fn place_haven(&mut self, systems: &[StarSystem]) {
        if self.navigation.haven.is_some() {
            return;
        }
        self.navigation.haven = haven_site(systems).map(|(system, body)| RaiderHaven {
            system,
            body,
            garrison: GARRISON.to_vec(),
            sunk: 0,
            destroyed_run: None,
        });
    }

    /// Opens an assault on the Haven by `fleet`, which must be orbiting or
    /// landed at the Haven's body with no other contact pending.
    pub fn assault_haven(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let record = self
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?;
        if record.owner != player {
            return Err(FleetError::NotOwner.into());
        }
        if self.navigation.battle.is_some() {
            return Err(NavigationError::BattleUnderway.into());
        }
        if self
            .navigation
            .encounters
            .iter()
            .any(|contact| contact.fleet == fleet)
        {
            return Err(NavigationError::EncounterPending(fleet).into());
        }
        let haven = self
            .navigation
            .haven
            .as_ref()
            .filter(|haven| haven.standing())
            .ok_or(NavigationError::NoHaven)?;
        let lead = self.ship(*record.ships.first().ok_or(FleetError::UnknownFleet)?)?;
        let at_haven = matches!(
            lead.location,
            Location::Orbiting { system, body } | Location::Landed { system, body }
                if system == haven.system && body == haven.body
        );
        // Reaching the Haven takes at least one run, so run 0 never qualifies.
        if !at_haven || self.navigation.run == 0 {
            return Err(NavigationError::NotAtHaven.into());
        }
        let hostiles = haven.garrison.clone();
        let next = self
            .navigation
            .next_encounter_id
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let encounter = Encounter {
            id: self.navigation.next_encounter_id,
            fleet,
            kind: EncounterKind::Raiders,
            run: self.navigation.run,
            hostiles,
            band: None,
            haven: true,
            trader: None,
            police: None,
        };
        self.navigation.next_encounter_id = next;
        self.navigation.encounters.push(encounter.clone());
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        Ok(NavigationEvent::EncounterFound(encounter))
    }

    /// After an assault's battle: destroyed garrison ships are gone for good;
    /// with none left, the Haven falls on this run.
    pub(crate) fn haven_after_contact(&mut self, destroyed: &[ShipKind]) {
        let run = self.navigation.run;
        let Some(haven) = self.navigation.haven.as_mut() else {
            return;
        };
        for kind in destroyed {
            if let Some(slot) = haven.garrison.iter().position(|ship| ship == kind) {
                haven.garrison.remove(slot);
                haven.sunk += 1;
            }
        }
        if haven.garrison.is_empty() {
            haven.destroyed_run = Some(run);
        }
    }

    /// Structural checks for a saved Haven.
    pub(super) fn validate_haven(&self, systems: &[StarSystem]) -> Result<(), String> {
        let Some(haven) = &self.navigation.haven else {
            return Ok(());
        };
        let body = systems
            .iter()
            .find(|system| system.id == haven.system)
            .and_then(|system| system.bodies.iter().find(|body| body.id == haven.body));
        let full = GARRISON.len();
        let valid_garrison = haven.garrison.iter().all(|kind| {
            haven.garrison.iter().filter(|ship| *ship == kind).count()
                <= GARRISON.iter().filter(|ship| *ship == kind).count()
        });
        if body.is_none_or(|body| body.kind == BodyKind::Station)
            || haven.garrison.len() + haven.sunk as usize != full
            || !valid_garrison
            || haven.garrison.is_empty() == haven.standing()
            || haven
                .destroyed_run
                .is_some_and(|run| run > self.navigation.run)
        {
            return Err("Invalid Raider Haven".into());
        }
        Ok(())
    }
}
