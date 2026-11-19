use crate::{
    StarSystem,
    fleet::FleetId,
    travel::{Location, TravelState},
};
use std::collections::HashSet;

impl TravelState {
    pub fn validate_navigation(&self, systems: &[StarSystem]) -> Result<(), String> {
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
                let plan = preview
                    .plan_movement(fleet.owner, ship, target, systems)
                    .map_err(|error| error.to_string())?;
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
        let mut encounters = HashSet::new();
        let mut encounter_fleets: HashSet<FleetId> = HashSet::new();
        for encounter in &self.navigation.encounters {
            if encounter.id == 0
                || encounter.id >= self.navigation.next_encounter_id
                || !encounters.insert(encounter.id)
                || !encounter_fleets.insert(encounter.fleet)
                || encounter.run == 0
                || encounter.run > self.navigation.run
                || (encounter.kind == super::EncounterKind::Raiders)
                    != !encounter.hostiles.is_empty()
                || encounter.hostiles.len() > super::hostile::MAX_RAIDERS
                || !self
                    .navigation
                    .order(encounter.fleet)
                    .is_some_and(|order| order.started)
            {
                return Err("Invalid fleet encounter".into());
            }
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
}
