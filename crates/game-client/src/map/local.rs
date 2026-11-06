mod framing;
mod labels;
mod layout;
mod render;
use layout::{display_position, moon_parent};
mod scale;
mod ships;

use super::{
    art::{ArtKind, Artwork},
    camera::Camera,
};
use egui::{Pos2, Rect};
use game_core::{BodyId, BodyKind, LocalBody, LocalPosition, StarSystem};
use glam::DVec3;

/// Explicit view conversion only. Simulation coordinates always remain kilometers.
const KM_PER_VIEW_UNIT: f64 = 10_000_000.0;

fn kilometers(position: LocalPosition) -> DVec3 {
    DVec3::new(
        position.x_km as f64,
        position.y_km as f64,
        position.z_km as f64,
    )
}

fn view_position(body: &LocalBody) -> DVec3 {
    kilometers(body.position) / KM_PER_VIEW_UNIT
}

fn pick(system: &StarSystem, camera: &Camera, rect: Rect, pointer: Pos2) -> Vec<BodyId> {
    system
        .bodies
        .iter()
        .filter(|body| {
            let point = camera.project(display_position(system, body), rect);
            rect.contains(point)
                && pointer.distance(point)
                    <= (scale::SymbolScale::new(camera.zoom).body_radius(body.kind) + 6.0).max(24.0)
        })
        .map(|body| body.id)
        .collect()
}

pub struct LocalView {
    system: StarSystem,
    camera: Camera,
    selected: Option<BodyId>,
    candidates: Vec<BodyId>,
    pan_tool: bool,
    art: Option<Artwork>,
    frame_overview: bool,
    active_ship: game_core::travel::ShipId,
    preview_route: Vec<LocalPosition>,
    fleet_hits: Vec<crate::map::fleets::FleetHit>,
    selection_request: Option<game_core::travel::ShipId>,
    move_request: Option<game_core::movement::MoveTarget>,
}

impl LocalView {
    pub fn new(system: StarSystem) -> Self {
        let selected = system.bodies.first().map(|body| body.id);
        Self {
            system,
            camera: Self::initial_camera(),
            selected,
            candidates: Vec::new(),
            pan_tool: false,
            art: None,
            preview_route: Vec::new(),
            fleet_hits: Vec::new(),
            selection_request: None,
            move_request: None,
            frame_overview: true,
            active_ship: game_core::travel::ShipId(1),
        }
    }

    pub fn set_preview_route(&mut self, route: Vec<LocalPosition>) {
        self.preview_route = route;
    }

    pub fn system(&self) -> &StarSystem {
        &self.system
    }

    pub fn set_fleet_context(&mut self, active_ship: game_core::travel::ShipId) {
        self.active_ship = active_ship;
    }

    pub fn take_fleet_selection(&mut self) -> Option<game_core::travel::ShipId> {
        self.selection_request.take()
    }

    pub fn take_movement_request(&mut self) -> Option<game_core::movement::MoveTarget> {
        self.move_request.take()
    }

    fn initial_camera() -> Camera {
        // A steeper starting view separates the port from the enlarged star silhouette.
        Camera {
            yaw: 0.0,
            pitch: 1.05,
            zoom: 1.3,
            ..Camera::default()
        }
    }

