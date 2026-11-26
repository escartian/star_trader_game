mod campaign;
pub use campaign::CampaignMarks;
pub(crate) use campaign::fortress;
pub(crate) use raiders::dash;
mod framing;
mod labels;
mod layout;
mod police;
mod raiders;
mod render;
use layout::{display_position, moon_parent, short_name};
mod scale;
mod sheet;
mod ships;
mod traders;

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

/// Distance in points from `pointer` to the nearest pickable body centre.
fn nearest_body(system: &StarSystem, camera: &Camera, rect: Rect, pointer: Pos2) -> Option<f32> {
    pick(system, camera, rect, pointer)
        .iter()
        .filter_map(|id| system.bodies.iter().find(|body| body.id == *id))
        .map(|body| pointer.distance(camera.project(display_position(system, body), rect)))
        .min_by(f32::total_cmp)
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
    /// Faction police patrols with their faction's colour (drawn when in this system).
    police: Vec<crate::session::PatrolMarker>,
    /// NPC traders in this system (see `traders`).
    traders: Vec<crate::session::MapTrader>,
    /// Trader and band markers drawn last frame, for taps.
    npc_hits: Vec<crate::map::npc_marker::NpcHit>,
    /// A trader or band tapped this frame (the map selects it).
    npc_request: Option<game_core::pursuit::Quarry>,
    /// (selected NPC, pursued NPC, pursuing flagship) for rings and the line.
    npc_focus: (
        Option<game_core::pursuit::Quarry>,
        Option<game_core::pursuit::Quarry>,
        Option<game_core::fleet::FleetId>,
    ),
    /// The game's landing rules, for a surface's ore yield on the info card.
    landing: game_core::movement::LandingRules,
    /// The Raider Haven and chartered ports (see `campaign`).
    campaign: CampaignMarks,
    trade_options: Vec<crate::session::TradeOption>,
    market_request: Option<game_core::travel::ShipId>,
    selection_request: Option<game_core::travel::ShipId>,
    move_request: Option<game_core::movement::MoveTarget>,
    /// The map's area last frame; the sheet (drawn before the map) uses it.
    last_rect: Rect,
    /// Whether NPC traffic is drawn (the player's map option).
    show_traffic: bool,
    /// The traffic button was pressed this frame (see `take_traffic_toggle`).
    traffic_toggled: bool,
}

impl LocalView {
    pub fn new(system: StarSystem) -> Self {
        let selected = system.bodies.first().map(|body| body.id);
        Self {
            system,
            camera: Self::initial_camera(),
            selected,
            candidates: Vec::new(),
            art: None,
            preview_route: Vec::new(),
            fleet_hits: Vec::new(),
            trade_ready: Vec::new(),
            contacts: Vec::new(),
            bands: Vec::new(),
            police: Vec::new(),
            traders: Vec::new(),
            npc_hits: Vec::new(),
            npc_request: None,
            npc_focus: (None, None, None),
            landing: game_core::movement::LandingRules::default(),
            campaign: CampaignMarks::default(),
            headings: Vec::new(),
            trade_options: Vec::new(),
            market_request: None,
            selection_request: None,
            move_request: None,
            last_rect: Rect::NOTHING,
            show_traffic: true,
            traffic_toggled: false,
            frame_overview: true,
            active_ship: game_core::travel::ShipId(1),
        }
    }

    pub fn set_bands(&mut self, bands: Vec<game_core::navigation::RaiderBand>) {
        self.bands = bands;
    }

    /// The body selected in this view, if any.
    pub fn selected_body(&self) -> Option<BodyId> {
        self.selected
    }

    pub fn set_traders(&mut self, traders: Vec<crate::session::MapTrader>) {
        self.traders = traders;
    }

    pub fn set_npc_focus(
        &mut self,
        focus: (
            Option<game_core::pursuit::Quarry>,
            Option<game_core::pursuit::Quarry>,
            Option<game_core::fleet::FleetId>,
        ),
    ) {
        self.npc_focus = focus;
    }

    pub fn take_npc_request(&mut self) -> Option<game_core::pursuit::Quarry> {
        self.npc_request.take()
    }

    pub fn set_police(&mut self, police: Vec<crate::session::PatrolMarker>) {
        self.police = police;
    }

    pub fn set_landing(&mut self, landing: game_core::movement::LandingRules) {
        self.landing = landing;
    }

    pub fn set_show_traffic(&mut self, show: bool) {
        self.show_traffic = show;
    }

    /// True once after the player pressed the traffic button.
    pub fn take_traffic_toggle(&mut self) -> bool {
        std::mem::take(&mut self.traffic_toggled)
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
