use super::{camera::Camera, visuals};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
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
            let point = camera.project(view_position(body), rect);
            rect.contains(point) && pointer.distance(point) <= 24.0
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
}

impl LocalView {
    pub fn new(system: StarSystem) -> Self {
        let selected = system.bodies.first().map(|body| body.id);
        Self {
            system,
            camera: Camera::default(),
            selected,
            candidates: Vec::new(),
            pan_tool: false,
        }
    }

    fn focus(&mut self) {
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
        let mut back =
            !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("local_page").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("← Galaxy").clicked() { back = true; }
                    ui.label("/");
                    ui.heading(&self.system.name);
                });
                ui.label(format!("{} • local system", self.system.star_kind.label()));
                ui.label("Fixed navigation destinations • travel and docking are not yet implemented");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Reset").clicked() { self.camera = Camera::default(); }
                    if ui.button("Focus selection (F)").clicked() { self.focus(); }
                    if ui.button("Focus star").clicked() { self.camera.focus_on(DVec3::ZERO); }
                    ui.toggle_value(&mut self.pan_tool, "Pan tool");
                    if ui.button("Top (XZ)").clicked() { self.camera.yaw = 0.0; self.camera.pitch = std::f64::consts::FRAC_PI_2; }
                    if ui.button("Front (XY)").clicked() { self.camera.yaw = 0.0; self.camera.pitch = 0.0; }
                    if ui.button("Side (ZY)").clicked() { self.camera.yaw = std::f64::consts::FRAC_PI_2; self.camera.pitch = 0.0; }
                    ui.add(egui::Slider::new(&mut self.camera.zoom, 0.3..=4.0).text("Zoom"));
                });
                if !ctx.wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::F)) { self.focus(); }
                ui.label("Tap a destination • drag to orbit • Pan tool to move • Escape to galaxy");
                let height = (ui.available_height() * 0.65).clamp(220.0, 540.0);
                let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), Sense::click_and_drag());
                let rect = response.rect;
                let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
                visuals::background(&painter, rect, "SYSTEM / kilometers");
                if response.dragged_by(egui::PointerButton::Middle)
                    || (self.pan_tool && response.dragged_by(egui::PointerButton::Primary)) {
                    self.camera.pan += ui.input(|i| i.pointer.delta());
                } else if response.dragged_by(egui::PointerButton::Primary) || response.dragged_by(egui::PointerButton::Secondary) {
                    let delta = ui.input(|i| i.pointer.delta());
                    self.camera.yaw += f64::from(delta.x) * 0.008;
                    self.camera.pitch = (self.camera.pitch + f64::from(delta.y) * 0.008).clamp(-1.57, 1.57);
                }
                if response.hovered() {
                    self.camera.zoom = (self.camera.zoom * (ui.input(|i| i.smooth_scroll_delta.y) * 0.002).exp()).clamp(0.3, 4.0);
                }
                let hover = response.hover_pos().map(|p| pick(&self.system, &self.camera, rect, p)).unwrap_or_default();
                if response.clicked() {
                    self.candidates = response.interact_pointer_pos().map(|p| pick(&self.system, &self.camera, rect, p)).unwrap_or_default();
                    if self.candidates.len() == 1 { self.selected = self.candidates.first().copied(); }
                }
                if !hover.is_empty() {
                    response.clone().on_hover_ui(|ui| {
                        for body in self.system.bodies.iter().filter(|b| hover.contains(&b.id)) {
                            ui.label(format!("{} • {}", body.name, body.kind.label()));
                        }
                    });
                }
                let star = self.camera.project(DVec3::ZERO, rect);
                if let Some(body) = self.system.bodies.iter().find(|b| Some(b.id) == self.selected) {
                    painter.line_segment([star, self.camera.project(view_position(body), rect)], Stroke::new(1.0_f32, Color32::from_rgb(92, 180, 183)));
                }
                visuals::star(&painter, star, self.system.star_kind, false, false);
                let mut bodies: Vec<_> = self.system.bodies.iter().collect();
                bodies.sort_by_key(|b| Some(b.id) != self.selected);
                let mut labels: Vec<Rect> = Vec::new();
                for body in bodies {
                    let point = self.camera.project(view_position(body), rect);
                    let selected = Some(body.id) == self.selected;
                    let color = match body.kind {
                        BodyKind::RockyPlanet => Color32::from_rgb(98, 178, 191),
                        BodyKind::GasGiant => Color32::from_rgb(212, 168, 130),
                        BodyKind::Station => Color32::from_rgb(144, 228, 199),
                    };
                    match body.kind {
                        BodyKind::Station => {
                            painter.rect_filled(Rect::from_center_size(point, Vec2::splat(10.0)), 1.0, color);
                        }
                        kind => {
                            let radius = if kind == BodyKind::GasGiant { 9.0 } else { 6.0 };
                            painter.circle_filled(point, radius, color);
                            painter.circle_filled(point - Vec2::splat(radius * 0.25), radius * 0.45, color.gamma_multiply(1.25));
                        }
                    }
                    if selected || hover.contains(&body.id) {
                        painter.circle_stroke(point, 15.0, Stroke::new(1.5_f32, Color32::from_rgb(130, 230, 226)));
                    }
                    let galley = painter.layout_no_wrap(body.name.clone(), egui::FontId::proportional(13.0), Color32::WHITE);
                    // Keep names readable near the right/bottom edges of a narrow map.
                    let size = galley.size();
                    let min = Pos2::new((point.x + 20.0).min(rect.right() - size.x - 4.0).max(rect.left() + 4.0),
                        (point.y - 20.0).clamp(rect.top() + 4.0, rect.bottom() - size.y - 4.0));
                    let label = Rect::from_min_size(min, size);
                    if rect.contains(point) && (selected || !labels.iter().any(|r| r.expand(6.0).intersects(label))) {
                        labels.push(label);
                        painter.rect_filled(label.expand(3.0), 3.0, Color32::from_black_alpha(150));
                        painter.galley(label.min, galley, Color32::WHITE);
                    }
                }
                visuals::axes(&painter, rect, self.camera.rotation());
                if self.system.bodies.is_empty() { ui.label("No local destinations are catalogued in this test fixture."); }
                if self.candidates.len() > 1 {
                    ui.group(|ui| {
                        ui.label("Overlapping destinations — choose one:");
                        for body in self.system.bodies.iter().filter(|b| self.candidates.contains(&b.id)) {
                            if ui.selectable_label(Some(body.id) == self.selected, &body.name).clicked() { self.selected = Some(body.id); }
                        }
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    for body in &self.system.bodies {
                        if ui.selectable_label(Some(body.id) == self.selected, &body.name).clicked() { self.selected = Some(body.id); }
                    }
                });
                if let Some(body) = self.system.bodies.iter().find(|b| Some(b.id) == self.selected) {
                    ui.label(format!("{} • {}", body.name, body.kind.label()));
                    ui.label(format!("{:.0} km from {} • true 3D distance", kilometers(body.position).length(), self.system.name));
                    ui.label("Marker sizes are symbolic; local coordinates are independent of galaxy light-years.");
                    if !rect.contains(self.camera.project(view_position(body), rect)) {
                        ui.label("Destination is outside the map. Use Focus selection to reveal it.");
                    }
                }
            });
        });
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_distances_are_kilometers_and_do_not_depend_on_view_scale() {
        let system = game_core::demo_galaxy().remove(0);
        let body = &system.bodies[0];
        assert!(kilometers(body.position).length() > 1_000_000.0);
        assert_eq!(
            view_position(body) * KM_PER_VIEW_UNIT,
            kilometers(body.position)
        );
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
        let mut local = LocalView::new(system);
        local.camera.yaw = 1.0;
        local.camera.zoom = 4.0;
        local.focus();
        let selected = &local.system.bodies[0];
        assert_eq!(
            local.camera.project(view_position(selected), rect),
            rect.center()
        );
        assert!(pick(&local.system, &local.camera, rect, rect.center()).contains(&selected.id));
    }

    #[test]
    fn local_overlaps_are_not_resolved_arbitrarily() {
        let mut system = game_core::demo_galaxy().remove(0);
        system.bodies[1].position = system.bodies[0].position;
        let camera = Camera::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
        let point = camera.project(view_position(&system.bodies[0]), rect);
        let candidates = pick(&system, &camera, rect, point);
        assert!(candidates.contains(&system.bodies[0].id));
        assert!(candidates.contains(&system.bodies[1].id));
    }
}
