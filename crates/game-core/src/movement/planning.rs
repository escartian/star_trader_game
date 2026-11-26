use super::{
    KM_PER_LIGHT_YEAR, MoveTarget, MovementError, MovementMode, MovementPlan, SpacePosition,
    geometry, obstacles,
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
        // Plans always use bodies where they stand at this state's own tick.
        let placed = crate::orbits::placed(systems, self.tick);
        let systems = placed.as_slice();
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
        // A course from the surface starts by lifting off, at the body's cost.
        let fuel_per_ship = u32::try_from(distance_km.div_ceil(cost_unit).max(1))
            .ok()
            .and_then(|fuel| {
                fuel.checked_add(lift_off_fuel(&self.movement_rules, &origin, systems))
            })
            .ok_or(MovementError::Coordinates)?;
        let duration_ticks = if deep {
            distance_km.div_ceil(KM_PER_LIGHT_YEAR as u64).max(1)
        } else {
            distance_km
                .div_ceil(self.movement_rules.local_km_per_tick)
                .max(1)
        };
        let duration_ticks = self.pursuit_duration(fleet, duration_ticks);
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
            tick: self.tick,
        })
    }

    /// A pursuing fleet with the pursuit suite flies its courses in
    /// [`PURSUIT_COURSE_PERCENT`](crate::fleet::software::PURSUIT_COURSE_PERCENT)%
    /// of the time (at least one tick).
    fn pursuit_duration(&self, fleet: &crate::fleet::Fleet, ticks: u64) -> u64 {
        use crate::fleet::software::{FleetSoftware, PURSUIT_COURSE_PERCENT};
        let pursuing = self
            .navigation
            .pursuit
            .is_some_and(|pursuit| pursuit.fleet == fleet.id);
        if pursuing && fleet.software.contains(&FleetSoftware::PursuitSuite) {
            (ticks * PURSUIT_COURSE_PERCENT).div_ceil(100).max(1)
        } else {
            ticks
        }
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
                if *origin
                    == (Location::Landed {
                        system: *system,
                        body: *body,
                    })
                {
                    // Already down there: lift off instead.
                    return Err(MovementError::AlreadyThere);
                }
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
                self.local_destination(
                    system_by_id(systems, *system)?,
                    start,
                    destination.position,
                    location,
                    MovementMode::Local,
                )
            }
            MoveTarget::Local { system, position } => {
                if position.y_km != 0 {
                    return Err(MovementError::OffPlane);
                }
                let start = local_position(origin, *system, systems)?;
                let world = system_by_id(systems, *system)?;
                if geometry::local(*position)
                    .iter()
                    .all(|value| value.unsigned_abs() <= rules.system_half_extent_km as u64)
                {
                    let volumes = obstacles::obstacles(rules, world);
                    if let Some(volume) = obstacles::containing(*position, &volumes) {
                        return Err(if volume.star {
                            MovementError::StarCollision
                        } else {
                            MovementError::InsideBody
                        });
                    }
                    self.local_destination(
                        world,
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
                        world,
                        start,
                        outside,
                        Location::DeepSpace(absolute),
                        MovementMode::Exit,
                    )
                }
            }
            MoveTarget::DeepSpace(position) => {
                if position.y != 0 {
                    return Err(MovementError::OffPlane);
                }
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
                if super::routing::crosses_stellar_core(
                    *start,
                    end,
                    systems,
                    rules.star_exclusion_radius_km,
                )? {
                    return Err(MovementError::StarCollision);
                }
                Ok((
                    Location::DeepSpace(end),
                    MovementMode::DeepSpace,
                    geometry::distance(start.coordinates(), end.coordinates())?,
                ))
            }
            MoveTarget::LeaveSystem(system) => {
                let start = local_position(origin, *system, systems)?;
                let values = geometry::local(start);
                let largest = values
                    .iter()
                    .map(|value| value.unsigned_abs())
                    .max()
                    .ok_or(MovementError::Coordinates)?;
                if largest == 0 {
                    return Err(MovementError::StarCollision);
                }
                let half = i128::from(rules.system_half_extent_km) + 1;
                let position = geometry::position(
                    values.map(|value| (i128::from(value) * half / i128::from(largest)) as i64),
                );
                self.destination(
                    origin,
                    &MoveTarget::Local {
                        system: *system,
                        position,
                    },
                    systems,
                )
            }
            MoveTarget::EnterSystem(system) => {
                let Location::DeepSpace(start) = origin else {
                    return Err(MovementError::WrongSpace);
                };
                let center = system_center(systems, *system)?;
                let entry = geometry::entry(*start, center, rules.system_half_extent_km)?;
                let end = geometry::absolute(center, entry)?;
                if super::routing::crosses_stellar_core(
                    *start,
                    end,
                    systems,
                    rules.star_exclusion_radius_km,
                )? {
                    return Err(MovementError::StarCollision);
                }
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

    /// A straight local leg; it may not pass through the star or any body
    /// other than one it leaves from or arrives at.
    fn local_destination(
        &self,
        system: &StarSystem,
        start: LocalPosition,
        end: LocalPosition,
        destination: Location,
        mode: MovementMode,
    ) -> Result<(Location, MovementMode, u64), MovementError> {
        let volumes = obstacles::obstacles(&self.movement_rules, system);
        if let Some(volume) = obstacles::blocking(start, end, &volumes) {
            return Err(volume.error());
        }
        Ok((
            destination,
            mode,
            geometry::distance(geometry::local(start), geometry::local(end))?,
        ))
    }
}

/// Fuel per ship to lift off before a course: the landed body's landing fuel,
/// or 0 for a fleet that is not on a surface.
pub fn lift_off_fuel(
    rules: &super::MovementRules,
    origin: &Location,
    systems: &[StarSystem],
) -> u32 {
    let Location::Landed { system, body } = origin else {
        return 0;
    };
    systems
        .iter()
        .find(|known| known.id == *system)
        .and_then(|known| known.bodies.iter().find(|known| known.id == *body))
        .and_then(|body| rules.landing.fuel(body.kind))
        .unwrap_or(0)
}

fn system_by_id(systems: &[StarSystem], id: SystemId) -> Result<&StarSystem, MovementError> {
    systems
        .iter()
        .find(|system| system.id == id)
        .ok_or(MovementError::WrongSpace)
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
        }
        | Location::Landed {
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
