use super::{LocalView, labels, layout::display_position, pick, scale::SymbolScale};
use crate::map::{art::ArtKind, visuals};
use egui::{Color32, Rect, Sense, Stroke, Vec2};
use game_core::BodyKind;
use glam::DVec3;

impl LocalView {
    pub(super) fn draw_map(
        &mut self,
        ui: &mut egui::Ui,
        ships: &[game_core::travel::Ship],
        fleets: &[game_core::fleet::Fleet],
    ) -> Rect {
        let (response, painter) = ui.allocate_painter(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
        if self.frame_overview {
            self.fit_overview(rect, ships);
            self.frame_overview = false;
        }
        visuals::background(&painter, rect, "SYSTEM / AU");
        if response.dragged_by(egui::PointerButton::Middle)
            || (self.pan_tool && response.dragged_by(egui::PointerButton::Primary))
        {
            self.camera.pan += ui.input(|i| i.pointer.delta());
        } else if response.dragged_by(egui::PointerButton::Primary)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            let delta = ui.input(|i| i.pointer.delta());
            self.camera.yaw += f64::from(delta.x) * 0.008;
            self.camera.pitch = (self.camera.pitch + f64::from(delta.y) * 0.008).clamp(-1.57, 1.57);
        }
        if response.hovered() {
            self.camera.zoom = crate::map::camera::zoom_input(ui, self.camera.zoom);
        }
        let hover = response
            .hover_pos()
            .map(|p| pick(&self.system, &self.camera, rect, p))
            .unwrap_or_default();
        if !hover.is_empty() {
            response.clone().on_hover_ui(|ui| {
                for body in self.system.bodies.iter().filter(|b| hover.contains(&b.id)) {
                    ui.label(format!(
                        "{} • {}",
                        super::layout::short_name(&self.system, body),
                        body.kind.label()
                    ));
                }
            });
        }
        let star = self.camera.project(DVec3::ZERO, rect);
        for segment in self.preview_route.windows(2) {
            let project = |position: game_core::LocalPosition| {
                let schematic = self
                    .system
                    .bodies
                    .iter()
                    .find(|body| body.position == position)
                    .map_or(
                        super::kilometers(position) / super::KM_PER_VIEW_UNIT,
                        |body| display_position(&self.system, body),
                    );
                self.camera.project(schematic, rect)
            };
            painter.line_segment(
                [project(segment[0]), project(segment[1])],
                Stroke::new(1.5_f32, crate::theme::CYAN),
            );
            painter.circle_filled(project(segment[1]), 3.0, crate::theme::CYAN);
        }
        let scale = SymbolScale::new(self.camera.zoom);
        visuals::star_with_radius(
            &painter,
            star,
            self.system.star_kind,
            scale.star_radius(self.system.star_kind),
        );
        if let Some(art) = &self.art {
            art.paint(
                ArtKind::from(self.system.star_kind),
                &painter,
                star,
                scale.star_radius(self.system.star_kind) * 2.25,
            );
        }
        let mut bodies: Vec<_> = self.system.bodies.iter().collect();
        bodies.sort_by_key(|b| Some(b.id) != self.selected);
        let marker_bounds: Vec<_> = bodies
            .iter()
            .map(|body| {
                let point = self
                    .camera
                    .project(display_position(&self.system, body), rect);
                Rect::from_center_size(
                    point,
                    Vec2::splat((scale.body_radius(body.kind) + 4.0) * 2.0),
                )
            })
            .collect();
        let mut label_bounds: Vec<Rect> = Vec::new();
        for body in bodies {
            let point = self
                .camera
                .project(display_position(&self.system, body), rect);
            let selected = Some(body.id) == self.selected;
            let radius = scale.body_radius(body.kind);
            let size = if body.kind == BodyKind::Station {
                scale.station_size()
            } else {
                radius * 2.25
            };
            let painted = self
                .art
                .as_ref()
                .is_some_and(|art| art.paint(ArtKind::for_body(body), &painter, point, size));
            if !painted {
                painter.circle_filled(point, radius, Color32::LIGHT_BLUE);
            }
            if selected || hover.contains(&body.id) {
                painter.circle_stroke(
                    point,
                    scale.body_radius(body.kind) + 5.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(130, 230, 226)),
                );
            }
            let galley = painter.layout_no_wrap(
                super::layout::short_name(&self.system, body).to_string(),
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            if rect.contains(point)
                && let Some(label) = labels::place(
                    point,
                    radius,
                    galley.size(),
                    rect,
                    &label_bounds,
                    &marker_bounds,
                    selected,
                )
            {
                label_bounds.push(label);
                painter.rect_filled(label.expand(3.0), 3.0, Color32::from_black_alpha(150));
                painter.galley(label.min, galley, Color32::WHITE);
            }
        }
        let reserved_labels = label_bounds;
        self.draw_fleets(
            &painter,
            rect,
            ships,
            fleets,
            &reserved_labels,
            &marker_bounds,
        );
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            if let Some(hit) = crate::map::fleets::pick(&self.fleet_hits, pointer) {
                self.selection_request = Some(hit.ship);
                self.active_ship = hit.ship;
            } else {
                self.candidates = pick(&self.system, &self.camera, rect, pointer);
                if self.candidates.len() == 1 {
                    self.selected = self.candidates.first().copied();
                    self.move_request =
                        self.selected
                            .map(|body| game_core::movement::MoveTarget::Body {
                                system: self.system.id,
                                body,
                            });
                }
            }
        }
        visuals::axes(&painter, rect, self.camera.rotation());
        rect
    }
}
