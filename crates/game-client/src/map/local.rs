mod render;
mod scale;

use super::{
    art::{ArtKind, Artwork},
    camera::Camera,
};
use egui::{Pos2, Rect};
use game_core::{BodyId, LocalBody, LocalPosition, StarSystem};
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
            let point = camera.project(view_position(body), rect);
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
        }
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
            self.camera.focus_on(view_position(body));
        } else {
            self.camera.focus_on(DVec3::ZERO);
        }
    }

    /// Returns true when the player chooses the galaxy breadcrumb or Escape.
    pub fn ui(&mut self, ctx: &egui::Context) -> bool {
        // A resumed GPU window has a new egui context; refresh shared handles too.
        self.art = Some(Artwork::preload(ctx));
        let mut back =
            !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("local_page")
                .show(ui, |ui| {
                    self.header(ui, ctx, &mut back);
                    let rect = self.draw_map(ui);
                    self.details(ui, rect);
                });
        });
        back
    }
    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, back: &mut bool) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Back to galaxy").clicked() {
                *back = true;
            }
            ui.label("/");
            ui.heading(&self.system.name);
        });
        ui.label(format!("{} • local system", self.system.star_kind.label()));
        ui.label("Inspecting this system • your ship location is shown below; docking is not implemented");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Reset").clicked() {
                self.camera = Self::initial_camera();
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
        if !ctx.wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus();
        }
        ui.label("Tap a destination • drag to orbit • Pan tool to move • Escape to galaxy");
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
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            for body in &self.system.bodies {
                if ui
                    .selectable_label(Some(body.id) == self.selected, &body.name)
                    .clicked()
                {
                    self.selected = Some(body.id);
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
            ui.label(format!(
                "{:.0} km from {} • true 3D distance",
                kilometers(body.position).length(),
                self.system.name
            ));
            ui.label("Marker sizes are symbolic; local coordinates are independent of galaxy light-years.");
            if !rect.contains(self.camera.project(view_position(body), rect)) {
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
                art.preview(ArtKind::from(body.kind), ui);
            }
            art.warnings(ui);
        }
    }
}

#[cfg(test)]
mod tests;
