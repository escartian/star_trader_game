//! Movement inputs and confirmation. All costs and destinations come from the host.
use super::LocalSession;
use game_core::{
    GalaxyPosition, LocalPosition, StarSystem, SystemId,
    movement::{KM_PER_LIGHT_YEAR, MoveTarget},
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage};

#[derive(Default)]
pub(super) struct NavigationPanel {
    pub open: bool,
    pub(super) target: Option<MoveTarget>,
    local: [f64; 3],
    galaxy: [i64; 3],
}

pub fn location_label(location: &Location, systems: &[StarSystem]) -> String {
    let name = |id| {
        systems
            .iter()
            .find(|system| system.id == id)
            .map_or("Unknown", |system| system.name.as_str())
    };
    match location {
        Location::Docked(system) => format!("Docked at {} trade port", name(*system)),
        Location::Orbiting { system, body } => {
            let body = systems
                .iter()
                .find(|known| known.id == *system)
                .and_then(|known| known.bodies.iter().find(|known| known.id == *body))
                .map_or("Unknown body", |body| body.name.as_str());
            format!("Near {body}")
        }
        Location::InSystem { system, position } => format!(
            "{}: ({:.4}, {:.4}, {:.4}) AU",
            name(*system),
            crate::units::kilometers_to_au(position.x_km as f64),
            crate::units::kilometers_to_au(position.y_km as f64),
            crate::units::kilometers_to_au(position.z_km as f64)
        ),
        Location::DeepSpace(position) => format!(
            "Deep space: ({:.5}, {:.5}, {:.5}) ly",
            position.x_km as f64 / KM_PER_LIGHT_YEAR as f64,
            position.y_km as f64 / KM_PER_LIGHT_YEAR as f64,
            position.z_km as f64 / KM_PER_LIGHT_YEAR as f64
        ),
        Location::InTransit(fleet) => format!("Fleet {} is underway", fleet.0),
        Location::Travelling(journey) => format!("Saved journey to {}", name(journey.destination)),
    }
}

impl LocalSession {
    pub fn open_navigation(&mut self) {
        self.navigation.open = true;
    }

    pub fn request_movement(&mut self, target: MoveTarget) {
        self.navigation.target = Some(target);
        self.navigation.open = true;
    }

    pub fn show_navigation(&mut self, ctx: &egui::Context, systems: &[StarSystem]) {
        let mut open = self.navigation.open;
        egui::Window::new("Fleet navigation")
            .open(&mut open)
            .default_width(600.0)
            .vscroll(true)
            .show(ctx, |ui| self.navigation_ui(ui, systems));
        self.navigation.open = open;
    }