    fn focus(&mut self) {
        self.candidates.clear();
        if let Some(body) = self
            .system
            .bodies
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            self.camera.focus_on(display_position(&self.system, body));
        } else {
            self.camera.focus_on(DVec3::ZERO);
        }
    }

    /// Returns true when the player chooses the galaxy breadcrumb or Escape.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ships: &[game_core::travel::Ship],
        fleets: &[game_core::fleet::Fleet],
    ) -> bool {
        // A resumed GPU window has a new egui context; refresh shared handles too.
        self.art = Some(Artwork::preload(ctx));
        let mut back =
            !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::CentralPanel::default().show(ctx, |ui| {
            crate::theme::metal_panel(ui);
            egui::ScrollArea::vertical()
                .id_salt("local_page")
                .show(ui, |ui| {
                    self.header(ui, ctx, &mut back);
                    ui.horizontal_wrapped(|ui| {
                        if crate::theme::icons::button(
                            ui,
                            crate::theme::icons::Icon::Focus,
                            "System overview",
                        )
                        .clicked()
                        {
                            self.frame_overview = true;
                        }
                        if crate::theme::icons::button(
                            ui,
                            crate::theme::icons::Icon::Focus,
                            "Focus selected fleet",
                        )
                        .clicked()
                            && let Some(ship) =
                                ships.iter().find(|ship| ship.id == self.active_ship)
                            && let Some(anchor) = self.ship_anchor(&ship.location)
                        {
                            self.camera.focus_on(anchor);
                        }
                    });
                    let rect = self.draw_map(ui, ships, fleets);
                    self.details(ui, rect);
                });
        });
        back
    }
    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, back: &mut bool) {
        ui.horizontal_wrapped(|ui| {
            if crate::theme::icons::button(ui, crate::theme::icons::Icon::Back, "Galaxy").clicked()
            {
                *back = true;
            }
            ui.label("/");
            ui.heading(format!("{} / INNER SYSTEM", self.system.name));
        });
        egui::CollapsingHeader::new("View / camera controls").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Reset").clicked() {
                    self.camera = Self::initial_camera();
                    self.frame_overview = true;
                }
                if ui.button("Focus selection (F)").clicked() {
                    self.focus();
                }
                if ui.button("Focus star").clicked() {
                    self.camera.focus_on(DVec3::ZERO);
                }
                ui.toggle_value(&mut self.pan_tool, "Pan tool");
                if ui.button("Top (XZ)").clicked() {
                    self.camera.yaw = 0.0;
                    self.camera.pitch = std::f64::consts::FRAC_PI_2;
                }
                if ui.button("Front (XY)").clicked() {
                    self.camera.yaw = 0.0;
                    self.camera.pitch = 0.0;
                }
                if ui.button("Side (ZY)").clicked() {
                    self.camera.yaw = std::f64::consts::FRAC_PI_2;
                    self.camera.pitch = 0.0;
                }
                ui.add(egui::Slider::new(&mut self.camera.zoom, 0.3..=4.0).text("Zoom"));
            });
        });
        if !ctx.wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus();
        }
        ui.small("Fleet / destination / plot course • drag to orbit • wheel to zoom • AU");
    }

    fn details(&mut self, ui: &mut egui::Ui, rect: Rect) {
        if self.system.bodies.is_empty() {
            ui.label("No local destinations are catalogued in this test fixture.");
        }
        if self.candidates.len() > 1 {
            ui.group(|ui| {
                ui.label("Overlapping destinations — choose one:");
                for body in self
                    .system
                    .bodies
                    .iter()
                    .filter(|b| self.candidates.contains(&b.id))
                {
                    if ui
                        .selectable_label(Some(body.id) == self.selected, &body.name)
                        .clicked()
                    {
                        self.selected = Some(body.id);
                        self.move_request = Some(game_core::movement::MoveTarget::Body {
                            system: self.system.id,
                            body: body.id,
                        });
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            for body in &self.system.bodies {
                if self.art.as_ref().is_some_and(|art| {
                    art.tile(
                        ui,
                        ArtKind::from(body.kind),
                        &body.name,
                        Some(body.id) == self.selected,
                    )
                    .clicked()
                }) {
                    self.selected = Some(body.id);
                    self.move_request = Some(game_core::movement::MoveTarget::Body {
                        system: self.system.id,
                        body: body.id,
                    });
                }
            }
        });
        if let Some(body) = self
            .system
            .bodies
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            ui.label(format!("{} • {}", body.name, body.kind.label()));
            if let Some(profile) = &body.planet {
                ui.label(format!(
                    "{} economy • {} • danger: {}",
                    profile.economy.label(),
                    profile.biome.label(),
                    profile.danger.label()
                ));
                ui.label(format!("Production specialization: {} • resource market and shipyard available at this body's orbital berth", profile.specialization.label()));
            }
            if ui
                .button(if body.kind == BodyKind::Station {
                    "Select station as destination"
                } else {
                    "Select as destination"
                })
                .clicked()
            {
                self.move_request = Some(game_core::movement::MoveTarget::Body {
                    system: self.system.id,
                    body: body.id,
                });
            }
            ui.label(format!(
                "{:.4} AU from {} • true 3D distance",
                crate::units::kilometers_to_au(kilometers(body.position).length()),
                self.system.name
            ));
            ui.label("Marker sizes are symbolic; local distances use AU, galaxy distances use light-years.");
            if body.kind == BodyKind::Moon
                && let Some(parent) = moon_parent(&self.system, body)
            {
                let distance = kilometers(body.position).distance(kilometers(parent.position));
                let note = if display_position(&self.system, body) != view_position(body) {
                    " • spacing exaggerated in the diagram"
                } else {
                    ""
                };
                ui.label(format!(
                    "{:.5} AU from {}{note}",
                    crate::units::kilometers_to_au(distance),
                    parent.name
                ));
            }
            if !rect.contains(
                self.camera
                    .project(display_position(&self.system, body), rect),
            ) {
                ui.label("Destination is outside the map. Use Focus selection to reveal it.");
            }
        }
        if let Some(art) = &self.art {
            if let Some(body) = self
                .system
                .bodies
                .iter()
                .find(|body| Some(body.id) == self.selected)
            {
                egui::CollapsingHeader::new("Sensor image")
                    .show(ui, |ui| art.preview(ArtKind::from(body.kind), ui));
            }
            art.warnings(ui);
        }
    }
}

#[cfg(test)]
mod tests;
