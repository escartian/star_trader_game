//! Roaming raider bands: persistent hostile fleets the player can see coming.
//!
//! - Each system but the home system holds one band: two scouts, or a patrol
//!   ship leading a scout one time in three. Bands may cross to other systems,
//!   chasing a fleet that fled or roaming (`band_roaming`).
//! - Every run, after fleets move, each band hunts the nearest exposed player
//!   fleet within [`DETECTION_KM`] (fleets docked at a station or landed on a
//!   surface are safe), or stalks a nearer NPC trader in the open ([`Prey`],
//!   where traders stood before the run), or patrols near a body, preferring
//!   dangerous planets.
//!   It moves up to [`BAND_KM_PER_RUN`], around planets rather than through.
//! - Hunting your fleet, it chases on a curve through the run
//!   (`navigation::chase`) and strikes once within [`STRIKE_KM`]: a raider
//!   contact with the
//!   band's real ships (Fight, Auto-resolve or Evade). A stalked trader is
//!   struck once traders have moved, if it is still that close and in the open
//!   (`npc::band_raids`).
//! - Destroyed raiders leave the band; afterwards it backs off for
//!   [`COOLDOWN_RUNS`] ([`LOOT_COOLDOWN_RUNS`] after striking a trader). A band wiped out returns after [`RESPAWN_RUNS`], but
//!   only while the Raider Haven stands (`haven`).
//! - Fleets on a chartered shipping lane (`economy::charters`) are never hunted.
//!
//! Bands draw from their own random stream, so they never change the outcome
//! of other contact rolls. With `encounter_chance_percent` at 0 (a peaceful
//! game, or a quiet test voyage) bands still roam but never strike.
use super::band_motion::{distance, local_in, near, step_toward};
use super::{Encounter, EncounterKind, NavigationEvent};
use crate::{
    BodyKind, LocalPosition, StarSystem, SystemId,
    fleet::{FleetId, FleetMovementError, ShipKind},
    movement::SpacePosition,
    planet::Danger,
    travel::{Location, PlayerId, TravelError, TravelState},
};
use serde::{Deserialize, Serialize};

/// Distance a band covers per run, in km: half a fleet's local speed.
pub const BAND_KM_PER_RUN: i64 = 100_000_000;
/// A band hunts the nearest exposed player fleet within this distance, in km.
pub const DETECTION_KM: i64 = 150_000_000;
/// Close enough to strike one of your fleets (or for police to stop it), in
/// km. Hunters chase on a curve within each run (`chase`), so they close in
/// rather than needing a wide reach.
pub const STRIKE_KM: i64 = 500_000;
/// Reach between NPCs: a band robbing a trader, police engaging a band.
/// Wider, as these still close in one straight step per run.
pub const NPC_STRIKE_KM: i64 = 5_000_000;
/// Runs a band backs off after a fight or an escape.
pub const COOLDOWN_RUNS: u32 = 3;
/// Runs a band lies low after striking an NPC trader, fencing the loot.
pub const LOOT_COOLDOWN_RUNS: u32 = 12;
/// Runs before a wiped-out band's system gets a new band.
pub const RESPAWN_RUNS: u64 = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaiderBand {
    pub id: u64,
    pub system: SystemId,
    pub position: LocalPosition,
    pub ships: Vec<ShipKind>,
    /// Where the band is going: its quarry, or a patrol point.
    pub heading: LocalPosition,
    /// The player fleet it is chasing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hunting: Option<FleetId>,
    /// The NPC trader it is chasing (`npc::Trader::id`); never with `hunting`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stalking: Option<u64>,
    /// Runs left backing off (no hunting) after a fight or an escape.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cooldown: u32,
    /// Set while the band crosses to `system` from another system; it then
    /// neither hunts nor strikes, and `position` is where it will arrive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transit: Option<super::BandTransit>,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// An NPC trader out in the open, where it stood before the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prey {
    pub trader: u64,
    pub position: SpacePosition,
}

/// What a band chases this run.
#[derive(Clone, Copy)]
enum Quarry {
    Fleet(FleetId),
    Trader(u64),
}

impl TravelState {
    /// New-game setup: one band in every system but the home (first) system,
    /// and the Raider Haven they come from.
    pub fn spawn_raider_bands(&mut self, systems: &[StarSystem]) {
        self.place_haven(systems);
        self.navigation.band_rng = self.navigation.rng_state ^ 0x5bd1_e995;
        for system in systems.iter().skip(1) {
            self.spawn_band(system);
        }
    }

    fn spawn_band(&mut self, system: &StarSystem) {
        let roll = self.band_roll();
        let mut ships = if roll.is_multiple_of(3) {
            vec![ShipKind::Patrol, ShipKind::Scout]
        } else {
            vec![ShipKind::Scout, ShipKind::Scout]
        };
        // Perilous games field bigger bands.
        let extra = self.movement_rules.raider_band_extra as usize;
        ships.extend(std::iter::repeat_n(ShipKind::Scout, extra));
        let position = self.patrol_point(system);
        let id = self.navigation.next_band_id;
        self.navigation.next_band_id += 1;
        self.navigation.bands.push(RaiderBand {
            id,
            system: system.id,
            position,
            heading: position,
            ships,
            hunting: None,
            stalking: None,
            cooldown: 0,
            transit: None,
        });
    }

