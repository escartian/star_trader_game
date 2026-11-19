pub(crate) mod art;
mod camera;
#[cfg(any(test, feature = "dev-tools"))]
mod capture_state;
mod fixtures;
mod fleets;
mod interaction;
mod local;
mod panels;
mod render;
mod toolbar;
mod visuals;

use camera::Camera;
use egui::{Pos2, Rect};
use fixtures::Scene;
use game_core::{StarSystem, SystemId};
use glam::DVec3;
use toolbar::CameraAction;

pub struct Map {
    observed_run: u64,
    observed_system: Option<SystemId>,
    observed_position: Option<DVec3>,
    galaxy_fleet_hits: Vec<fleets::FleetHit>,
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
            .find(|s| Some(s.id) == session.display_system())
            .or_else(|| systems.first())
            .map(|s| s.id);
        Self {
            observed_run: session.run_number(),
            observed_system: session.current_system(),
            observed_position: session.galaxy_position(&systems),
            galaxy_fleet_hits: Vec::new(),
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

    #[cfg(any(test, feature = "dev-tools"))]
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

    fn route_position(&self) -> Option<DVec3> {
        if self.scene == Scene::Session {
            self.session.galaxy_position(&self.systems)
        } else {
            self.systems.first().map(position)
        }
    }

    /// Developer fixtures stay available but out of the way of the session view.
    #[cfg(any(test, feature = "dev-tools"))]
    fn test_scenes(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Test scenes").show(ui, |ui| {
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
        });
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        crate::theme::apply(ctx);
        if let Some(systems) = self.session.take_loaded_galaxy() {
            self.session_systems = systems.clone();
            self.systems = systems;
            self.scene = Scene::Session;
            self.selected = self
                .session
                .display_system()
                .or_else(|| self.systems.first().map(|system| system.id));
            self.session_selection = self.selected;
            self.local = None;
            self.camera = Camera::default();
            self.candidates.clear();
        }
        let _ = art::Artwork::preload(ctx);
        self.follow_navigation_transitions();
        self.handle_focus_request();
        // The map fills the window; every HUD element floats above it as a card.
        if self.local.is_some() {
            self.local_ui(ctx);
        } else {
            self.galaxy_ui(ctx);
        }
        let target = (self.scene == Scene::Session && self.local.is_none())
            .then_some(self.selected)
            .flatten();
        self.session.roster_card(ctx, &self.session_systems);
        self.session.hud(ctx, target, &self.session_systems);
        self.session.show_market(ctx, &self.session_systems);
        self.session.show_fleets(ctx, &self.session_systems);
        self.session.show_session_controls(ctx);
        self.session.show_navigation(ctx, &self.session_systems);
        self.session.show_combat_report(ctx);
    }

    fn handle_focus_request(&mut self) {
        if !self.session.take_focus_request() {
            return;
        }
        if let Some(system) = self
            .session
            .current_system()
            .and_then(|id| self.session_systems.iter().find(|system| system.id == id))
        {
            let mut view = local::LocalView::new(system.clone());
            view.focus_ship(self.session.selected_ship());
            self.local = Some(view);
        } else {
            self.local = None;
            if let Some(position) = self.session.galaxy_position(&self.session_systems) {
                self.camera.focus_on(position);
            }
        }
    }

    fn local_ui(&mut self, ctx: &egui::Context) {
        let Some(local) = &mut self.local else {
            return;
        };
        local.set_preview_route(
            self.session
                .map_route_points(local.system().id, &self.session_systems),
        );
        local.set_fleet_context(self.session.selected_ship().id);
        local.set_trade_ready(self.session.trade_ready_ships());
        local.set_contacts(self.session.contact_fleets());
        local.set_trade_options(self.session.trade_options(local.system().id));
        let display_ships = self
            .session
            .ships_for_map(local.system(), &self.session_systems);
        let fleet_records = self.session.fleets().to_vec();
        let back = local.ui(ctx, &display_ships, &fleet_records);
        if let Some(ship) = local.take_market_request() {
            // Trading from another fleet in this system switches the selection first.
            self.session.select_map_fleet(ship);
            self.session.open_market();
        }
        if let Some(ship) = local.take_fleet_selection() {
            self.session.select_map_fleet(ship);
        }
        if let Some(target) = local.take_movement_request() {
            match target {
                game_core::movement::MoveTarget::Body { system, body } => {
                    self.session.select_destination(system, Some(body))
                }
                game_core::movement::MoveTarget::LeaveSystem(system) => {
                    self.session.select_departure(system)
                }
                target => self.session.request_movement(target),
            }
        }
        if back {
            self.local = None;
        }
    }

    fn galaxy_ui(&mut self, ctx: &egui::Context) {
        let rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.draw_galaxy(ui))
            .inner;
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus_selection();
        }
        crate::theme::metal::card(
            ctx,
            "view-toolbar",
            egui::Align2::CENTER_TOP,
            egui::vec2(0.0, 10.0),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("OUTER GALAXY")
                            .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                            .color(crate::theme::GREEN),
                    );
                });
                let action = toolbar::camera_toolbar(
                    ui,
                    &mut self.camera,
                    &mut self.pan_tool,
                    &[
                        CameraAction::Reset,
                        CameraAction::FocusSelection,
                        CameraAction::FocusFleet,
                    ],
                    "Outer galaxy (light-years)\nClick: select system and set course\nDouble-click: inspect system\nDrag: orbit • Pan tool: move\nWheel or pinch: zoom",
                );
                match action {
                    Some(CameraAction::Reset) => {
                        self.camera = Camera::default();
                        self.candidates.clear();
                    }
                    Some(CameraAction::FocusSelection) => self.focus_selection(),
                    Some(CameraAction::FocusFleet) => {
                        if let Some(position) = self.route_position() {
                            self.camera.focus_on(position);
                        }
                    }
                    _ => {}
                }
                ui.set_max_width(440.0);
                self.session.guide(ui);
            },
        );
        let width = (ctx.screen_rect().width() - 600.0).clamp(300.0, 640.0);
        crate::theme::metal::card(
            ctx,
            "view-selection",
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -10.0),
            |ui| {
                ui.set_max_width(width);
                self.details(ui, rect);
                #[cfg(any(test, feature = "dev-tools"))]
                self.test_scenes(ui);
            },
        );
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod trade_tests;
