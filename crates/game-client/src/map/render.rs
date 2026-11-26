use super::{Map, pick, position, visuals};
use egui::{Color32, Rect, Sense, Stroke, Vec2};

impl Map {
    pub(super) fn draw_galaxy(&mut self, ui: &mut egui::Ui) -> Rect {
        let (response, painter) = ui.allocate_painter(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        // Empty-space clicks stay silent; hits below mark the selection cue.
        crate::audio::quiet_clicks(ui.ctx(), response.id);
        // Every background, glow and label shares the map's clip rectangle.
        let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
        // Move the camera before drawing anything, so every layer this frame
        // (territory, stars, fleets) uses the same view.
        if response.dragged() {
            self.camera.pan += ui.input(|i| i.pointer.delta());
        }
        self.camera.zoom_toward_pointer(ui, &response, 0.15);
        visuals::background(&painter, rect, "GALAXY / light-years");
        if self.scene == super::fixtures::Scene::Session
            && self.session.client_options().show_territory
        {
            self.draw_territory(&painter, rect);
        }
        let hover = response
            .hover_pos()
            .map(|p| pick(&self.systems, &self.camera, rect, p))
            .unwrap_or_default();
        // The selected fleet's course to another system (none without one).
        let bound = if self.scene == super::fixtures::Scene::Session {
            self.session.course_system()
        } else {
            self.selected
        };
        if let (Some(origin), Some(target)) = (
            self.route_position(),
            self.systems.iter().find(|s| Some(s.id) == bound),
        ) {
            let route = [
                self.camera.project(origin, rect),
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
        self.galaxy_fleet_hits.clear();
        if self.scene == super::fixtures::Scene::Session {
            self.draw_galaxy_campaign(&painter, rect);
            self.draw_galaxy_fleets(ui, &painter, rect, &labels);
            self.galaxy_npc_hits = self.draw_galaxy_threats(ui, &painter, rect);
        }
        if (response.clicked() || response.double_clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let system = super::nearest_system(&self.systems, &self.camera, rect, pointer);
            // NPC markers draw on top, so they take the tap first.
            if let Some(quarry) = super::npc_marker::pick(&self.galaxy_npc_hits, pointer) {
                self.session.select_npc(quarry);
                self.sheet = crate::session::SheetTab::Selection;
                crate::audio::mark_select(ui.ctx());
            } else if let Some(hit) =
                super::fleets::pick_over_targets(&self.galaxy_fleet_hits, pointer, system)
            {
                self.session.select_map_fleet(hit.ship);
                crate::audio::mark_select(ui.ctx());
            } else {
                let candidates = pick(&self.systems, &self.camera, rect, pointer);
                if !candidates.is_empty() {
                    crate::audio::mark_select(ui.ctx());
                }
                self.select_targets(candidates, response.double_clicked());
                if self.scene == super::fixtures::Scene::Session
                    && self.candidates.len() == 1
                    && let Some(system) = self.selected
                {
                    self.session.select_destination(system, None);
                }
            }
            ui.ctx().request_repaint();
        }
        // After the map, so the box draws on top and takes its own clicks.
        if let Some(system) = super::search::search_box(ui, rect, &self.systems) {
            self.selected = Some(system);
            self.candidates.clear();
            self.focus_selection();
        }
        if let Some(pointer) = response.hover_pos()
            && let Some(hit) = super::fleets::pick_over_targets(
                &self.galaxy_fleet_hits,
                pointer,
                super::nearest_system(&self.systems, &self.camera, rect, pointer),
            )
        {
            response
                .clone()
                .on_hover_text(format!("Select Fleet {}", hit.fleet.0));
        }
        rect
    }
}
