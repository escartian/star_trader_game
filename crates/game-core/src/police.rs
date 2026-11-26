//! Faction police: a patrol in every claimed system that keeps the peace and
//! comes after you when you break the law there.
//!
//! - New-game setup (and any later faction setup) places one patrol, a patrol
//!   ship leading a scout, in every system a faction holds. A patrol never
//!   leaves its system; it visits the system's bodies in turn.
//! - Each run, after fleets, raider bands and traders move (`patrol_run`):
//!   while the faction pursues you (wanted, or Hostile standing; see
//!   `factions::wanted`) the patrol hunts the nearest exposed fleet of yours
//!   within [`DETECTION_KM`] (docked and landed fleets are safe) and, within
//!   [`STRIKE_KM`], stops it: a `Police` contact. Otherwise it answers a
//!   raider band hunting or stalking someone in its system and fights it,
//!   auto-resolved on the spot.
//! - At a `Police` contact (`contact`): **Comply** pays the fine (whatever
//!   the wallet holds, plus all cargo aboard if that falls short) and clears
//!   the record; **Fight** or **Auto-resolve** opens a battle and is itself a
//!   crime; **Evade** is one engine roll, a failure opens the battle.
//!   Destroying police costs standing like piracy (host side, `standing`).
//! - Afterwards the patrol backs off for [`COOLDOWN_RUNS`]; a patrol wiped
//!   out is replaced after [`RESPAWN_RUNS`]. Patrol ships repair between fights.
//!
//! Police draw no random numbers of their own except to seed fights with
//! bands, which use the bands' stream.
mod contact;
mod patrol_run;
pub use contact::{fine, pay_fine_at_port, resolve};
pub(crate) use patrol_run::intercept;
pub use patrol_run::patrol_run;

use crate::{
    LocalPosition, StarSystem, SystemId,
    factions::{Faction, Factions},
    fleet::{FleetId, ShipKind},
    navigation::{EncounterKind, local_in},
    travel::TravelState,
};
use serde::{Deserialize, Serialize};

pub use crate::navigation::STRIKE_KM;
/// A patrol hunts your exposed fleets (or a raider band) within this
/// distance, in km: the same reach as a raider band.
pub const DETECTION_KM: i64 = 150_000_000;
/// Runs a patrol backs off after a contact with you.
pub const COOLDOWN_RUNS: u32 = 3;
/// Runs a patrol rests after fighting a raider band.
pub const BAND_FIGHT_COOLDOWN_RUNS: u32 = 6;
/// Runs before a wiped-out patrol is replaced: twice a raider band's wait, so
/// destroying the law buys a real stretch of lawless space.
pub const RESPAWN_RUNS: u64 = 40;
/// A patrol's ships when it is placed.
pub const PATROL_SHIPS: [ShipKind; 2] = [ShipKind::Patrol, ShipKind::Scout];

/// Every patrol, and the systems waiting for a replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoliceState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patrols: Vec<FactionPatrol>,
    /// (system, run) a wiped-out patrol is replaced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub respawns: Vec<(SystemId, u64)>,
    #[serde(default = "first_id")]
    pub next_id: u64,
}

fn first_id() -> u64 {
    1
}

impl Default for PoliceState {
    fn default() -> Self {
        Self {
            patrols: Vec::new(),
            respawns: Vec::new(),
            next_id: 1,
        }
    }
}

impl PoliceState {
    pub fn is_empty(&self) -> bool {
        self.patrols.is_empty() && self.respawns.is_empty() && self.next_id == 1
    }