    fn navigation_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        self.run_review(ui, systems);
        let ship = self.selected_ship().clone();
        ui.label(format!(
            "{} • {}",
            ship.name,
            location_label(&ship.location, systems)
        ));
        ui.label(
            "Plot courses for your fleets, then Execute navigation run when every fleet is ready.",
        );
        let rules = self.state.movement_rules.clone();
        if let Some(system) = ship
            .location
            .system()
            .filter(|_| ship.location.stationary())
        {
            ui.label(format!(
                "Local cube: ±{:.4} AU on each axis",
                crate::units::kilometers_to_au(rules.system_half_extent_km as f64)
            ));
            ui.label("Local waypoint (AU); an outside target exits at the first boundary:");
            ui.horizontal_wrapped(|ui| {
                for (axis, value) in ["X", "Y", "Z"].into_iter().zip(&mut self.navigation.local) {
                    ui.label(axis);
                    ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(6));
                }
                if ui.button("Preview waypoint").clicked() {
                    let [x, y, z] = self.navigation.local.map(crate::units::au_to_kilometers);
                    if let (Some(x_km), Some(y_km), Some(z_km)) = (x, y, z) {
                        self.navigation.target = Some(MoveTarget::Local {
                            system,
                            position: LocalPosition { x_km, y_km, z_km },
                        });
                    } else {
                        self.navigation.target = None;
                        self.record("Waypoint exceeds the supported coordinate range.".into());
                    }
                }
            });
            if let Ok(current) =
                game_core::movement::local_position(&ship.location, system, systems)
            {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Exit toward:");
                    for (axis, label) in ["X", "Y", "Z"].into_iter().enumerate() {
                        for sign in [-1, 1] {
                            if ui
                                .button(format!("{}{label}", if sign < 0 { "−" } else { "+" }))
                                .clicked()
                            {
                                let mut position = [current.x_km, current.y_km, current.z_km];
                                position[axis] = sign * (rules.system_half_extent_km + 1);
                                self.navigation.target = Some(MoveTarget::Local {
                                    system,
                                    position: LocalPosition {
                                        x_km: position[0],
                                        y_km: position[1],
                                        z_km: position[2],
                                    },
                                });
                            }
                        }
                    }
                });
            }
            ui.label(
                "Choose a planet, moon, asteroid or trade port in the system view to move there.",
            );
        } else if matches!(ship.location, Location::DeepSpace(_)) {
            ui.label(format!(
                "Galaxy cube: ±{} ly • waypoint coordinates (ly)",
                rules.galaxy_half_extent_ly
            ));
            ui.horizontal_wrapped(|ui| {
                for (axis, value) in ["X", "Y", "Z"].into_iter().zip(&mut self.navigation.galaxy) {
                    ui.label(axis);
                    ui.add(
                        egui::DragValue::new(value)
                            .range(-rules.galaxy_half_extent_ly..=rules.galaxy_half_extent_ly),
                    );
                }
                if ui.button("Preview deep-space move").clicked() {
                    self.navigation.target = Some(MoveTarget::DeepSpace(GalaxyPosition {
                        x: self.navigation.galaxy[0],
                        y: self.navigation.galaxy[1],
                        z: self.navigation.galaxy[2],
                    }));
                }
            });
            ui.label("A system targeted by name or exact coordinates enters at its near cube face. Passing it along a path does not enter it.");
            ui.horizontal_wrapped(|ui| {
                for system in systems {
                    if ui.button(format!("Enter {}", system.name)).clicked() {
                        self.navigation.target = Some(MoveTarget::EnterSystem(system.id));
                    }
                }
            });
        }
        self.movement_preview(ui, systems);
    }

    fn movement_preview(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let Some(target) = self.navigation.target.clone() else {
            return;
        };
        ui.separator();
        match self.authority.handle(ClientCommand::PreviewMove {
            ship: self.active_ship,
            target: target.clone(),
        }) {
            ServerMessage::MovementPreview(plan) => {
                ui.strong(plan.mode.label());
                ui.label(location_label(&plan.destination, systems));
                let distance = if matches!(
                    plan.mode,
                    game_core::movement::MovementMode::Entry
                        | game_core::movement::MovementMode::DeepSpace
                ) {
                    format!(
                        "{:.3} ly",
                        plan.distance_km as f64 / KM_PER_LIGHT_YEAR as f64
                    )
                } else {
                    format!(
                        "{:.4} AU",
                        crate::units::kilometers_to_au(plan.distance_km as f64)
                    )
                };
                ui.label(format!(
                    "{distance} • {} fuel per ship at departure • at least {} navigation runs",
                    plan.fuel_per_ship,
                    plan.duration_ticks
                        .div_ceil(self.state.movement_rules.ticks_per_navigation_run)
                ));
                if ui.button("Plot course").clicked() {
                    self.command(ClientCommand::Move {
                        ship: self.active_ship,
                        target,
                        expected_revision: plan.revision,
                    });
                    self.navigation.target = None;
                    ui.ctx().request_repaint();
                }
            }
            ServerMessage::CommandRejected(error) => {
                ui.label(error.to_string());
            }
            _ => {}
        }
        ui.label(&self.notice);
    }

    pub fn current_system(&self) -> Option<SystemId> {
        self.selected_ship().location.system()
    }
}

impl LocalSession {
    /// Capture fixtures execute ordinary commands in a fresh, unsaved session.
    pub fn capture_movement(&mut self, target: MoveTarget) -> Result<(), String> {
        let response = self.authority.handle_offline(ClientCommand::Move {
            ship: self.active_ship,
            target,
            expected_revision: self.state.revision,
        });
        if let ServerMessage::CommandRejected(error) = &response {
            return Err(error.to_string());
        }
        self.accept(response);
        for _ in 0..128 {
            if self.state.navigation.orders.is_empty() {
                return Ok(());
            }
            let fleets = self.state.fleets.clone();
            for fleet in fleets {
                if self.state.navigation.order(fleet.id).is_none() {
                    self.command(ClientCommand::HoldFleet {
                        fleet: fleet.id,
                        expected_revision: self.state.revision,
                    });
                }
            }
            let encounters = self.state.navigation.encounters.clone();
            for encounter in encounters {
                self.command(ClientCommand::ResolveEncounter {
                    encounter: encounter.id,
                    choice: game_core::navigation::EncounterChoice::Bypass,
                    expected_revision: self.state.revision,
                });
            }
            self.command(ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision,
            });
        }
        Err("Capture course did not finish within 128 runs".into())
    }
}

#[cfg(test)]
mod tests;
