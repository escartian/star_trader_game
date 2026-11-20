//! Patrols: a secondary fleet's standing order to guard one system. The fleet
//! flies there, then visits the system's bodies in turn, one course at a time,
//! so raider bands hunting the system meet it (secondary fleets' fights resolve
//! automatically and pay bounties). At a station or planet market it tops up
//! fuel when credits allow. A patrol that cannot plot its next leg (out of
//! fuel, for example) reports it and tries again on the next run.
use crate::{
    BodyId, StarSystem, SystemId,
    economy::Economy,
    fleet::{FleetError, FleetId, FleetMovementError},
    navigation::{NavigationError, NavigationEvent},
    travel::{PlayerId, TravelState},
};
use serde::{Deserialize, Serialize};

/// A fleet's standing order to guard `system`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patrol {
    pub fleet: FleetId,
    pub system: SystemId,
    /// Bodies visited so far; the next stop is the body at this count, in ID order.
    #[serde(default)]
    pub stops: u64,
}

impl TravelState {
    pub fn patrol(&self, fleet: FleetId) -> Option<&Patrol> {
        self.navigation
            .patrols
            .iter()
            .find(|patrol| patrol.fleet == fleet)
    }

    /// Removes a fleet's patrol; returns whether it had one.
    pub fn end_patrol(&mut self, fleet: FleetId) -> bool {
        let before = self.navigation.patrols.len();
        self.navigation
            .patrols
            .retain(|patrol| patrol.fleet != fleet);
        self.navigation.patrols.len() != before
    }

    /// Orders `fleet` to guard `system`, replacing a Wait or trade route. The
    /// flagship takes direct orders instead, so it cannot patrol.
    pub fn start_patrol(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
        system: SystemId,
        systems: &[StarSystem],
    ) -> Result<NavigationEvent, FleetMovementError> {
        let record = self
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?;
        if record.owner != player {
            return Err(FleetError::NotOwner.into());
        }
        if self.flagship(player) == Some(fleet) {
            return Err(NavigationError::FlagshipCommanded.into());
        }
        if !systems.iter().any(|known| known.id == system) {
            return Err(NavigationError::InvalidRouteStop.into());
        }
        self.end_trade_route(fleet);
        self.navigation.holds.retain(|held| *held != fleet);
        self.end_patrol(fleet);
        self.navigation.patrols.push(Patrol {
            fleet,
            system,
            stops: 0,
        });
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(crate::travel::TravelError::TimeOverflow)?;
        Ok(NavigationEvent::PatrolNote {
            fleet,
            note: "Patrol ordered.".into(),
        })
    }
}

impl TravelState {
    /// Rejects a flagship that is not a fleet, and patrols on unknown fleets or
    /// systems, on the flagship, or twice for one fleet.
    pub(crate) fn validate_patrols(&self, systems: &[StarSystem]) -> Result<(), String> {
        let exists = |id: FleetId| self.fleets.iter().any(|fleet| fleet.id == id);
        if self.navigation.flagship.is_some_and(|id| !exists(id)) {
            return Err("Invalid flagship".into());
        }
        let flagship = self
            .navigation
            .flagship
            .or_else(|| self.fleets.first().map(|fleet| fleet.id));
        let mut seen = std::collections::HashSet::new();
        for patrol in &self.navigation.patrols {
            if !exists(patrol.fleet)
                || !seen.insert(patrol.fleet)
                || Some(patrol.fleet) == flagship
                || !systems.iter().any(|known| known.id == patrol.system)
                || self.trade_route(patrol.fleet).is_some()
                || self.navigation.holds.contains(&patrol.fleet)
            {
                return Err("Invalid patrol".into());
            }
        }
        Ok(())
    }
}

/// The patrol's next body: the system's bodies in ID order, cycling.
fn next_stop(patrol: &Patrol, systems: &[StarSystem]) -> Option<BodyId> {
    let system = systems.iter().find(|known| known.id == patrol.system)?;
    let mut bodies: Vec<BodyId> = system.bodies.iter().map(|body| body.id).collect();
    bodies.sort_by_key(|body| body.0);
    let count = bodies.len() as u64;
    (count > 0).then(|| bodies[(patrol.stops % count) as usize])
}

/// After a run: every patrol fleet without a course or contact refuels where it
/// can and sets off for its next body.
pub fn service_patrols(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let mut patrols = state.navigation.patrols.clone();
    patrols.sort_by_key(|patrol| patrol.fleet.0);
    let mut events = Vec::new();
    for patrol in patrols {
        let fleet = patrol.fleet;
        let busy = state.navigation.order(fleet).is_some()
            || state
                .navigation
                .encounters
                .iter()
                .any(|encounter| encounter.fleet == fleet);
        let Some(record) = state
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .cloned()
        else {
            continue;
        };
        if busy || record.owner != player {
            continue;
        }
        for id in &record.ships {
            let Ok(ship) = state.ship_mut(*id) else {
                continue;
            };
            let missing = ship.fuel_capacity.saturating_sub(ship.fuel);
            if missing > 0 {
                // Best effort: a patrol without money for fuel keeps going.
                let units = missing.min(economy.max_refuel_units(ship).unwrap_or(0));
                if units > 0 {
                    let _ = economy.refuel(ship, units);
                }
            }
        }
        let Some(body) = next_stop(&patrol, systems) else {
            continue;
        };
        let lead = *record.ships.first().ok_or(FleetError::InvalidSelection)?;
        match state.plot_destination(player, lead, patrol.system, Some(body), systems) {
            Ok(_) => {
                if let Some(record) = state
                    .navigation
                    .patrols
                    .iter_mut()
                    .find(|known| known.fleet == fleet)
                {
                    record.stops += 1;
                }
            }
            Err(error) => events.push(NavigationEvent::PatrolNote {
                fleet,
                note: format!("Patrol held: {error}"),
            }),
        }
    }
    Ok(events)
}
