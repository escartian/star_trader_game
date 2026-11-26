//! Landing on and lifting off bodies, and mining while landed. Which bodies
//! allow landing, what it costs and what is mined: `movement::LandingRules`.
//! Landing and lifting off are immediate (no navigation run); mining happens
//! once per run for every waiting fleet on a moon or asteroid.
use super::{NavigationError, NavigationEvent};
use crate::{
    BodyId, StarSystem, SystemId,
    economy::Commodity,
    fleet::{FleetError, FleetId, FleetMovementError},
    movement::{MovementError, lift_off_fuel},
    travel::{Location, PlayerId, TravelError, TravelState},
};

impl TravelState {
    /// Sets an orbiting fleet down on its body; each ship pays the landing fuel.
    pub fn land(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
        systems: &[StarSystem],
    ) -> Result<NavigationEvent, FleetMovementError> {
        let Location::Orbiting { system, body } = self.fleet_position(fleet, player)?.clone()
        else {
            return Err(NavigationError::NotInOrbit.into());
        };
        let kind = landed_body(systems, system, body)?.kind;
        let fuel = self
            .movement_rules
            .landing
            .fuel(kind)
            .ok_or(NavigationError::CannotLand)?;
        self.relocate_fleet(fleet, fuel, Location::Landed { system, body })?;
        Ok(NavigationEvent::Landed { fleet, body })
    }

    /// Lifts a landed fleet back into orbit; each ship pays the same fuel again.
    pub fn lift_off(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
        systems: &[StarSystem],
    ) -> Result<NavigationEvent, FleetMovementError> {
        let origin = self.fleet_position(fleet, player)?.clone();
        let Location::Landed { system, body } = origin else {
            return Err(NavigationError::NotLanded.into());
        };
        let fuel = lift_off_fuel(&self.movement_rules, &origin, systems);
        self.relocate_fleet(fleet, fuel, Location::Orbiting { system, body })?;
        Ok(NavigationEvent::LiftedOff { fleet, body })
    }

    /// Moves every ship of an idle fleet to `location`, charging `fuel` each;
    /// all or nothing.
    fn relocate_fleet(
        &mut self,
        fleet: FleetId,
        fuel: u32,
        location: Location,
    ) -> Result<(), FleetMovementError> {
        if self.navigation.order(fleet).is_some() {
            return Err(NavigationError::FleetHasCourse.into());
        }
        if self
            .navigation
            .encounters
            .iter()
            .any(|encounter| encounter.fleet == fleet)
        {
            return Err(NavigationError::EncounterPending(fleet).into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let members = self
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?
            .ships
            .clone();
        for id in &members {
            if self.ship(*id)?.fuel < fuel {
                return Err(TravelError::InsufficientFuel.into());
            }
        }
        for id in members {
            let ship = self.ship_mut(id)?;
            ship.fuel -= fuel;
            ship.location = location.clone();
        }
        self.revision = revision;
        Ok(())
    }

    /// Run step: every fleet waiting on a moon or asteroid mines ore into each
    /// ship's free cargo space.
    pub(super) fn mine(
        &mut self,
        systems: &[StarSystem],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        let mut events = Vec::new();
        for fleet in self.fleets.clone() {
            if !self.navigation.holds.contains(&fleet.id) {
                continue;
            }
            let Some(first) = fleet.ships.first() else {
                continue;
            };
            let Location::Landed { system, body } = self.ship(*first)?.location.clone() else {
                continue;
            };
            let place = landed_body(systems, system, body)?;
            let base = self.movement_rules.landing.ore_per_ship(place.kind);
            // A surface's richness scales the kind's yield.
            let per_ship = place
                .surface
                .as_ref()
                .map_or(base, |surface| surface.ore_per_ship(base));
            if per_ship == 0 {
                continue;
            }
            let mut ore = 0;
            for id in &fleet.ships {
                let ship = self.ship_mut(*id)?;
                let free = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
                let bonus = if ship
                    .gear
                    .contains(&crate::combat::gear::Gear::ProspectorDrill)
                {
                    crate::combat::gear::PROSPECTOR_ORE_PER_RUN
                } else {
                    0
                };
                let mined = per_ship
                    .saturating_add(bonus)
                    .min(u32::try_from(free).unwrap_or(u32::MAX));
                ship.cargo[Commodity::Ore.index()] += mined;
                ore += mined;
            }
            events.push(NavigationEvent::Mined {
                fleet: fleet.id,
                body,
                ore,
            });
        }
        Ok(events)
    }
}

fn landed_body(
    systems: &[StarSystem],
    system: SystemId,
    body: BodyId,
) -> Result<&crate::LocalBody, MovementError> {
    systems
        .iter()
        .find(|known| known.id == system)
        .and_then(|known| known.bodies.iter().find(|known| known.id == body))
        .ok_or(MovementError::UnknownBody)
}
