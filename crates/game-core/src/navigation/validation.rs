use crate::{
    StarSystem,
    fleet::FleetId,
    travel::{Location, TravelState},
};
use std::collections::HashSet;

impl TravelState {
    pub fn validate_navigation(&self, systems: &[StarSystem]) -> Result<(), String> {
        if crate::travel::clean_captain_name(&self.captain_name) != self.captain_name {
            return Err("Invalid captain name".into());
        }
        if self.navigation.orders.len() > self.fleets.len()
            || self.navigation.encounters.len() > self.fleets.len()
            || u128::from(self.navigation.run)
                * u128::from(self.movement_rules.ticks_per_navigation_run)
                > u128::from(self.tick)
            || self.navigation.next_encounter_id == 0
        {
            return Err("Invalid navigation counters".into());
        }
        let mut ordered = HashSet::new();
        for order in &self.navigation.orders {
            if !ordered.insert(order.plan.fleet)
                || order.remaining_ticks == 0
                || order.remaining_ticks > order.plan.duration_ticks
                || (!order.started && order.remaining_ticks != order.plan.duration_ticks)
                || order.remaining_waypoints.len() > 32
                || order.plan.revision > self.revision
                || !order.plan.origin.stationary()
                || !order.plan.destination.stationary()
            {
                return Err("Invalid navigation course".into());
            }
            let fleet = self
                .fleets
                .iter()
                .find(|fleet| fleet.id == order.plan.fleet)
                .ok_or("Unknown course fleet")?;
            let ship = *fleet.ships.first().ok_or("Empty course fleet")?;
            let expected = if order.started {
                Location::InTransit(fleet.id)
            } else {
                order.plan.origin.clone()
            };
            let mut preview = self.clone();
            if order.plan.tick > self.tick {
                return Err("Course planned in the future".into());
            }
            // Re-plan where the bodies stood when the course was computed.
            preview.tick = order.plan.tick;
            for id in &fleet.ships {
                let member = preview.ship_mut(*id).map_err(|_| "Unknown course member")?;
                if member.location != expected {
                    return Err("Course location differs from fleet location".into());
                }
                member.location = order.plan.origin.clone();
                if order.started {
                    member.fuel = member
                        .fuel
                        .checked_add(order.plan.fuel_per_ship)
                        .filter(|fuel| *fuel <= member.fuel_capacity)
                        .ok_or("Invalid paid course fuel")?;
                }
            }
            let mut checked = preview
                .plan_movement(fleet.owner, ship, &order.target, systems)
                .map_err(|error| error.to_string())?;
            checked.revision = order.plan.revision;
            if checked != order.plan {
                return Err("Saved course differs from authoritative geometry".into());
            }
            for target in std::iter::once(&order.target).chain(order.remaining_waypoints.iter()) {
                use crate::{fleet::FleetMovementError, movement::MovementError};
                let plan = match preview.plan_movement(fleet.owner, ship, target, systems) {
                    Ok(plan) => plan,
                    // Bodies keep orbiting on the plane: a later leg that is
                    // blocked now is re-routed when it starts (see `next_leg`).
                    Err(FleetMovementError::Movement(
                        MovementError::BodyCollision | MovementError::StarCollision,
                    )) => break,
                    Err(error) => return Err(error.to_string()),
                };
                for id in &fleet.ships {
                    let member = preview.ship_mut(*id).map_err(|_| "Unknown course ship")?;
                    member.fuel -= plan.fuel_per_ship;
                    member.location = plan.destination.clone();
                }
            }
        }
        for ship in &self.ships {
            if let Location::InTransit(fleet) = ship.location
                && !self.navigation.order(fleet).is_some_and(|order| {
                    order.started
                        && self
                            .fleet_for_ship(ship.id)
                            .is_ok_and(|known| known.id == fleet)
                })
            {
                return Err("Ship has no matching active course".into());
            }
        }
        let mut held = HashSet::new();
        for fleet in &self.navigation.holds {
            if !held.insert(*fleet)
                || ordered.contains(fleet)
                || self
                    .fleets
                    .iter()
                    .find(|record| record.id == *fleet)
                    .is_none_or(|record| self.fleet_position(*fleet, record.owner).is_err())
            {
                return Err("Invalid Wait order".into());
            }
        }
        let known_stop = |stop: &crate::economy::MarketId| {
            systems.iter().any(|system| {
                system.id == stop.system && system.bodies.iter().any(|body| body.id == stop.body)
            })
        };
        let mut routed = HashSet::new();
        for route in &self.navigation.routes {
            if !routed.insert(route.fleet)
                || held.contains(&route.fleet)
                || route.stops[0] == route.stops[1]
                || !route.stops.iter().all(known_stop)
                || !self.fleets.iter().any(|fleet| fleet.id == route.fleet)
            {
                return Err("Invalid trade route".into());
            }
        }
        let mut encounters = HashSet::new();
        let mut encounter_fleets: HashSet<FleetId> = HashSet::new();
        for encounter in &self.navigation.encounters {
            if encounter.id == 0
                || encounter.id >= self.navigation.next_encounter_id
                || !encounters.insert(encounter.id)
                || !encounter_fleets.insert(encounter.fleet)
                || encounter.run == 0
                || encounter.run > self.navigation.run
                || (encounter.kind.with_trader()
                    || matches!(
                        encounter.kind,
                        super::EncounterKind::Raiders
                            | super::EncounterKind::Police
                            | super::EncounterKind::TraderDistress
                    ))
                    == encounter.hostiles.is_empty()
                || (encounter.kind == super::EncounterKind::TraderDistress)
                    != (encounter.rescue.is_some() && encounter.band.is_some())
                || encounter.kind.with_trader() != encounter.trader.is_some()
                || encounter.hostiles.len() > super::hostile::MAX_SAVED_RAIDERS
                || !self.encounter_source_valid(encounter)
            {
                return Err("Invalid fleet encounter".into());
            }
        }
        let mut escorted = HashSet::new();
        for escort in &self.navigation.escorts {
            if !escorted.insert(escort.fleet)
                || !(1..=crate::combat::gear::ESCORT_RUNS).contains(&escort.runs_left)
                || !self.fleets.iter().any(|fleet| fleet.id == escort.fleet)
            {
                return Err("Invalid escort contract".into());
            }
        }
        for ship in &self.ships {
            if !crate::combat::gear::valid_fit(&ship.gear) {
                return Err("Invalid fitted gear".into());
            }
        }
        if !self
            .fleets
            .iter()
            .all(|fleet| crate::fleet::software::valid_install(&fleet.software))
        {
            return Err("Invalid fleet software".into());
        }
        self.validate_bands(systems)?;
        self.validate_haven(systems)?;
        self.validate_patrols(systems)?;
        self.validate_police(systems)?;
        if self.navigation.payout != 0 {
            return Err("Unsettled contact payout".into());
        }
        if let Some(report) = &self.navigation.last_combat {
            report.validate()?;
            if report.encounter >= self.navigation.next_encounter_id
                || report.run > self.navigation.run
            {
                return Err("Combat report refers to a future encounter".into());
            }
        }
        Ok(())
    }

