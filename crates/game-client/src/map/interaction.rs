//! Selection and view changes are presentation; plotted routes move only on runs.
use super::{Map, local::LocalView};

impl Map {
    pub(super) fn follow_navigation_transitions(&mut self) {
        let run = self.session.run_number();
        if run == self.observed_run {
            self.observed_system = self.session.current_system();
            self.observed_location = self.session.selected_ship().location.clone();
            return;
        }
        self.observed_run = run;
        let current = self.session.current_system();
        let previous = self.observed_system;
        self.observed_system = current;
        let moved = self.observed_location != self.session.selected_ship().location;
        self.observed_location = self.session.selected_ship().location.clone();
        if current == previous {
            if moved && let Some(view) = &mut self.local {
                view.focus_ship(self.session.selected_ship());
            }
            return;
        }
        if let Some(system) =
            current.and_then(|id| self.session_systems.iter().find(|system| system.id == id))
        {
            if self
                .local
                .as_ref()
                .is_none_or(|view| view.system().id != system.id)
            {
                let mut view = LocalView::new(system.clone());
                view.focus_ship(self.session.selected_ship());
                self.local = Some(view);
                self.selected = Some(system.id);
            } else if moved && let Some(view) = &mut self.local {
                view.focus_ship(self.session.selected_ship());
            }
        } else if self.local.is_some() {
            // Local movement remains in its system while underway. Entry/deep-space
            // travel uses the outer map until the fleet actually arrives.
            if let Some(system) = self.session.local_course_system()
                && self
                    .local
                    .as_ref()
                    .is_some_and(|view| view.system().id == system)
            {
                return;
            }
            self.local = None;
            if let Some(position) = self.session.galaxy_position(&self.session_systems) {
                self.camera.focus_on(position);
            }
        }
    }
}
