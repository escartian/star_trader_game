//! Movement inputs and confirmation. All costs and destinations come from the host.
use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{CYAN, GREEN};
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
    local: [f64; 2],
    galaxy: [i64; 2],
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
            format!("In orbit of {body}")
        }
        Location::Landed { system, body } => {
            format!("Landed on {}", body_name(systems, *system, *body))
        }
        Location::InSystem { system, position } => format!(
            "{}: ({:.4}, {:.4}) AU (X, Z)",
            name(*system),
            crate::units::kilometers_to_au(position.x_km as f64),
            crate::units::kilometers_to_au(position.z_km as f64)
        ),
        Location::DeepSpace(position) => format!(
            "Deep space: ({:.5}, {:.5}) ly (X, Z)",
            position.x_km as f64 / KM_PER_LIGHT_YEAR as f64,
            position.z_km as f64 / KM_PER_LIGHT_YEAR as f64
        ),
        Location::InTransit(fleet) => format!("Fleet {} is underway", fleet.0),
        Location::Travelling(journey) => format!("Saved journey to {}", name(journey.destination)),
    }
}

fn body_name(systems: &[StarSystem], system: game_core::SystemId, body: game_core::BodyId) -> &str {
    systems
        .iter()
        .find(|known| known.id == system)
        .and_then(|known| known.bodies.iter().find(|known| known.id == body))
        .map_or("Unknown body", |body| body.name.as_str())
}

/// Compact location: an icon for the kind of place plus its short name.
pub fn location_chip(location: &Location, systems: &[StarSystem]) -> (Icon, String) {
    let name = |id| {
        systems
            .iter()
            .find(|system| system.id == id)
            .map_or_else(|| "Unknown".to_string(), |system| system.name.clone())
    };
    match location {
        Location::Docked(system) => (Icon::Dock, format!("Docked · {} port", name(*system))),
        Location::Orbiting { system, body } => (
            Icon::Orbit,
            format!(
                "Orbit · {}",
                systems
                    .iter()
                    .find(|known| known.id == *system)
                    .and_then(|known| known.bodies.iter().find(|known| known.id == *body))
                    .map_or("Unknown body", |body| body.name.as_str())
            ),
        ),
        Location::Landed { system, body } => (
            Icon::Dock,
            format!("Landed · {}", body_name(systems, *system, *body)),
        ),
        Location::InSystem { system, .. } => (Icon::Pin, name(*system)),
        Location::DeepSpace(_) => (Icon::DeepSpace, "Deep space".into()),
        Location::InTransit(_) => (Icon::Underway, "Underway".into()),
        Location::Travelling(journey) => (Icon::Underway, name(journey.destination)),
    }
}

/// Location chip with the full description as hover text.
pub fn location_stat(ui: &mut egui::Ui, location: &Location, systems: &[StarSystem]) {
    let (icon, short) = location_chip(location, systems);
    // Green = clamped to the trade port; cyan = holding orbit or flying.
    let color = if matches!(location, Location::Docked(_)) {
        crate::theme::GREEN
    } else {
        crate::theme::CYAN
    };
    stat(ui, icon, color, &short, &location_label(location, systems));
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
        crate::theme::metal::window(ctx, "Navigation", &mut open, 600.0, |ui| {
            self.navigation_ui(ui, systems)
        });
        self.navigation.open = open;
    }

    fn navigation_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        self.run_review(ui, systems);
        let ship = self.selected_ship().clone();
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Ship, GREEN, ship.kind.label());
            ui.strong(&ship.name);
            location_stat(ui, &ship.location, systems);
            icons::glyph(
                ui,
                Icon::Help,
                CYAN,
                "Select a fleet, then tap a destination on the map to plot its course.\nRoutes handle local travel, departure and entry.\nPress Advance once every fleet has a course or a Wait order.",
            );
        });
        let rules = self.state.movement_rules.clone();
        egui::CollapsingHeader::new("Advanced coordinates").show(ui, |ui| {
        if let Some(system) = ship
            .location
            .system()
            .filter(|_| ship.location.stationary())
        {
            ui.label(format!(
                "Local map: ±{:.4} AU on X and Z",
                crate::units::kilometers_to_au(rules.system_half_extent_km as f64)
            ));
            ui.label("Local waypoint (AU); an outside target exits at the first boundary:");
            ui.horizontal_wrapped(|ui| {
                for (axis, value) in ["X", "Z"].into_iter().zip(&mut self.navigation.local) {
                    ui.label(axis);
                    ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(6));
                }
                if ui.button("Preview waypoint").clicked() {
                    let [x, z] = self.navigation.local.map(crate::units::au_to_kilometers);
                    if let (Some(x_km), Some(z_km)) = (x, z) {
                        self.navigation.target = Some(MoveTarget::Local {
                            system,
                            position: LocalPosition { x_km, y_km: 0, z_km },
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
                    for (axis, label) in [(0, "X"), (2, "Z")] {
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
                "Galaxy map: ±{} ly • waypoint coordinates (ly)",
                rules.galaxy_half_extent_ly
            ));
            ui.horizontal_wrapped(|ui| {
                for (axis, value) in ["X", "Z"].into_iter().zip(&mut self.navigation.galaxy) {
                    ui.label(axis);
                    ui.add(
                        egui::DragValue::new(value)
                            .range(-rules.galaxy_half_extent_ly..=rules.galaxy_half_extent_ly),
                    );
                }
                if ui.button("Preview deep-space move").clicked() {
                    self.navigation.target = Some(MoveTarget::DeepSpace(GalaxyPosition {
                        x: self.navigation.galaxy[0],
                        y: 0,
                        z: self.navigation.galaxy[1],
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
        });
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
                let runs = plan
                    .duration_ticks
                    .div_ceil(self.state.movement_rules.ticks_per_navigation_run);
                let mut plot = false;
                ui.horizontal_wrapped(|ui| {
                    ui.strong(plan.mode.label());
                    location_stat(ui, &plan.destination, systems);
                    stat(ui, Icon::Distance, CYAN, &distance, "Distance");
                    stat(
                        ui,
                        Icon::Fuel,
                        CYAN,
                        &plan.fuel_per_ship.to_string(),
                        "Fuel per ship, charged at departure",
                    );
                    stat(
                        ui,
                        Icon::Execute,
                        GREEN,
                        &runs.to_string(),
                        "Navigation runs, at least",
                    );
                    plot = icons::button(ui, Icon::Plot, "Plot course").clicked();
                });
                if plot {
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
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
    }

    pub fn current_system(&self) -> Option<SystemId> {
        self.selected_ship().location.system()
    }
}
#[cfg(test)]
mod tests;
