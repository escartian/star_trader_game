use super::{
    KM_PER_LIGHT_YEAR, MoveTarget, MovementError, MovementMode, MovementPlan, SpacePosition,
    geometry,
};
use crate::{
    BodyKind, LocalPosition, StarSystem, SystemId,
    travel::{Location, PlayerId, ShipId, TravelError, TravelState},
};

impl TravelState {
    pub fn plan_movement(
        &self,
        player: PlayerId,
        ship: ShipId,
        target: &MoveTarget,
        systems: &[StarSystem],
    ) -> Result<MovementPlan, crate::fleet::FleetMovementError> {
        self.movement_rules.validate_world(systems)?;
        self.authorize(player, ship)?;
        let fleet = self.fleet_for_ship(ship)?;
        let origin = self.fleet_position(fleet.id, player)?.clone();
        self.movement_rules.validate_location(&origin, systems)?;
        let (destination, mode, distance_km) = self.destination(&origin, target, systems)?;
        if destination == origin {
            return Err(MovementError::AlreadyThere.into());
        }
        let deep = matches!(mode, MovementMode::Entry | MovementMode::DeepSpace);
        let cost_unit = if deep {
            KM_PER_LIGHT_YEAR as u64
        } else {
            self.movement_rules.local_km_per_fuel
        };
        let fuel_per_ship = u32::try_from(distance_km.div_ceil(cost_unit).max(1))
            .map_err(|_| MovementError::Coordinates)?;
        let duration_ticks = if deep {
            distance_km.div_ceil(KM_PER_LIGHT_YEAR as u64).max(1)
        } else {
            distance_km
                .div_ceil(self.movement_rules.local_km_per_tick)
                .max(1)
        };
        for id in &fleet.ships {
            let member = self.ship(*id)?;
            if deep {
                let start = match origin {
                    Location::DeepSpace(position) => position,
                    _ => return Err(MovementError::WrongSpace.into()),
                };
                let end = match destination {
                    Location::DeepSpace(position) => position,
                    Location::InSystem { system, position } => {
                        geometry::absolute(system_center(systems, system)?, position)?
                    }
                    _ => return Err(MovementError::WrongSpace.into()),
                };
                let range = u128::from(member.range_ly) * KM_PER_LIGHT_YEAR as u128;
                if geometry::distance_squared(start.coordinates(), end.coordinates())?
                    > range.checked_mul(range).ok_or(MovementError::Coordinates)?
                {
                    return Err(TravelError::OutOfRange.into());
                }
            }
            if member.fuel < fuel_per_ship {
                return Err(TravelError::InsufficientFuel.into());
            }
        }
        self.tick
            .checked_add(duration_ticks)
            .ok_or(TravelError::TimeOverflow)?;
        self.revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        Ok(MovementPlan {
            fleet: fleet.id,
            origin,
            destination,
            mode,
            distance_km,
            fuel_per_ship,
            duration_ticks,
            revision: self.revision,
        })
    }

