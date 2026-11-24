//! Clearing the flagship's plotted course: tapping the place it already rests
//! at cancels a course that has not set off (a hull purchase, a landing or a
//! fleet change needs none), and the destination card and shipyard offer a
//! Cancel course button. The authority still checks every cancel.
use super::{LocalSession, navigation::location_label};
use crate::theme::{
    AMBER,
    icons::{self, Icon},
};
use game_core::{BodyId, BodyKind, StarSystem, SystemId, travel::Location};
use game_protocol::ClientCommand;

impl LocalSession {
    /// Whether the selected ship rests at `system` (and `body`, when given).
    pub(super) fn rests_at(
        &self,
        system: SystemId,
        body: Option<BodyId>,
        systems: &[StarSystem],
    ) -> bool {
        let location = &self.selected_ship().location;
        if !location.stationary() || location.system() != Some(system) {
            return false;
        }
        let Some(body) = body else {
            return true;
        };
        match location {
            Location::Orbiting { body: at, .. } | Location::Landed { body: at, .. } => *at == body,
            Location::Docked(_) => systems
                .iter()
                .find(|known| known.id == system)
                .and_then(|known| known.bodies.iter().find(|known| known.id == body))
                .is_some_and(|known| known.kind == BodyKind::Station),
            _ => false,
        }
    }

    /// The selected fleet's course that has not set off yet, if any.
    fn pending_course(&self) -> Option<(game_core::fleet::FleetId, Location)> {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok()?;
        let order = self.state.navigation.order(fleet.id)?;
        (!order.started).then(|| (fleet.id, order.plan.destination.clone()))
    }

    /// Cancels the selected fleet's course if it has not set off; returns
    /// whether there was one.
    pub(super) fn cancel_pending_course(&mut self) -> bool {
        let Some((fleet, _)) = self.pending_course() else {
            return false;
        };
        self.command(ClientCommand::CancelCourse {
            fleet,
            expected_revision: self.state.revision,
        });
        true
    }

    /// A cancelled course (or a Wait) for the selected fleet drops its
    /// destination, so no route line lingers on the map.
    pub(super) fn forget_cancelled_target(&mut self, event: &game_protocol::SessionEvent) {
        use game_core::navigation::NavigationEvent;
        let game_protocol::SessionEvent::Navigation(
            NavigationEvent::CourseCancelled(fleet) | NavigationEvent::Holding(fleet),
        ) = event
        else {
            return;
        };
        if self
            .state
            .fleet_for_ship(self.active_ship)
            .is_ok_and(|selected| selected.id == *fleet)
        {
            self.map_target = None;
        }
    }

    /// Where the selected fleet's course ends, by star system.
    pub fn course_system(&self) -> Option<SystemId> {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok()?;
        let order = self.state.navigation.order(fleet.id)?;
        let last = order.remaining_waypoints.last().unwrap_or(&order.target);
        match last {
            game_core::movement::MoveTarget::Body { system, .. }
            | game_core::movement::MoveTarget::Local { system, .. }
            | game_core::movement::MoveTarget::EnterSystem(system)
            | game_core::movement::MoveTarget::LeaveSystem(system) => Some(*system),
            game_core::movement::MoveTarget::DeepSpace(_) => None,
        }
    }

    /// "Course plotted to X · Cancel course", while one waits to set off.
    pub(super) fn pending_course_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let Some((_, destination)) = self.pending_course() else {
            return;
        };
        let mut cancel = false;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Plot, AMBER, "Plotted course");
            ui.small(format!(
                "Course plotted to {}; it sets off on the next run.",
                location_label(&destination, systems)
            ));
            cancel = icons::button(ui, Icon::Cancel, "Cancel course")
                .on_hover_text("Stay here (tapping the fleet's own spot does the same)")
                .clicked();
        });
        if cancel {
            self.cancel_pending_course();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LocalSession;
    use game_core::{SystemId, fleet::ShipKind};
    use game_protocol::ClientCommand;

    #[test]
    fn tapping_the_fleets_own_spot_drops_its_course_so_hulls_can_be_bought() {
        let mut host = LocalSession::new(game_server::Session::default());
        host.economy.credits = game_core::economy::Credits(1_000_000);
        let here = host.state.ships[0].location.clone();
        let (system, body) = match here {
            game_core::travel::Location::Orbiting { system, body } => (system, Some(body)),
            game_core::travel::Location::Docked(system) => (system, None),
            other => panic!("unexpected start {other:?}"),
        };
        host.select_destination(SystemId(2), None);
        assert_eq!(host.state.navigation.orders.len(), 1);
        let buy = |host: &mut LocalSession| {
            let ships = host.state.ships.len();
            host.command(ClientCommand::BuyShip {
                ship: host.active_ship,
                kind: ShipKind::Scout,
                expected_revision: host.state.revision,
            });
            host.state.ships.len() > ships
        };
        assert!(!buy(&mut host), "a plotted course blocks fleet changes");
        host.select_destination(system, body);
        assert!(host.state.navigation.orders.is_empty(), "{}", host.notice);
        assert_eq!(host.state.ships[0].location, here, "nothing moved");
        assert!(buy(&mut host), "{}", host.notice);
        // With no course, the own spot is simply selected: no error.
        host.select_destination(system, body);
        assert!(host.state.navigation.orders.is_empty());
    }

    #[test]
    fn cancelling_a_course_clears_its_line() {
        let mut host = LocalSession::new(game_server::Session::default());
        let systems = host.authority.systems().to_vec();
        let here = host.state.ships[0].location.system().unwrap();
        let body = systems
            .iter()
            .find(|known| known.id == here)
            .unwrap()
            .bodies
            .iter()
            .map(|body| body.id)
            .find(|body| !host.rests_at(here, Some(*body), &systems))
            .unwrap();
        host.select_destination(here, Some(body));
        assert!(
            !host.map_route_points(here, &systems).is_empty(),
            "a line to it"
        );
        assert_eq!(host.course_system(), Some(here));
        assert!(host.cancel_pending_course());
        assert!(
            host.map_route_points(here, &systems).is_empty(),
            "no line left"
        );
        assert_eq!(host.course_system(), None);
    }
}
