use super::{NavigationError, NavigationOrder};
use crate::{
    StarSystem,
    fleet::{FleetError, FleetId, FleetMovementError},
    movement::MoveTarget,
    travel::{Location, PlayerId, ShipId, TravelError, TravelState},
};

impl TravelState {
    pub fn navigation_attention(&self, player: PlayerId) -> Option<FleetId> {
        if let Some(encounter) = self.navigation.encounters.iter().find(|encounter| {
            self.fleets
                .iter()
                .any(|fleet| fleet.id == encounter.fleet && fleet.owner == player)
        }) {
            return Some(encounter.fleet);
        }
        self.fleets
            .iter()
            .filter(|fleet| fleet.owner == player)
            .find(|fleet| {
                self.navigation.order(fleet.id).is_none()
                    && !self.navigation.holds.contains(&fleet.id)
                    && !fleet.ships.iter().any(|id| {
                        self.ship(*id)
                            .is_ok_and(|ship| matches!(ship.location, Location::Travelling(_)))
                    })
            })
            .map(|fleet| fleet.id)
    }

    pub fn plot_course(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        target: MoveTarget,
        systems: &[StarSystem],
    ) -> Result<FleetId, FleetMovementError> {
        self.authorize(player, ship)?;
        let id = self.fleet_for_ship(ship)?.id;
        if self.navigation.order(id).is_some_and(|order| order.started) {
            return Err(NavigationError::Underway.into());
        }
        let plan = self.plan_movement(player, ship, &target, systems)?;
        self.store_course(target, plan)
    }

    pub fn plot_destination(
        &mut self,
        player: PlayerId,
        ship: ShipId,
        system: crate::SystemId,
        body: Option<crate::BodyId>,
        systems: &[StarSystem],
    ) -> Result<FleetId, FleetMovementError> {
        let preview = self.plan_destination(player, ship, system, body, systems)?;
        let first = preview
            .legs
            .first()
            .ok_or(NavigationError::NoCourse)?
            .clone();
        let mut targets = preview.targets.into_iter();
        let target = targets.next().ok_or(NavigationError::NoCourse)?;
        let fleet = self.store_course(target, first)?;
        if let Some(order) = self
            .navigation
            .orders
            .iter_mut()
            .find(|order| order.plan.fleet == fleet)
        {
            order.remaining_waypoints = targets.collect();
        }
        Ok(fleet)
    }

    pub fn store_course(
        &mut self,
        target: MoveTarget,
        plan: crate::movement::MovementPlan,
    ) -> Result<FleetId, FleetMovementError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        if self
            .navigation
            .order(plan.fleet)
            .is_some_and(|order| order.started)
        {
            return Err(NavigationError::Underway.into());
        }
        let fleet = plan.fleet;
        self.navigation
            .orders
            .retain(|order| order.plan.fleet != fleet);
        self.navigation.holds.retain(|id| *id != fleet);
        self.navigation.orders.push(NavigationOrder {
            remaining_waypoints: Vec::new(),
            remaining_ticks: plan.duration_ticks,
            plan,
            target,
            started: false,
        });
        self.revision = revision;
        Ok(fleet)
    }

    pub fn hold_fleet(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
    ) -> Result<(), FleetMovementError> {
        self.fleet_position(fleet, player)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        if self
            .navigation
            .order(fleet)
            .is_some_and(|order| order.started)
        {
            return Err(NavigationError::Underway.into());
        }
        self.navigation
            .orders
            .retain(|order| order.plan.fleet != fleet);
        if !self.navigation.holds.contains(&fleet) {
            self.navigation.holds.push(fleet);
        }
        self.revision = revision;
        Ok(())
    }

    /// Drops Wait orders and trade routes whose fleet no longer exists (sold, merged
    /// away, destroyed). Surviving fleets keep theirs: only the player replaces them.
    pub fn prune_standing_orders(&mut self) {
        let fleets = &self.fleets;
        let exists = |id: FleetId| fleets.iter().any(|fleet| fleet.id == id);
        self.navigation.holds.retain(|id| exists(*id));
        self.navigation.routes.retain(|route| exists(route.fleet));
    }

    pub fn cancel_course(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
    ) -> Result<(), FleetMovementError> {
        self.fleet_position(fleet, player)?;
        let order = self
            .navigation
            .order(fleet)
            .ok_or(NavigationError::NoCourse)?;
        if order.started {
            return Err(NavigationError::Underway.into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        self.navigation
            .orders
            .retain(|order| order.plan.fleet != fleet);
        self.revision = revision;
        Ok(())
    }

    pub fn guard_fleet_changes(
        &self,
        player: PlayerId,
        fleet: FleetId,
    ) -> Result<(), FleetMovementError> {
        let record = self
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?;
        if record.owner != player {
            return Err(FleetError::NotOwner.into());
        }
        if self.navigation.order(fleet).is_some() {
            return Err(NavigationError::FleetHasCourse.into());
        }
        Ok(())
    }
}
