mod camera;
mod fixtures;
mod visuals;

use camera::Camera;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use fixtures::Scene;
use game_core::{StarSystem, SystemId};
use glam::DVec3;

pub struct Map {
    session_systems: Vec<StarSystem>,
    systems: Vec<StarSystem>,
    selected: Option<SystemId>,
    camera: Camera,
    candidates: Vec<SystemId>,
    scene: Scene,
    session_selection: Option<SystemId>,
    pan_tool: bool,
}

fn position(system: &StarSystem) -> DVec3 {
    DVec3::new(
        system.position.x as f64,
        system.position.y as f64,
        system.position.z as f64,
    )
}

fn pick(systems: &[StarSystem], camera: &Camera, rect: Rect, pointer: Pos2) -> Vec<SystemId> {
    systems
        .iter()
        .filter(|s| {
            let point = camera.project(position(s), rect);
            rect.contains(point) && pointer.distance(point) <= 24.0
        })
        .map(|s| s.id)
        .collect()
}

impl Map {
    pub fn new(systems: Vec<StarSystem>) -> Self {
        let selected = systems.first().map(|s| s.id);
        Self {
            session_systems: systems.clone(),
            systems,
            selected,
            camera: Camera::default(),
            candidates: Vec::new(),
            scene: Scene::Session,
            session_selection: selected,
            pan_tool: false,
        }
    }

    fn change_scene(&mut self, scene: Scene) {
        if self.scene == Scene::Session {
            self.session_selection = self.selected;
        }
        self.scene = scene;
        self.systems = if scene == Scene::Session {
            self.session_systems.clone()
        } else {
            fixtures::systems(scene)
        };
        self.selected = if scene == Scene::Session {
            self.session_selection
        } else {
            self.systems.first().map(|s| s.id)
        };
        self.camera = Camera::default();
        // An exact front view makes the deliberately overlapping fixture reproducible.
        if scene == Scene::Overlap {
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
        }
        self.candidates.clear();
    }