    pub(crate) fn band_roll(&mut self) -> u64 {
        self.navigation.band_rng = self.navigation.band_rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.navigation.band_rng;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// A point beside a body, chosen by danger (stations are avoided).
    fn patrol_point(&mut self, system: &StarSystem) -> LocalPosition {
        let weight = |body: &crate::LocalBody| match (body.kind, body.danger()) {
            (BodyKind::Station, _) => 0,
            (_, Some(danger)) => match danger {
                Danger::VerySafe | Danger::Safe => 1,
                Danger::Moderate => 2,
                Danger::Dangerous => 4,
                Danger::VeryDangerous => 6,
                Danger::Insidious => 8,
            },
            _ => 1,
        };
        let total: u64 = system.bodies.iter().map(weight).sum();
        let mut pick = self.band_roll() % total.max(1);
        let body = system
            .bodies
            .iter()
            .find(|body| {
                let w = weight(body);
                if pick < w {
                    return true;
                }
                pick -= w;
                false
            })
            .or(system.bodies.first());
        let Some(body) = body else {
            return LocalPosition {
                x_km: self.movement_rules.system_half_extent_km / 2,
                y_km: 0,
                z_km: 0,
            };
        };
        // Hold station well outside the body's volume, on the plane, on the
        // side away from the star.
        let offset = self.movement_rules.body_radii.of(body.kind) * 4;
        let outward = if body.position.x_km < 0 {
            -offset
        } else {
            offset
        };
        LocalPosition {
            x_km: body.position.x_km + outward,
            ..body.position
        }
    }

    /// Run step, after fleets moved: bands back off, hunt or patrol, and strike.
    pub(super) fn move_bands(
        &mut self,
        player: PlayerId,
        run: u64,
        systems: &[StarSystem],
        on_lane: &[FleetId],
        prey: &[Prey],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        // Bands come from the Raider Haven: once it falls, none return.
        if self
            .navigation
            .haven
            .as_ref()
            .is_some_and(|haven| !haven.standing())
        {
            self.navigation.band_respawns.clear();
        }
        let due: Vec<SystemId> = self
            .navigation
            .band_respawns
            .iter()
            .filter(|(_, at)| *at <= run)
            .map(|(system, _)| *system)
            .collect();
        self.navigation.band_respawns.retain(|(_, at)| *at > run);
        for system in due {
            if !self.band_room(system) {
                // Full of roaming bands: try again later.
                let later = self.navigation.run.saturating_add(RESPAWN_RUNS);
                self.navigation.band_respawns.push((system, later));
            } else if let Some(world) = systems.iter().find(|known| known.id == system) {
                self.spawn_band(world);
            }
        }
        let mut events = Vec::new();
        for index in 0..self.navigation.bands.len() {
            if self.advance_transit(index) {
                continue;
            }
            let band = self.navigation.bands[index].clone();
            let Some(world) = systems.iter().find(|known| known.id == band.system) else {
                continue;
            };
            let quarry = if band.cooldown == 0 {
                let fleet = self.nearest_exposed(player, &band, world, systems, on_lane);
                let half = self.movement_rules.system_half_extent_km;
                let trader = nearest_prey(&band, world, prey, half);
                // Your fleets win ties: the band was hunting you first.
                match (fleet, trader) {
                    (Some((gap, fleet, at)), Some((other, _, _))) if gap <= other => {
                        Some((Quarry::Fleet(fleet), at))
                    }
                    (_, Some((_, trader, at))) => Some((Quarry::Trader(trader), at)),
                    (Some((_, fleet, at)), None) => Some((Quarry::Fleet(fleet), at)),
                    (None, None) => None,
                }
            } else {
                None
            };
            if quarry.is_none()
                && let Some(destination) = self.crossing(&band, player, systems)
            {
                self.send_band(index, destination, systems);
                continue;
            }
            let chasing = band.hunting.is_some() || band.stalking.is_some();
            let heading = match quarry {
                Some((_, position)) => position,
                None if chasing || near(band.position, band.heading) => self.patrol_point(world),
                None => band.heading,
            };
            let peaceful = self.movement_rules.encounter_chance_percent == 0;
            // Hunting your fleet: chase it on a curve from where it was last
            // seen (the band's heading) to where it is now.
            let hunt = match quarry {
                Some((Quarry::Fleet(fleet), now)) if !peaceful => {
                    let seen = if band.hunting == Some(fleet) {
                        band.heading
                    } else {
                        now
                    };
                    Some(super::chase::chase(
                        band.position,
                        BAND_KM_PER_RUN,
                        (seen, now),
                        STRIKE_KM,
                        self.movement_rules.star_exclusion_radius_km,
                    ))
                }
                _ => None,
            };
            let position = match hunt {
                Some(hunt) => hunt.end,
                None => step_toward(&self.movement_rules, world, band.position, heading),
            };
            let record = &mut self.navigation.bands[index];
            record.cooldown = record.cooldown.saturating_sub(1);
            (record.hunting, record.stalking) = match quarry {
                Some((Quarry::Fleet(fleet), _)) => (Some(fleet), None),
                Some((Quarry::Trader(trader), _)) => (None, Some(trader)),
                None => (None, None),
            };
            record.heading = heading;
            record.position = position;
            if let (Some((Quarry::Fleet(fleet), _)), Some(true)) =
                (quarry, hunt.map(|hunt| hunt.caught))
            {
                events.push(self.strike(&band, fleet, run)?);
            }
        }
        Ok(events)
    }

    /// The closest player fleet in the band's system that is out in the open
    /// (not docked or landed, no contact pending) and within detection range.
    fn nearest_exposed(
        &self,
        player: PlayerId,
        band: &RaiderBand,
        world: &StarSystem,
        systems: &[StarSystem],
        on_lane: &[FleetId],
    ) -> Option<(i64, FleetId, LocalPosition)> {
        let centre = SpacePosition::from_galaxy(world.position).ok()?;
        let half = self.movement_rules.system_half_extent_km;
        self.fleets
            .iter()
            .filter(|fleet| fleet.owner == player && !on_lane.contains(&fleet.id))
            .filter(|fleet| {
                !self
                    .navigation
                    .encounters
                    .iter()
                    .any(|contact| contact.fleet == fleet.id)
            })
            .filter_map(|fleet| {
                let ship = self.ship(*fleet.ships.first()?).ok()?;
                if matches!(ship.location, Location::Docked(_) | Location::Landed { .. }) {
                    return None;
                }
                let at = self.navigation_position(ship.id, systems)?;
                let local = LocalPosition {
                    x_km: at.x_km.checked_sub(centre.x_km)?,
                    y_km: at.y_km.checked_sub(centre.y_km)?,
                    z_km: at.z_km.checked_sub(centre.z_km)?,
                };
                let inside = [local.x_km, local.y_km, local.z_km]
                    .iter()
                    .all(|value| value.abs() <= half);
                let gap = distance(band.position, local);
                (inside && gap <= DETECTION_KM).then_some((gap, fleet.id, local))
            })
            .min_by_key(|(gap, fleet, _)| (*gap, fleet.0))
    }

    pub(crate) fn strike(
        &mut self,
        band: &RaiderBand,
        fleet: FleetId,
        run: u64,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let next = self
            .navigation
            .next_encounter_id
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let encounter = Encounter {
            id: self.navigation.next_encounter_id,
            fleet,
            kind: EncounterKind::Raiders,
            run,
            hostiles: band.ships.clone(),
            band: Some(band.id),
            haven: false,
            trader: None,
            police: None,
            rescue: None,
        };
        self.navigation.next_encounter_id = next;
        self.navigation.encounters.push(encounter.clone());
        Ok(NavigationEvent::EncounterFound(encounter))
    }

    /// After a band's contact ends (battle or escape): its destroyed raiders
    /// leave it, and it backs off, or it is gone and will return later.
    pub(crate) fn band_after_contact(&mut self, band: u64, destroyed: &[ShipKind]) {
        self.band_after_fight(band, destroyed, COOLDOWN_RUNS);
        if self.navigation.bands.iter().all(|known| known.id != band) {
            self.jobs.record_cleared_band(band);
        }
    }

    /// After a band strikes an NPC trader: as [`Self::band_after_contact`],
    /// but it lies low for [`LOOT_COOLDOWN_RUNS`].
    pub(crate) fn band_after_raid(&mut self, band: u64, destroyed: &[ShipKind]) {
        self.band_after_fight(band, destroyed, LOOT_COOLDOWN_RUNS);
    }

    fn band_after_fight(&mut self, band: u64, destroyed: &[ShipKind], cooldown: u32) {
        let Some(index) = self
            .navigation
            .bands
            .iter()
            .position(|known| known.id == band)
        else {
            return;
        };
        let record = &mut self.navigation.bands[index];
        for kind in destroyed {
            if let Some(slot) = record.ships.iter().position(|ship| ship == kind) {
                record.ships.remove(slot);
            }
        }
        record.cooldown = cooldown;
        record.hunting = None;
        record.stalking = None;
        if record.ships.is_empty() {
            let system = record.system;
            self.navigation.bands.remove(index);
            let due = self.navigation.run.saturating_add(RESPAWN_RUNS);
            self.navigation.band_respawns.push((system, due));
        }
    }
}

/// The nearest trader within detection range inside the band's system:
/// (distance in km, trader, its position relative to the star).
fn nearest_prey(
    band: &RaiderBand,
    world: &StarSystem,
    prey: &[Prey],
    half_extent_km: i64,
) -> Option<(i64, u64, LocalPosition)> {
    prey.iter()
        .filter_map(|target| {
            let local = local_in(world, target.position, half_extent_km)?;
            let gap = distance(band.position, local);
            (gap <= DETECTION_KM).then_some((gap, target.trader, local))
        })
        .min_by_key(|(gap, trader, _)| (*gap, *trader))
}
