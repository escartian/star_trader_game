//! Point-and-select itineraries. A small visibility graph routes around the star
//! and every body's collision volume.
use super::{
    MoveTarget, MovementError, MovementPlan, SpacePosition, geometry, local_position, obstacles,
};
use crate::{
    BodyId, LocalPosition, StarSystem, SystemId,
    fleet::FleetMovementError,
    travel::{Location, PlayerId, ShipId, TravelError, TravelState},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoursePreview {
    pub targets: Vec<MoveTarget>,
    pub legs: Vec<MovementPlan>,
}

impl TravelState {
    pub fn plan_destination(
        &self,
        player: PlayerId,
        ship: ShipId,
        system: SystemId,
        body: Option<BodyId>,
        systems: &[StarSystem],
    ) -> Result<CoursePreview, FleetMovementError> {
        // Courses always use bodies where they stand at this state's own tick.
        let placed = crate::orbits::placed(systems, self.tick);
        let systems = placed.as_slice();
        self.authorize(player, ship)?;
        self.movement_rules.validate_world(systems)?;
        self.fleet_position(self.fleet_for_ship(ship)?.id, player)?;
        self.movement_rules
            .validate_location(&self.ship(ship)?.location, systems)?;
        let destination = systems
            .iter()
            .find(|known| known.id == system)
            .ok_or(TravelError::UnknownSystem)?;
        let mut preview = self.clone();
        let mut course = CoursePreview {
            targets: Vec::new(),
            legs: Vec::new(),
        };
        if let Some(origin) = self.ship(ship)?.location.system()
            && origin != system
        {
            let source = systems
                .iter()
                .find(|known| known.id == origin)
                .ok_or(TravelError::UnknownSystem)?;
            let direction = [
                i128::from(destination.position.x) - i128::from(source.position.x),
                i128::from(destination.position.y) - i128::from(source.position.y),
                i128::from(destination.position.z) - i128::from(source.position.z),
            ];
            let largest = direction
                .iter()
                .map(|value| value.abs())
                .max()
                .ok_or(MovementError::Coordinates)?;
            if largest == 0 {
                return Err(MovementError::WrongSpace.into());
            }
            let half = i128::from(self.movement_rules.system_half_extent_km) + 1;
            let exit = geometry::position(direction.map(|value| (value * half / largest) as i64));
            preview.append_local_route(
                (player, ship),
                origin,
                exit,
                MoveTarget::Local {
                    system: origin,
                    position: exit,
                },
                systems,
                &mut course,
            )?;
        }
        if matches!(preview.ship(ship)?.location, Location::DeepSpace(_)) {
            preview.append_leg(
                player,
                ship,
                MoveTarget::EnterSystem(system),
                systems,
                &mut course,
            )?;
        }
        if let Some(body) = body {
            let target = destination
                .bodies
                .iter()
                .find(|known| known.id == body)
                .ok_or(MovementError::UnknownBody)?;
            preview.append_local_route(
                (player, ship),
                system,
                target.position,
                MoveTarget::Body { system, body },
                systems,
                &mut course,
            )?;
        }
        if course.legs.is_empty() {
            return Err(MovementError::AlreadyThere.into());
        }
        Ok(course)
    }

    fn append_local_route(
        &mut self,
        actor: (PlayerId, ShipId),
        system: SystemId,
        end: LocalPosition,
        final_target: MoveTarget,
        systems: &[StarSystem],
        course: &mut CoursePreview,
    ) -> Result<(), FleetMovementError> {
        let (player, ship) = actor;
        let start = local_position(&self.ship(ship)?.location, system, systems)?;
        let world = systems
            .iter()
            .find(|known| known.id == system)
            .ok_or(TravelError::UnknownSystem)?;
        let volumes = obstacles::obstacles(&self.movement_rules, world);
        let path = safe_local_path(
            start,
            end,
            &volumes,
            self.movement_rules.system_half_extent_km,
        )?;
        for waypoint in path {
            self.append_leg(
                player,
                ship,
                MoveTarget::Local {
                    system,
                    position: waypoint,
                },
                systems,
                course,
            )?;
        }
        self.append_leg(player, ship, final_target, systems, course)
    }

    fn append_leg(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        target: MoveTarget,
        systems: &[StarSystem],
        course: &mut CoursePreview,
    ) -> Result<(), FleetMovementError> {
        let plan = self.plan_movement(player, ship, &target, systems)?;
        let members = self.fleet_for_ship(ship)?.ships.clone();
        for id in members {
            let member = self.ship_mut(id)?;
            member.fuel -= plan.fuel_per_ship;
            member.location = plan.destination.clone();
        }
        course.targets.push(target);
        course.legs.push(plan);
        Ok(())
    }
}

/// Waypoints from `start` to `end` inside `system` that avoid the star and every
/// body as they stand now (empty when the straight line is clear).
pub(crate) fn safe_route(
    rules: &super::MovementRules,
    system: &StarSystem,
    start: LocalPosition,
    end: LocalPosition,
) -> Result<Vec<LocalPosition>, MovementError> {
    let volumes = obstacles::obstacles(rules, system);
    safe_local_path(start, end, &volumes, rules.system_half_extent_km)
}

/// Waypoints for a local course that avoids every collision volume: empty when
/// the straight line is clear. A small visibility graph over points around each
/// obstacle (the corners of a cube twice its radius, clamped to the system cube)
/// finds the shortest clear chain by Dijkstra.
fn safe_local_path(
    start: LocalPosition,
    end: LocalPosition,
    volumes: &[obstacles::Obstacle],
    half: i64,
) -> Result<Vec<LocalPosition>, MovementError> {
    let Some(first) = obstacles::blocking(start, end, volumes) else {
        return Ok(Vec::new());
    };
    let mut nodes = vec![start, end];
    for volume in volumes {
        let clearance = (volume.radius * 2).min(half);
        let centre = geometry::local(volume.centre);
        for corner in 0..8 {
            let node = geometry::position(std::array::from_fn(|axis| {
                let offset = if corner & (1 << axis) == 0 {
                    -clearance
                } else {
                    clearance
                };
                (centre[axis] + offset).clamp(-half, half)
            }));
            if obstacles::containing(node, volumes).is_none() {
                nodes.push(node);
            }
        }
    }
    let mut costs = vec![u64::MAX; nodes.len()];
    let mut previous = vec![None; nodes.len()];
    let mut visited = vec![false; nodes.len()];
    costs[0] = 0;
    for _ in 0..nodes.len() {
        let Some(current) = (0..nodes.len())
            .filter(|index| !visited[*index])
            .min_by_key(|index| costs[*index])
        else {
            break;
        };
        if costs[current] == u64::MAX || current == 1 {
            break;
        }
        visited[current] = true;
        for next in 0..nodes.len() {
            if next == current
                || visited[next]
                || obstacles::blocking(nodes[current], nodes[next], volumes).is_some()
            {
                continue;
            }
            let distance = geometry::distance(
                geometry::local(nodes[current]),
                geometry::local(nodes[next]),
            )?;
            let cost = costs[current]
                .checked_add(distance)
                .ok_or(MovementError::Coordinates)?;
            if cost < costs[next] {
                costs[next] = cost;
                previous[next] = Some(current);
            }
        }
    }
    if costs[1] == u64::MAX {
        return Err(first.error());
    }
    let mut path = Vec::new();
    let mut index = previous[1].ok_or(first.error())?;
    while index != 0 {
        path.push(nodes[index]);
        index = previous[index].ok_or(first.error())?;
    }
    path.reverse();
    Ok(path)
}

/// Conservative segment/cube rejection for deep-space stellar cores. Rational
/// clipping avoids floating-point precision loss and unbounded squared products.
pub(super) fn crosses_stellar_core(
    start: SpacePosition,
    end: SpacePosition,
    systems: &[StarSystem],
    radius: i64,
) -> Result<bool, MovementError> {
    for system in systems {
        let center = SpacePosition::from_galaxy(system.position)?.coordinates();
        let (mut enter, mut leave) = ((0_i128, 1_i128), (1_i128, 1_i128));
        let mut intersects = true;
        for (axis, center_value) in center.into_iter().enumerate() {
            let a = i128::from(start.coordinates()[axis]) - i128::from(center_value);
            let d = i128::from(end.coordinates()[axis]) - i128::from(start.coordinates()[axis]);
            if d == 0 {
                if a.abs() <= i128::from(radius) {
                    continue;
                }
                intersects = false;
                break;
            }
            let (low, high, denominator) = if d > 0 {
                (-i128::from(radius) - a, i128::from(radius) - a, d)
            } else {
                (a - i128::from(radius), a + i128::from(radius), -d)
            };
            if low * enter.1 > enter.0 * denominator {
                enter = (low, denominator);
            }
            if high * leave.1 < leave.0 * denominator {
                leave = (high, denominator);
            }
            if enter.0 * leave.1 > leave.0 * enter.1 {
                intersects = false;
                break;
            }
        }
        if intersects {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests;
