use super::{Map, pick, position, visuals};
use egui::{Color32, Rect, Sense, Stroke, Vec2};

impl Map {
    pub(super) fn draw_galaxy(&mut self, ui: &mut egui::Ui) -> Rect {
        let (response, painter) = ui.allocate_painter(
            ui.available_size().max(Vec2::splat(1.0)),
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
            self.camera.zoom = crate::map::camera::zoom_input(ui, self.camera.zoom);
        }
        let hover = response
            .hover_pos()
            .map(|p| pick(&self.systems, &self.camera, rect, p))
            .unwrap_or_default();
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
            self.route_position(),
            self.systems.iter().find(|s| Some(s.id) == self.selected),
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
            let art = super::art::Artwork::preload(ui.ctx());
            let mut reserved = labels;
            reserved.extend(self.systems.iter().map(|system| {
                Rect::from_center_size(
                    self.camera.project(position(system), rect),
                    Vec2::splat(45.0),
                )
            }));
            let trade_ready = self.session.trade_ready_ships();
            for fleet in self.session.fleets() {
                let Some(ship) = fleet
                    .ships
                    .first()
                    .and_then(|id| self.session.ships().iter().find(|ship| ship.id == *id))
                else {
                    continue;
                };
                let Some(world) = self.session.ship_position(ship.id, &self.systems) else {
                    continue;
                };
                let anchor = self.camera.project(world, rect);
                if let Some(hit) = super::fleets::paint(
                    &painter,
                    &art,
                    rect,
                    anchor,
                    super::fleets::FleetSymbol {
                        fleet,
                        ship,
                        selected: fleet.ships.contains(&self.session.selected_ship().id),
                        berth: super::fleets::Berth::new(
                            &ship.location,
                            trade_ready.contains(&ship.id),
                            0.0,
                        ),
                    },
                    &mut reserved,
                ) {
                    self.galaxy_fleet_hits.push(hit);
                }
            }
        }
        if self.scene == super::fixtures::Scene::Session {
            let mut reserved = self
                .galaxy_fleet_hits
                .iter()
                .map(|hit| hit.bounds)
                .collect::<Vec<_>>();
            reserved.extend(self.systems.iter().map(|system| {
                Rect::from_center_size(
                    self.camera.project(position(system), rect),
                    Vec2::splat(54.0),
                )
            }));
            for trader in self.session.traffic() {
                let Some(ship) = trader.state.ships.first() else {
                    continue;
                };
                let Some(world) = trader.state.navigation_position(ship.id, &self.systems) else {
                    continue;
                };
                let world =
                    glam::DVec3::new(world.x_km as f64, world.y_km as f64, world.z_km as f64)
                        / game_core::movement::KM_PER_LIGHT_YEAR as f64;
                let anchor = self.camera.project(world, rect);
                if let Some(bounds) = super::fleets::place(anchor, rect, &reserved) {
                    reserved.push(bounds);
                    let center = bounds.center();
                    painter
                        .line_segment([anchor, center], Stroke::new(0.5_f32, crate::theme::AMBER));
                    painter.circle_stroke(center, 12.0, Stroke::new(1.0_f32, crate::theme::AMBER));
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "NPC",
                        egui::FontId::monospace(9.0),
                        crate::theme::AMBER,
                    );
                    painter.text(
                        center + Vec2::new(0.0, 17.0),
                        egui::Align2::CENTER_TOP,
                        &trader.name,
                        egui::FontId::proportional(10.0),
                        crate::theme::AMBER,
                    );
                }
            }
        }
        if (response.clicked() || response.double_clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            if let Some(hit) = super::fleets::pick(&self.galaxy_fleet_hits, pointer) {
                self.session.select_map_fleet(hit.ship);
            } else {
                let candidates = pick(&self.systems, &self.camera, rect, pointer);
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
        if let Some(pointer) = response.hover_pos()
            && let Some(hit) = super::fleets::pick(&self.galaxy_fleet_hits, pointer)
        {
            response
                .clone()
                .on_hover_text(format!("Select Fleet {}", hit.fleet.0));
        }
        visuals::axes(&painter, rect, self.camera.rotation());

        rect
    }
}