    pub fn patrol(&self, id: u64) -> Option<&FactionPatrol> {
        self.patrols.iter().find(|patrol| patrol.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionPatrol {
    pub id: u64,
    pub faction: Faction,
    pub system: SystemId,
    /// Relative to the system's star, in km.
    pub position: LocalPosition,
    /// Where it is going: its quarry, or the body it visits next.
    pub heading: LocalPosition,
    pub ships: Vec<ShipKind>,
    /// Your fleet it is pursuing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pursuing: Option<FleetId>,
    /// The raider band it is closing on (`navigation::RaiderBand::id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answering: Option<u64>,
    /// Bodies visited so far; the next is the body at this count, in order.
    #[serde(default)]
    pub stops: u64,
    /// Runs left backing off.
    #[serde(default)]
    pub cooldown: u32,
}

/// The patrol behind a `Police` contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoliceContact {
    pub patrol: u64,
    pub faction: Faction,
    /// You ran the patrol down (a pursuit), rather than it stopping you:
    /// there is no fine to pay, only Fight, Auto-resolve or let it go.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub intercepted: bool,
}

/// The point beside the body a patrol visits after `stops` visits: well
/// outside the body, on the side away from the star.
pub(crate) fn post(
    world: &StarSystem,
    stops: u64,
    rules: &crate::movement::MovementRules,
) -> LocalPosition {
    let count = world.bodies.len() as u64;
    let Some(body) = (count > 0).then(|| &world.bodies[(stops % count) as usize]) else {
        return LocalPosition {
            x_km: rules.system_half_extent_km / 2,
            y_km: 0,
            z_km: 0,
        };
    };
    let offset = rules.body_radii.of(body.kind) * 4;
    LocalPosition {
        x_km: body.position.x_km
            + if body.position.x_km < 0 {
                -offset
            } else {
                offset
            },
        ..body.position
    }
}

/// The stop count and post an idle patrol (no fleet or band to chase) heads
/// for next: on to the next body once it has reached its current post.
pub(crate) fn rounds(
    patrol: &FactionPatrol,
    world: &StarSystem,
    rules: &crate::movement::MovementRules,
) -> (u64, LocalPosition) {
    let arrived = crate::navigation::near(patrol.position, post(world, patrol.stops, rules));
    let stops = patrol.stops + u64::from(arrived);
    (stops, post(world, stops, rules))
}

impl TravelState {
    /// Places one patrol in every claimed system, replacing any there were.
    pub fn spawn_police(&mut self, factions: &Factions, systems: &[StarSystem]) {
        self.navigation.police = PoliceState::default();
        for system in systems {
            if let Some(faction) = factions.owner(system.id) {
                self.add_patrol(faction, system);
            }
        }
    }

    pub(crate) fn add_patrol(&mut self, faction: Faction, world: &StarSystem) {
        let police = &mut self.navigation.police;
        let position = post(world, 0, &self.movement_rules);
        police.patrols.push(FactionPatrol {
            id: police.next_id,
            faction,
            system: world.id,
            position,
            heading: position,
            ships: PATROL_SHIPS.to_vec(),
            pursuing: None,
            answering: None,
            stops: 0,
            cooldown: 0,
        });
        police.next_id += 1;
    }

    /// After a contact with you ends: the patrol loses its destroyed ships
    /// and backs off, or it is gone and will be replaced.
    pub(crate) fn police_after_contact(&mut self, patrol: u64, destroyed: &[ShipKind]) {
        self.patrol_losses(patrol, destroyed, COOLDOWN_RUNS);
    }

    pub(crate) fn patrol_losses(&mut self, patrol: u64, destroyed: &[ShipKind], cooldown: u32) {
        let police = &mut self.navigation.police;
        let Some(index) = police.patrols.iter().position(|known| known.id == patrol) else {
            return;
        };
        let record = &mut police.patrols[index];
        for kind in destroyed {
            if let Some(slot) = record.ships.iter().position(|ship| ship == kind) {
                record.ships.remove(slot);
            }
        }
        record.cooldown = cooldown;
        record.pursuing = None;
        record.answering = None;
        if record.ships.is_empty() {
            let system = record.system;
            police.patrols.remove(index);
            let due = self.navigation.run.saturating_add(RESPAWN_RUNS);
            police.respawns.push((system, due));
        }
    }

    /// Save check of the patrols on their own; `validate_factions` checks
    /// them against territory.
    pub fn validate_police(&self, systems: &[StarSystem]) -> Result<(), String> {
        let police = &self.navigation.police;
        let half = self.movement_rules.system_half_extent_km;
        let inside =
            |at: LocalPosition| [at.x_km, at.y_km, at.z_km].iter().all(|v| v.abs() <= half);
        let mut ids = std::collections::HashSet::new();
        let mut held = std::collections::HashSet::new();
        for patrol in &police.patrols {
            if patrol.id == 0
                || patrol.id >= police.next_id
                || !ids.insert(patrol.id)
                || !held.insert(patrol.system)
                || !systems.iter().any(|known| known.id == patrol.system)
                || !inside(patrol.position)
                || !inside(patrol.heading)
                || patrol.ships.is_empty()
                || patrol.ships.len() > PATROL_SHIPS.len()
                || patrol.cooldown > BAND_FIGHT_COOLDOWN_RUNS
                || (patrol.pursuing.is_some() && patrol.answering.is_some())
            {
                return Err("Invalid police patrol".into());
            }
        }
        let respawning = police
            .respawns
            .iter()
            .all(|(system, _)| held.insert(*system));
        let contacts = self.navigation.encounters.iter().all(|encounter| {
            let police = encounter.police.and_then(|contact| {
                police
                    .patrol(contact.patrol)
                    .filter(|patrol| patrol.faction == contact.faction)
            });
            match police {
                Some(patrol) => {
                    encounter.kind == EncounterKind::Police
                        && encounter.hostiles == patrol.ships
                        && encounter.band.is_none()
                        && encounter.trader.is_none()
                        && !encounter.haven
                }
                None => encounter.police.is_none() && encounter.kind != EncounterKind::Police,
            }
        });
        if !respawning || !contacts {
            return Err("Invalid police patrol".into());
        }
        Ok(())
    }

    /// Save check against factions: each patrol (and replacement) belongs to
    /// the holder of its system.
    pub fn validate_police_factions(&self, factions: &Factions) -> Result<(), String> {
        let police = &self.navigation.police;
        let held = police
            .patrols
            .iter()
            .all(|patrol| factions.owner(patrol.system) == Some(patrol.faction));
        let replaced = police
            .respawns
            .iter()
            .all(|(system, _)| factions.owner(*system).is_some());
        if !held || !replaced {
            return Err("Police patrol outside its faction's territory".into());
        }
        Ok(())
    }
}

/// Where `fleet`'s lead ship stands relative to `world`'s star, if it is
/// out in the open inside that system (not docked or landed).
pub(crate) fn exposed_at(
    state: &TravelState,
    fleet: FleetId,
    world: &StarSystem,
    systems: &[StarSystem],
) -> Option<LocalPosition> {
    let record = state.fleets.iter().find(|record| record.id == fleet)?;
    let ship = state.ship(*record.ships.first()?).ok()?;
    if matches!(
        ship.location,
        crate::travel::Location::Docked(_) | crate::travel::Location::Landed { .. }
    ) {
        return None;
    }
    let at = state.navigation_position(ship.id, systems)?;
    local_in(world, at, state.movement_rules.system_half_extent_km)
}

#[cfg(test)]
mod tests;