    fn destination(
        &self,
        origin: &Location,
        target: &MoveTarget,
        systems: &[StarSystem],
    ) -> Result<(Location, MovementMode, u64), MovementError> {
        let rules = &self.movement_rules;
        match target {
            MoveTarget::Body { system, body } => {
                let start = local_position(origin, *system, systems)?;
                let destination = systems
                    .iter()
                    .find(|known| known.id == *system)
                    .and_then(|known| known.bodies.iter().find(|known| known.id == *body))
                    .ok_or(MovementError::UnknownBody)?;
                let location = if destination.kind == BodyKind::Station {
                    // Until per-planet markets, only the primary trade station has services.
                    if systems
                        .iter()
                        .find(|known| known.id == *system)
                        .and_then(|known| {
                            known
                                .bodies
                                .iter()
                                .find(|body| body.kind == BodyKind::Station)
                        })
                        .map(|body| body.id)
                        != Some(*body)
                    {
                        return Err(MovementError::UnknownBody);
                    }
                    Location::Docked(*system)
                } else {
                    Location::Orbiting {
                        system: *system,
                        body: *body,
                    }
                };
                self.local_destination(start, destination.position, location, MovementMode::Local)
            }
            MoveTarget::Local { system, position } => {
                let start = local_position(origin, *system, systems)?;
                if geometry::local(*position)
                    .iter()
                    .all(|value| value.unsigned_abs() <= rules.system_half_extent_km as u64)
                {
                    self.local_destination(
                        start,
                        *position,
                        Location::InSystem {
                            system: *system,
                            position: *position,
                        },
                        MovementMode::Local,
                    )
                } else {
                    let outside = geometry::exit(start, *position, rules.system_half_extent_km)?;
                    let absolute = geometry::absolute(system_center(systems, *system)?, outside)?;
                    if absolute
                        .coordinates()
                        .iter()
                        .any(|value| !rules.contains_space(*value))
                    {
                        return Err(MovementError::OutsideGalaxy);
                    }
                    self.local_destination(
                        start,
                        outside,
                        Location::DeepSpace(absolute),
                        MovementMode::Exit,
                    )
                }
            }
            MoveTarget::DeepSpace(position) => {
                let Location::DeepSpace(start) = origin else {
                    return Err(MovementError::WrongSpace);
                };
                let end = SpacePosition::from_galaxy(*position)?;
                if end
                    .coordinates()
                    .iter()
                    .any(|value| !rules.contains_space(*value))
                {
                    return Err(MovementError::OutsideGalaxy);
                }
                // Explicitly targeting a known system coordinate enters it; a
                // path passing through that coordinate never intercepts it.
                if let Some(system) = systems.iter().find(|system| system.position == *position) {
                    return self.destination(origin, &MoveTarget::EnterSystem(system.id), systems);
                }
                Ok((
                    Location::DeepSpace(end),
                    MovementMode::DeepSpace,
                    geometry::distance(start.coordinates(), end.coordinates())?,
                ))
            }
            MoveTarget::EnterSystem(system) => {
                let Location::DeepSpace(start) = origin else {
                    return Err(MovementError::WrongSpace);
                };
                let center = system_center(systems, *system)?;
                let entry = geometry::entry(*start, center, rules.system_half_extent_km)?;
                let end = geometry::absolute(center, entry)?;
                Ok((
                    Location::InSystem {
                        system: *system,
                        position: entry,
                    },
                    MovementMode::Entry,
                    geometry::distance(start.coordinates(), end.coordinates())?,
                ))
            }
        }
    }

    fn local_destination(
        &self,
        start: LocalPosition,
        end: LocalPosition,
        destination: Location,
        mode: MovementMode,
    ) -> Result<(Location, MovementMode, u64), MovementError> {
        if geometry::crosses_star(start, end, self.movement_rules.star_exclusion_radius_km) {
            return Err(MovementError::StarCollision);
        }
        Ok((
            destination,
            mode,
            geometry::distance(geometry::local(start), geometry::local(end))?,
        ))
    }

    /// Apply an authority-generated plan. Host resolution consumes ticks atomically;
    /// no renderer or untrusted client can supply a plan directly.
    pub fn resolve_movement(
        &mut self,
        player: PlayerId,
        plan: &MovementPlan,
    ) -> Result<(), crate::fleet::FleetMovementError> {
        if self.revision != plan.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let location = self.fleet_position(plan.fleet, player)?;
        if location != &plan.origin {
            return Err(TravelError::StaleQuote.into());
        }
        let members = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == plan.fleet)
            .ok_or(crate::fleet::FleetError::UnknownFleet)?
            .ships
            .clone();
        for id in &members {
            self.ship(*id)?
                .fuel
                .checked_sub(plan.fuel_per_ship)
                .ok_or(TravelError::InsufficientFuel)?;
        }
        self.advance(plan.duration_ticks)?;
        for id in members {
            let ship = self.ship_mut(id)?;
            ship.fuel -= plan.fuel_per_ship;
            ship.location = plan.destination.clone();
        }
        Ok(())
    }
}

fn system_center(systems: &[StarSystem], id: SystemId) -> Result<SpacePosition, MovementError> {
    SpacePosition::from_galaxy(
        systems
            .iter()
            .find(|system| system.id == id)
            .ok_or(MovementError::WrongSpace)?
            .position,
    )
}

pub fn local_position(
    location: &Location,
    system: SystemId,
    systems: &[StarSystem],
) -> Result<LocalPosition, MovementError> {
    let world = systems
        .iter()
        .find(|known| known.id == system)
        .ok_or(MovementError::WrongSpace)?;
    match location {
        Location::Docked(known) if *known == system => world
            .bodies
            .iter()
            .find(|body| body.kind == BodyKind::Station)
            .map(|body| body.position)
            .ok_or(MovementError::UnknownBody),
        Location::Orbiting {
            system: known,
            body,
        } if *known == system => world
            .bodies
            .iter()
            .find(|known| known.id == *body)
            .map(|body| body.position)
            .ok_or(MovementError::UnknownBody),
        Location::InSystem {
            system: known,
            position,
        } if *known == system => Ok(*position),
        _ => Err(MovementError::WrongSpace),
    }
}
