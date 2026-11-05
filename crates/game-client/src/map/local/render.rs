use super::{LocalView, pick, scale::SymbolScale, view_position};
use crate::map::{art::ArtKind, visuals};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use game_core::BodyKind;
use glam::DVec3;

impl LocalView {
    pub(super) fn draw_map(&mut self, ui: &mut egui::Ui) -> Rect {
        let height = (ui.available_height() * 0.65).clamp(220.0, 540.0);
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), height),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
        visuals::background(&painter, rect, "SYSTEM / kilometers");
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
            self.camera.zoom = (self.camera.zoom
                * (ui.input(|i| i.smooth_scroll_delta.y) * 0.002).exp())
            .clamp(0.3, 4.0);
        }
        let hover = response
            .hover_pos()
            .map(|p| pick(&self.system, &self.camera, rect, p))
            .unwrap_or_default();
        if response.clicked() {
            self.candidates = response
                .interact_pointer_pos()
                .map(|p| pick(&self.system, &self.camera, rect, p))
                .unwrap_or_default();
            if self.candidates.len() == 1 {
                self.selected = self.candidates.first().copied();
            }
        }
        if !hover.is_empty() {
            response.clone().on_hover_ui(|ui| {
                for body in self.system.bodies.iter().filter(|b| hover.contains(&b.id)) {
                    ui.label(format!("{} • {}", body.name, body.kind.label()));
                }
            });
        }
        let star = self.camera.project(DVec3::ZERO, rect);
        if let Some(body) = self
            .system
            .bodies
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            painter.line_segment(
                [star, self.camera.project(view_position(body), rect)],
                Stroke::new(1.0_f32, Color32::from_rgb(92, 180, 183)),
            );
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
        let mut labels: Vec<Rect> = Vec::new();
        for body in bodies {
            let point = self.camera.project(view_position(body), rect);
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
                .is_some_and(|art| art.paint(ArtKind::from(body.kind), &painter, point, size));
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
                body.name.clone(),
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            // Keep names readable near the right/bottom edges of a narrow map.
            let size = galley.size();
            let offset = scale.body_radius(body.kind) + 10.0;
            let min = Pos2::new(
                (point.x + offset)
                    .min(rect.right() - size.x - 4.0)
                    .max(rect.left() + 4.0),
                (point.y - 20.0).clamp(rect.top() + 4.0, rect.bottom() - size.y - 4.0),
            );
            let label = Rect::from_min_size(min, size);
            if rect.contains(point)
                && (selected || !labels.iter().any(|r| r.expand(6.0).intersects(label)))
            {
                labels.push(label);
                painter.rect_filled(label.expand(3.0), 3.0, Color32::from_black_alpha(150));
                painter.galley(label.min, galley, Color32::WHITE);
            }
        }
        visuals::axes(&painter, rect, self.camera.rotation());
        rect
    }
}