    /// A random contact pauses a fleet under way; a band's strike may also
    /// catch a waiting fleet, but its band must still exist in that system;
    /// the player may hail a trader from anywhere it is in reach.
    fn encounter_source_valid(&self, encounter: &super::Encounter) -> bool {
        // A hailed trader: its hull (and escort), checked against the trader
        // by the host.
        // A police patrol may stop any exposed fleet; `validate_police`
        // checks the patrol itself.
        if encounter.police.is_some() {
            return encounter.trader.is_none() && encounter.band.is_none() && !encounter.haven;
        }
        if let Some(trader) = &encounter.trader {
            return !encounter.haven
                && encounter.band.is_none()
                && encounter.hostiles.len() == 1 + usize::from(trader.escort)
                && !trader.name.is_empty()
                && trader.name.len() <= 128;
        }
        if encounter.haven {
            return encounter.band.is_none()
                && self
                    .navigation
                    .haven
                    .as_ref()
                    .is_some_and(|haven| haven.standing() && haven.garrison == encounter.hostiles);
        }
        match encounter.band {
            None => self
                .navigation
                .order(encounter.fleet)
                .is_some_and(|order| order.started),
            Some(band) => {
                matches!(
                    encounter.kind,
                    super::EncounterKind::Raiders | super::EncounterKind::TraderDistress
                ) && self
                    .navigation
                    .bands
                    .iter()
                    .any(|known| known.id == band && known.ships == encounter.hostiles)
            }
        }
    }

    fn validate_bands(&self, systems: &[StarSystem]) -> Result<(), String> {
        let navigation = &self.navigation;
        let half = self.movement_rules.system_half_extent_km;
        let inside = |position: crate::LocalPosition| {
            position.y_km == 0
                && [position.x_km, position.z_km]
                    .iter()
                    .all(|value| value.unsigned_abs() <= half as u64)
        };
        let known_system = |id| systems.iter().any(|system| system.id == id);
        let mut ids = HashSet::new();
        for band in &navigation.bands {
            if band.id == 0
                || band.id >= navigation.next_band_id
                || !ids.insert(band.id)
                || !known_system(band.system)
                || !inside(band.position)
                || !inside(band.heading)
                || band.ships.is_empty()
                || band.ships.len() > super::hostile::MAX_SAVED_RAIDERS
                || band.cooldown > super::raiders::LOOT_COOLDOWN_RUNS
                || band
                    .hunting
                    .is_some_and(|fleet| !self.fleets.iter().any(|known| known.id == fleet))
                || (band.hunting.is_some() && band.stalking.is_some())
                || band
                    .stalking
                    .is_some_and(|trader| !(1..=16).contains(&trader))
                || band.transit.is_some_and(|transit| {
                    transit.from == band.system
                        || !known_system(transit.from)
                        || !(1..=super::band_roaming::MAX_TRANSIT_RUNS)
                            .contains(&transit.total_runs)
                        || !(1..=transit.total_runs).contains(&transit.runs_left)
                        || band.stalking.is_some()
                        || band.cooldown > 0
                })
                || navigation
                    .bands
                    .iter()
                    .filter(|other| other.system == band.system)
                    .count()
                    > super::MAX_BANDS_PER_SYSTEM
            {
                return Err("Invalid raider band".into());
            }
        }
        if navigation.band_respawns.iter().any(|(system, at)| {
            !known_system(*system)
                || *at > navigation.run.saturating_add(super::raiders::RESPAWN_RUNS)
        }) || navigation.bands.len() + navigation.band_respawns.len() > systems.len()
        {
            return Err("Invalid raider band respawn".into());
        }
        Ok(())
    }
}