    fn focus_selection(&mut self) {
        if let Some(system) = self.systems.iter().find(|s| Some(s.id) == self.selected) {
            self.camera.focus_on(position(system));
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Star Trader");
                ui.label("Offline session • galaxy navigation");
                ui.horizontal_wrapped(|ui| {
                    for scene in [Scene::Session, Scene::Overlap, Scene::Dense] {
                        if ui
                            .selectable_label(self.scene == scene, scene.label())
                            .clicked()
                        {
                            self.change_scene(scene);
                        }
                    }
                });
                if self.scene != Scene::Session {
                    ui.label("Navigation test scene • separate from your session");
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Reset").clicked() {
                        self.camera = Camera::default();
                    }
                    if ui.button("Focus selection (F)").clicked() {
                        self.focus_selection();
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
                    self.focus_selection();
                }
                ui.label("Tap to select • drag to orbit • Pan tool to move • wheel/slider to zoom");
                let height = (ui.available_height() * 0.65).clamp(220.0, 540.0);
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(ui.available_width(), height),
                    Sense::click_and_drag(),
                );
                let rect = response.rect;
                // Every background, glow and label shares the map's clip rectangle.
                let painter = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
                visuals::background(&painter, rect);
                if response.dragged_by(egui::PointerButton::Middle)
                    || (self.pan_tool && response.dragged_by(egui::PointerButton::Primary))
                {
                    self.camera.pan += ui.input(|i| i.pointer.delta());
                } else if response.dragged_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Secondary)
                {
                    let delta = ui.input(|i| i.pointer.delta());
                    self.camera.yaw += f64::from(delta.x) * 0.008;
                    self.camera.pitch =
                        (self.camera.pitch + f64::from(delta.y) * 0.008).clamp(-1.57, 1.57);
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
                if response.clicked() {
                    self.candidates = response
                        .interact_pointer_pos()
                        .map(|p| pick(&self.systems, &self.camera, rect, p))
                        .unwrap_or_default();
                    if self.candidates.len() == 1 {
                        self.selected = self.candidates.first().copied();
                    }
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
                    self.systems.first(),
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
                    painter
                        .line_segment(route, Stroke::new(1.0_f32, Color32::from_rgb(92, 180, 183)));
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
                    let label_rect =
                        Rect::from_min_size(point + Vec2::new(20.0, -20.0), galley.size());
                    if selected
                        || (!labels.iter().any(|r| r.expand(6.0).intersects(label_rect))
                            && rect.contains_rect(label_rect))
                    {
                        labels.push(label_rect);
                        painter.rect_filled(
                            label_rect.expand(3.0),
                            3.0,
                            Color32::from_black_alpha(150),
                        );
                        painter.galley(label_rect.min, galley, Color32::WHITE);
                    }
                }
                let axis_origin = rect.left_bottom() + Vec2::new(35.0, -35.0);
                for (axis, label, color) in [
                    (DVec3::X, "X", Color32::LIGHT_RED),
                    (DVec3::Y, "Y", Color32::LIGHT_GREEN),
                    (DVec3::Z, "Z", Color32::LIGHT_BLUE),
                ] {
                    let axis = self.camera.rotation() * axis * 23.0;
                    let end = axis_origin + Vec2::new(axis.x as f32, -axis.y as f32);
                    painter.line_segment([axis_origin, end], Stroke::new(1.0_f32, color));
                    painter.text(
                        end,
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        color,
                    );
                }
                if self.candidates.len() > 1 {
                    ui.group(|ui| {
                        ui.label("Overlapping targets — choose one:");
                        for system in self
                            .systems
                            .iter()
                            .filter(|s| self.candidates.contains(&s.id))
                        {
                            let distance = self
                                .systems
                                .first()
                                .map_or(0.0, |s| position(s).distance(position(system)));
                            if ui
                                .selectable_label(
                                    Some(system.id) == self.selected,
                                    format!(
                                        "{} • {} • {:.2} ly",
                                        system.name,
                                        system.star_kind.label(),
                                        distance
                                    ),
                                )
                                .clicked()
                            {
                                self.selected = Some(system.id);
                            }
                        }
                    });
                }
                if let Some(system) = self.systems.iter().find(|s| Some(s.id) == self.selected) {
                    let origin = self.systems.first();
                    let distance = origin.map_or(0.0, |s| position(s).distance(position(system)));
                    ui.label(format!("{} • {}", system.name, system.star_kind.label()));
                    ui.label(format!(
                        "{:.2} light-years from {} • true 3D distance",
                        distance,
                        origin.map_or("origin", |s| s.name.as_str())
                    ));
                    if !rect.contains(self.camera.project(position(system), rect)) {
                        ui.label("Selection is outside the map. Use Focus selection to reveal it.");
                    }
                }
                egui::CollapsingHeader::new("Systems & star legend").show(ui, |ui| {
                    ui.label("Symbolic sizes • background stars are decorative");
                    egui::ScrollArea::vertical()
                        .id_salt("systems")
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for system in &self.systems {
                                ui.horizontal(|ui| {
                                    ui.colored_label(visuals::appearance(system.star_kind).0, "●");
                                    if ui
                                        .selectable_label(
                                            Some(system.id) == self.selected,
                                            format!(
                                                "{} — {}",
                                                system.name,
                                                system.star_kind.label()
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.selected = Some(system.id);
                                    }
                                });
                            }
                        });
                });
                ui.separator();
                ui.heading("Market layout fixture");
                ui.label("Sample prices only; trading is not implemented.");
                egui::ScrollArea::both()
                    .id_salt("market")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        egui::Grid::new("market_table")
                            .striped(true)
                            .min_col_width(75.0)
                            .show(ui, |ui| {
                                for title in ["Commodity", "Buy (cr)", "Sell (cr)", "Stock"] {
                                    ui.strong(title);
                                }
                                ui.end_row();
                                for (name, buy, sell, stock) in [
                                    ("Food", 12, 10, 80),
                                    ("Ore", 30, 26, 40),
                                    ("Medicine", 70, 62, 15),
                                    ("Machinery", 90, 80, 20),
                                    ("Textiles", 20, 17, 65),
                                    ("Electronics", 110, 98, 10),
                                ] {
                                    ui.label(name);
                                    ui.label(buy.to_string());
                                    ui.label(sell.to_string());
                                    ui.label(stock.to_string());
                                    ui.end_row();
                                }
                            });
                    });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_targets_are_all_pickable_and_distance_is_view_independent() {
        let systems = fixtures::systems(Scene::Overlap);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
        let mut camera = Camera {
            yaw: 0.0,
            pitch: 0.0,
            ..Camera::default()
        };
        assert_eq!(pick(&systems, &camera, rect, rect.center()).len(), 5);
        let distance = position(&systems[0]).distance(position(&systems[4]));
        assert_eq!(distance, 16.0);
        camera.yaw = 1.0;
        assert_ne!(
            camera.project(position(&systems[0]), rect),
            camera.project(position(&systems[4]), rect)
        );
        assert_eq!(
            position(&systems[0]).distance(position(&systems[4])),
            distance
        );
        assert!(pick(&systems, &camera, rect, Pos2::new(-100.0, -100.0)).is_empty());
    }

    #[test]
    fn test_scenes_do_not_replace_session_selection() {
        let mut map = Map::new(game_core::demo_galaxy());
        map.selected = Some(SystemId(2));
        map.change_scene(Scene::Dense);
        map.selected = Some(SystemId(1010));
        map.change_scene(Scene::Session);
        assert_eq!(map.selected, Some(SystemId(2)));
        assert_eq!(map.systems, game_core::demo_galaxy());
    }

    #[test]
    fn phone_and_desktop_layouts_tessellate_all_scenes() {
        let ctx = egui::Context::default();
        let mut map = Map::new(game_core::demo_galaxy());
        for size in [Vec2::new(320.0, 480.0), Vec2::new(1000.0, 720.0)] {
            for scene in [Scene::Session, Scene::Overlap, Scene::Dense] {
                map.change_scene(scene);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| map.ui(ctx),
                );
                assert!(
                    !ctx.tessellate(output.shapes, output.pixels_per_point)
                        .is_empty()
                );
            }
        }
    }
}
