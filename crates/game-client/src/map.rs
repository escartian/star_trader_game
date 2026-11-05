mod art;
mod camera;
mod capture_state;
mod fixtures;
mod local;
mod panels;
mod render;
mod visuals;

use camera::Camera;
use egui::{Pos2, Rect};
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
    local: Option<local::LocalView>,
    session: crate::session::LocalSession,
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
    pub fn new(systems: Vec<StarSystem>, authority: game_server::Session) -> Self {
        let session = crate::session::LocalSession::new(authority);
        let selected = systems
            .iter()
            .find(|s| s.id == session.display_system())
            .or_else(|| systems.first())
            .map(|s| s.id);
        Self {
            session_systems: systems.clone(),
            systems,
            selected,
            camera: Camera::default(),
            candidates: Vec::new(),
            scene: Scene::Session,
            session_selection: selected,
            pan_tool: false,
            local: None,
            session,
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
            self.candidates.clear();
        }
    }

    fn select_targets(&mut self, candidates: Vec<SystemId>, enter_system: bool) {
        self.candidates = candidates;
        if self.candidates.len() == 1 {
            self.selected = self.candidates.first().copied();
            if enter_system {
                self.open_selected_system();
            }
        }
    }

    fn open_selected_system(&mut self) {
        if let Some(system) = self.systems.iter().find(|s| Some(s.id) == self.selected) {
            self.local = Some(local::LocalView::new(system.clone()));
        }
    }

    pub fn apply_preferences(
        &mut self,
        settings: crate::preferences::Preferences,
        path: Option<std::path::PathBuf>,
    ) -> Result<(), String> {
        self.session.apply_preferences(settings, path)
    }

    pub fn notify(&mut self, message: String) {
        self.session.notify(message);
    }

    pub fn autosave(&mut self) -> Result<(), String> {
        self.session.autosave()
    }

    fn route_origin(&self) -> Option<&StarSystem> {
        if self.scene == Scene::Session {
            self.systems.iter().find(|s| s.id == self.session.origin())
        } else {
            self.systems.first()
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        if let Some(systems) = self.session.take_loaded_galaxy() {
            self.session_systems = systems.clone();
            self.systems = systems;
            self.scene = Scene::Session;
            self.selected = Some(self.session.display_system());
            self.session_selection = self.selected;
            self.local = None;
            self.camera = Camera::default();
            self.candidates.clear();
        }
        let _ = art::Artwork::preload(ctx);
        egui::TopBottomPanel::bottom("ship_status").show(ctx, |ui| {
            let target = if self.scene == Scene::Session {
                self.selected
            } else {
                None
            };
            self.session.ui(ui, target, &self.session_systems);
        });
        self.session.show_market(ctx, &self.session_systems);
        self.session.show_fleets(ctx, &self.session_systems);
        self.session.show_session_controls(ctx);
        if let Some(local) = &mut self.local {
            if local.ui(ctx, self.session.ships()) {
                self.local = None;
            }
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Star Trader");
                ui.label("Offline session • galaxy navigation");
                self.session.guide(ui);
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
                                        self.candidates.clear();
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
                ui.label("Tap to select • double-click to inspect • drag to orbit • Pan tool to move • wheel/slider to zoom");
                let rect = self.draw_galaxy(ui);
                self.details(ui, rect);

            });
        });
    }
}

#[cfg(test)]
mod tests;
