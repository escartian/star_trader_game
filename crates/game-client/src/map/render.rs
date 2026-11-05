use super::{Map, pick, position, visuals};
use egui::{Color32, Rect, Sense, Stroke, Vec2};

impl Map {
    pub(super) fn draw_galaxy(&mut self, ui: &mut egui::Ui) -> Rect {
        let height = (ui.available_height() * 0.65).clamp(220.0, 540.0);
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), height),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        // Every background, glow and label shares the map's clip rectangle.
        let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
        visuals::background(&painter, rect, "GALAXY / light-years");
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
            .map(|p| pick(&self.systems, &self.camera, rect, p))
            .unwrap_or_default();
        if response.clicked() || response.double_clicked() {
            let candidates = response
                .interact_pointer_pos()
                .map(|p| pick(&self.systems, &self.camera, rect, p))
                .unwrap_or_default();
            self.select_targets(candidates, response.double_clicked());
            ui.ctx().request_repaint();
        }
        if !hover.is_empty() {
            response.clone().on_hover_ui(|ui| {
                for system in self.systems.iter().filter(|s| hover.contains(&s.id)) {
                    ui.label(format!("{} • {}", system.name, system.star_kind.label()));
                }
                if hover.len() > 1 {
                    ui.label("Click to list all overlapping targets.");
                }
            });
        }
        if let (Some(origin), Some(target)) = (
            self.route_origin(),
            self.systems.iter().find(|s| Some(s.id) == self.selected),
        ) {
            let route = [
                self.camera.project(position(origin), rect),
                self.camera.project(position(target), rect),
            ];
            painter.line_segment(
                route,
                Stroke::new(5.0_f32, Color32::from_rgba_unmultiplied(90, 220, 210, 24)),
            );
            painter.line_segment(route, Stroke::new(1.0_f32, Color32::from_rgb(92, 180, 183)));
        }
        // Paint the selected star last so it stays visible.
        let mut order: Vec<_> = self.systems.iter().collect();
        order.sort_by_key(|s| Some(s.id) == self.selected);
        let mut labels: Vec<Rect> = Vec::new();
        // Reserve selected label before admitting less important labels.
        if let Some(system) = order.iter().find(|s| Some(s.id) == self.selected) {
            let point = self.camera.project(position(system), rect);
            let galley = painter.layout_no_wrap(
                system.name.clone(),
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
            labels.push(Rect::from_min_size(
                point + Vec2::new(20.0, -20.0),
                galley.size(),
            ));
        }
        for system in order {
            let point = self.camera.project(position(system), rect);
            if !rect.expand(50.0).contains(point) {
                continue;
            }
            let selected = Some(system.id) == self.selected;
            visuals::star(
                &painter,
                point,
                system.star_kind,
                selected,
                hover.contains(&system.id),
            );
            let galley = painter.layout_no_wrap(
                system.name.clone(),
                egui::FontId::proportional(14.0),
                if selected {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(174, 192, 215)
                },
            );
            let label_rect = Rect::from_min_size(point + Vec2::new(20.0, -20.0), galley.size());
            if selected
                || (!labels.iter().any(|r| r.expand(6.0).intersects(label_rect))
                    && rect.contains_rect(label_rect))
            {
                labels.push(label_rect);
                painter.rect_filled(label_rect.expand(3.0), 3.0, Color32::from_black_alpha(150));
                painter.galley(label_rect.min, galley, Color32::WHITE);
            }
        }
        if self.scene == super::fixtures::Scene::Session {
            let art = super::art::Artwork::preload(ui.ctx());
            for system in &self.systems {
                let docked: Vec<_> = self
                    .session
                    .ships()
                    .iter()
                    .filter(|ship| ship.location == game_core::travel::Location::Docked(system.id))
                    .collect();
                if let Some(ship) = docked.first() {
                    let point =
                        self.camera.project(position(system), rect) + Vec2::new(-24.0, 22.0);
                    art.paint(super::art::ArtKind::from(ship.kind), &painter, point, 30.0);
                    painter.text(
                        point + Vec2::new(17.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{} ships", docked.len()),
                        egui::FontId::proportional(11.0),
                        Color32::LIGHT_GREEN,
                    );
                }
            }
        }
        visuals::axes(&painter, rect, self.camera.rotation());

        rect
    }
}
