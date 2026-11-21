pub(crate) mod art;
mod camera;
#[cfg(any(test, feature = "dev-tools"))]
mod capture_state;
mod client_options;
mod fixtures;
mod fleets;
mod galaxy_campaign;
mod galaxy_fleets;
mod interaction;
pub(crate) mod local;
mod main_menu;
mod menu_backdrop;
mod menu_docking;
mod menu_flights;
mod menu_motion;
mod menu_scene;
mod menu_trails;
mod orbit_view;
mod panels;
mod recovery;
mod render;
mod shell;
mod toolbar;
mod visuals;

use camera::Camera;
use egui::{Pos2, Rect};
use fixtures::Scene;
use game_core::{StarSystem, SystemId};
use glam::DVec3;

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
    local: Option<local::LocalView>,
    session: crate::session::LocalSession,
    /// Which page the context sheet shows.
    sheet: crate::session::SheetTab,
    /// The galaxy map's area last frame (the sheet draws before the map).
    last_map_rect: Rect,
    /// The main menu, while it is open (its save slots and page).
    save_recovery: Option<recovery::SaveRecovery>,
    music: crate::audio::Music,
    /// Picks the main menu's planet and ship flights; fixed for the whole run
    /// of the game, so they change only on relaunch.
    menu_seed: u64,
    /// The menu moment (seconds) shown when animations are off: captures set
    /// it to inspect the animation frame by frame.
    menu_still_time: f64,
    /// Display options last sent to egui; `None` means egui's defaults.
    applied_options: Option<crate::preferences::ClientOptions>,
}

fn position(system: &StarSystem) -> DVec3 {
    DVec3::new(
        system.position.x as f64,
        system.position.y as f64,
        system.position.z as f64,
    )
}

/// Distance in points from `pointer` to the nearest pickable system centre.
fn nearest_system(
    systems: &[StarSystem],
    camera: &Camera,
    rect: Rect,
    pointer: Pos2,
) -> Option<f32> {
    pick(systems, camera, rect, pointer)
        .iter()
        .filter_map(|id| systems.iter().find(|system| system.id == *id))
        .map(|system| pointer.distance(camera.project(position(system), rect)))
        .min_by(f32::total_cmp)
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
        let zoom = camera::overview_zoom(&systems);
        Self {
            observed_run: session.run_number(),
            observed_system: session.current_system(),
            observed_position: session.galaxy_position(&systems),
            galaxy_fleet_hits: Vec::new(),
            session_systems: systems.clone(),
            systems,
            selected,
            camera: Camera {
                zoom,
                ..Camera::default()
            },
            candidates: Vec::new(),
            scene: Scene::Session,
            session_selection: selected,
            local: None,
            session,
            sheet: crate::session::SheetTab::default(),
            last_map_rect: Rect::NOTHING,
            save_recovery: None,
            music: crate::audio::Music::silent(),
            menu_seed: fastrand::u64(..),
            menu_still_time: menu_backdrop::STILL_TIME,
            applied_options: None,
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn autosave(&mut self) -> Result<(), String> {
        if self.save_recovery.is_some() {
            // The temporary fresh session must not replace an incompatible save.
            return Ok(());
        }
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
        self.apply_client_options(ctx);
        if self.save_recovery.is_some() {
            self.show_main_menu(ctx);
            return;
        }
        if let Some(systems) = self.session.take_loaded_galaxy() {
            let zoom = camera::overview_zoom(&systems);
            self.session_systems = systems.clone();
            self.systems = systems;
            self.scene = Scene::Session;
            self.selected = self
                .session
                .display_system()
                .or_else(|| self.systems.first().map(|system| system.id));
            self.session_selection = self.selected;
            self.local = None;
            self.camera = Camera {
                zoom,
                ..Camera::default()
            };
            self.candidates.clear();
        }
        let _ = art::Artwork::preload(ctx);
        // Step an advance and run glides before following transitions, so the camera
        // tracks this frame's run. Captures turn animation off and show final positions.
        let animate = ctx.style().animation_time > 0.0;
        if let Some(delay) = self.session.frame(ctx.input(|input| input.time), animate) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(delay));
        }
        if let Some(tab) = self.session.take_sheet_request() {
            self.sheet = tab;
        }
        self.follow_orbits();
        self.follow_navigation_transitions();
        self.handle_focus_request();
        crate::battle::claim_keyboard(ctx, self.session.battle_open());
        // Strip, action bar and sheet first; the map takes the space left.
        self.shell(ctx);
        if self.local.is_some() {
            self.local_ui(ctx);
        } else {
            self.galaxy_ui(ctx);
        }
        self.session.show_market(ctx, &self.session_systems);
        self.session.show_fleets(ctx, &self.session_systems);
        self.session.show_session_controls(ctx);
        self.session.show_settings(ctx);
        self.session.show_navigation(ctx, &self.session_systems);
        self.session.show_battle(ctx);
        self.session.show_combat_report(ctx);
        self.session.show_victory(ctx, &self.session_systems);
        self.session.show_factions(ctx, &self.session_systems);
        // Taps on the strip or sheet may ask for another page: show it at once.
        if let Some(tab) = self.session.take_sheet_request() {
            self.sheet = tab;
        }
        if self.session.take_main_menu_request() {
            self.sheet = crate::session::SheetTab::default();
            #[cfg(not(target_arch = "wasm32"))]
            self.open_save_chooser(self.session.save_path(), None);
        }
    }

    /// Session bodies drawn where their orbits are now (gliding after a run).
    fn follow_orbits(&mut self) {
        let (from, to, progress) = self.session.orbit_clock();
        self.session_systems =
            orbit_view::display_systems(&self.session_systems, from, to, progress);
        if self.scene != Scene::Session {
            return;
        }
        self.systems.clone_from(&self.session_systems);
        if let Some(local) = &mut self.local
            && let Some(system) = self
                .session_systems
                .iter()
                .find(|system| system.id == local.system().id)
        {
            local.follow_orbits(system);
        }
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
        local.set_bands(self.session.raider_bands().to_vec());
        local.set_police(self.session.police_patrols());
        local.set_landing(self.session.landing_rules().clone());
        local.set_campaign_marks(self.session.campaign_marks());
        local.set_headings(self.session.fleet_headings(&self.session_systems));
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
            self.sheet = crate::session::SheetTab::Fleets;
        }
        if let Some(target) = local.take_movement_request() {
            self.sheet = crate::session::SheetTab::Selection;
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
        if !ctx.wants_keyboard_input()
            && !crate::battle::keyboard_claimed(ctx)
            && ctx.input(|i| i.key_pressed(egui::Key::F))
        {
            self.focus_selection();
        }
        self.last_map_rect = rect;
        self.galaxy_buttons(ctx, rect);
    }
}

#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod trade_tests;
