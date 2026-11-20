//! Planning the next leg of a course once a leg ends. Bodies move along their
//! orbits, so a detour planned earlier can now cut through a body (or end
//! inside one). Then the stale detour waypoints are dropped and the rest of the
//! course is re-routed from where the fleet is now; if there is no clear way,
//! the course ends and the fleet waits for new orders.
use super::NavigationOrder;
use crate::{
    LocalPosition, StarSystem, SystemId,
    fleet::FleetMovementError,
    movement::{MoveTarget, MovementError, local_position, safe_route},
    travel::{PlayerId, ShipId, TravelState},
};

impl TravelState {
    /// The order for `target` (then `remaining`), re-routed around bodies that
    /// moved into the way; `None` when the course cannot continue.
    pub(super) fn next_leg(
        &self,
        player: PlayerId,
        ship: ShipId,
        target: MoveTarget,
        remaining: Vec<MoveTarget>,
        systems: &[StarSystem],
    ) -> Result<Option<NavigationOrder>, FleetMovementError> {
        match self.plan_movement(player, ship, &target, systems) {
            Ok(plan) => return Ok(Some(order(target, plan, remaining))),
            Err(error) if !blocked(&error) => return Err(error),
            Err(_) => {}
        }
        let mut course: Vec<MoveTarget> = std::iter::once(target).chain(remaining).collect();
        let half = self.movement_rules.system_half_extent_km;
        let Some(goal) = course.iter().position(|target| !detour(target, half)) else {
            return Ok(None);
        };
        let rest = course.split_off(goal);
        let Some((system, end)) = local_goal(&rest[0], systems) else {
            return Ok(None);
        };
        let Some(world) = systems.iter().find(|known| known.id == system) else {
            return Ok(None);
        };
        let here = &self.ship(ship)?.location;
        let Ok(start) = local_position(here, system, systems) else {
            return Ok(None);
        };
        let Ok(path) = safe_route(&self.movement_rules, world, start, end) else {
            return Ok(None);
        };
        let mut targets: Vec<MoveTarget> = path
            .into_iter()
            .map(|position| MoveTarget::Local { system, position })
            .chain(rest)
            .collect();
        let first = targets.remove(0);
        match self.plan_movement(player, ship, &first, systems) {
            Ok(plan) => Ok(Some(order(first, plan, targets))),
            Err(error) if blocked(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn order(
    target: MoveTarget,
    plan: crate::movement::MovementPlan,
    remaining_waypoints: Vec<MoveTarget>,
) -> NavigationOrder {
    NavigationOrder {
        target,
        remaining_ticks: plan.duration_ticks,
        plan,
        started: false,
        remaining_waypoints,
    }
}

fn blocked(error: &FleetMovementError) -> bool {
    matches!(
        error,
        FleetMovementError::Movement(
            MovementError::BodyCollision | MovementError::InsideBody | MovementError::StarCollision
        )
    )
}

/// A waypoint inside the system cube: part of a detour, safe to recompute.
fn detour(target: &MoveTarget, half: i64) -> bool {
    matches!(target, MoveTarget::Local { position, .. }
        if [position.x_km, position.y_km, position.z_km]
            .iter()
            .all(|value| value.unsigned_abs() <= half as u64))
}

/// Where in which system a course section ends: a body as it stands now, or a
/// point beyond the cube (a system exit).
fn local_goal(target: &MoveTarget, systems: &[StarSystem]) -> Option<(SystemId, LocalPosition)> {
    match target {
        MoveTarget::Body { system, body } => systems
            .iter()
            .find(|known| known.id == *system)?
            .bodies
            .iter()
            .find(|known| known.id == *body)
            .map(|body| (*system, body.position)),
        MoveTarget::Local { system, position } => Some((*system, *position)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyId, travel::Location};

    fn at(x: i64, y: i64, z: i64) -> LocalPosition {
        LocalPosition {
            x_km: x,
            y_km: y,
            z_km: z,
        }
    }

    /// The starting freighter at `position` in Sol, at tick 0 (authored layout).
    fn ship_at(position: LocalPosition) -> TravelState {
        let mut state = TravelState::default();
        state.ships[0].location = Location::InSystem {
            system: SystemId(1),
            position,
        };
        state
    }

    fn local(position: LocalPosition) -> MoveTarget {
        MoveTarget::Local {
            system: SystemId(1),
            position,
        }
    }

    #[test]
    fn a_stale_detour_through_a_body_is_re_routed_to_the_same_destination() {
        let systems = crate::demo_galaxy();
        // A detour waypoint whose leg now cuts straight through the Inner world.
        let state = ship_at(at(40_000_000, 5_500_000, -20_000_000));
        let stale = local(at(50_000_000, 10_500_000, -20_000_000));
        let moon = MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(4),
        };
        let order = state
            .next_leg(
                PlayerId(1),
                ShipId(1),
                stale.clone(),
                vec![moon.clone()],
                &systems,
            )
            .unwrap()
            .expect("a clear way around exists");
        assert_ne!(order.target, stale, "the blocked waypoint was dropped");
        let last = order.remaining_waypoints.last().unwrap_or(&order.target);
        assert_eq!(*last, moon, "the course still ends at the Moon");
    }

    #[test]
    fn a_course_with_nowhere_clear_to_go_ends() {
        let systems = crate::demo_galaxy();
        let state = ship_at(at(40_000_000, 5_500_000, -20_000_000));
        let inside = local(at(45_100_000, 8_000_000, -20_000_000));
        let next = state
            .next_leg(PlayerId(1), ShipId(1), inside, Vec::new(), &systems)
            .unwrap();
        assert!(next.is_none());
    }
}
