mod framing;
mod labels;
mod layout;
mod raiders;
mod render;
use layout::{display_position, moon_parent, short_name};
mod scale;
mod sheet;
mod ships;

use super::{art::Artwork, camera::Camera};
use egui::{Pos2, Rect};
use game_core::{BodyId, LocalBody, LocalPosition, StarSystem};
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
            let point = camera.project(display_position(system, body), rect);
            rect.contains(point)
                && pointer.distance(point)
                    <= (scale::SymbolScale::new(camera.zoom).body_radius(body.kind) + 6.0).max(24.0)
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
    art: Option<Artwork>,
    frame_overview: bool,
    active_ship: game_core::travel::ShipId,
    preview_route: Vec<LocalPosition>,
    fleet_hits: Vec<crate::map::fleets::FleetHit>,
    trade_ready: Vec<game_core::travel::ShipId>,
    contacts: Vec<game_core::fleet::FleetId>,
    /// Where own fleets under way are heading (galaxy-space km).
    headings: Vec<(
        game_core::fleet::FleetId,
        game_core::movement::SpacePosition,
    )>,
    /// Roaming raider bands (drawn when in this system).
    bands: Vec<game_core::navigation::RaiderBand>,
    trade_options: Vec<crate::session::TradeOption>,
    market_request: Option<game_core::travel::ShipId>,
    selection_request: Option<game_core::travel::ShipId>,
    move_request: Option<game_core::movement::MoveTarget>,
    /// The map's area last frame; the sheet (drawn before the map) uses it.
    last_rect: Rect,
}

impl LocalView {
    pub fn new(system: StarSystem) -> Self {
        let selected = system.bodies.first().map(|body| body.id);
        Self {
            system,
            camera: Self::initial_camera(),
            selected,
            candidates: Vec::new(),
            pan_tool: false,
            art: None,
            preview_route: Vec::new(),
            fleet_hits: Vec::new(),
            trade_ready: Vec::new(),
            contacts: Vec::new(),
            bands: Vec::new(),
            headings: Vec::new(),
            trade_options: Vec::new(),
            market_request: None,
            selection_request: None,
            move_request: None,
            last_rect: Rect::NOTHING,
            frame_overview: true,
            active_ship: game_core::travel::ShipId(1),
        }
    }

    pub fn set_bands(&mut self, bands: Vec<game_core::navigation::RaiderBand>) {
        self.bands = bands;
    }

    pub fn set_preview_route(&mut self, route: Vec<LocalPosition>) {
        self.preview_route = route;
    }

    pub fn system(&self) -> &StarSystem {
        &self.system
    }

    /// Moves the shown bodies to `system`'s positions (same system, later tick).
    pub fn follow_orbits(&mut self, system: &StarSystem) {
        if system.id == self.system.id {
            self.system.bodies.clone_from(&system.bodies);
        }
    }

    pub fn set_fleet_context(&mut self, active_ship: game_core::travel::ShipId) {
        self.active_ship = active_ship;
    }

    pub fn take_fleet_selection(&mut self) -> Option<game_core::travel::ShipId> {
        self.selection_request.take()
    }

    /// Fleets that can trade in this system, as reported by the session.
    pub fn set_trade_options(&mut self, options: Vec<crate::session::TradeOption>) {
        self.trade_options = options;
    }

    /// Ship whose market should open (selecting its fleet first), if requested.
    pub fn take_market_request(&mut self) -> Option<game_core::travel::ShipId> {
        self.market_request.take()
    }

    pub fn take_movement_request(&mut self) -> Option<game_core::movement::MoveTarget> {
        self.move_request.take()
    }

    fn initial_camera() -> Camera {
        // A steeper starting view separates the port from the enlarged star silhouette.
        Camera {
            yaw: 0.0,
            pitch: 1.05,
            zoom: 1.3,
            ..Camera::default()
        }
    }

    fn focus(&mut self) {
        self.candidates.clear();
        if let Some(body) = self
            .system
            .bodies
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            self.camera.focus_on(display_position(&self.system, body));
        } else {
            self.camera.focus_on(DVec3::ZERO);
        }
    }

    /// Returns true when the player chooses the galaxy breadcrumb or Escape.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ships: &[game_core::travel::Ship],
        fleets: &[game_core::fleet::Fleet],
    ) -> bool {
        // A resumed GPU window has a new egui context; refresh shared handles too.
        self.art = Some(Artwork::preload(ctx));
        let back = !ctx.wants_keyboard_input()
            && !crate::battle::keyboard_claimed(ctx)
            && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.draw_map(ui, ships, fleets))
            .inner;
        self.last_rect = rect;
        self.map_buttons(ctx, rect, ships);
        back
    }
}

#[cfg(test)]
mod tests;
