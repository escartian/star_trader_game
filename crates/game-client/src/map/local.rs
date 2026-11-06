mod framing;
mod labels;
mod layout;
mod render;
use layout::{display_position, moon_parent, short_name};
mod scale;
mod ships;

use super::{
    art::{ArtKind, Artwork},
    camera::Camera,
    toolbar::{CameraAction, camera_toolbar},
};
use crate::theme::icons::{self, Icon, stat};
use egui::{Pos2, Rect};
use game_core::{BodyId, BodyKind, LocalBody, LocalPosition, StarSystem};
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
    trade_options: Vec<crate::session::TradeOption>,
    market_request: Option<game_core::travel::ShipId>,
    selection_request: Option<game_core::travel::ShipId>,
    move_request: Option<game_core::movement::MoveTarget>,
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
            trade_options: Vec::new(),
            market_request: None,
            selection_request: None,
            move_request: None,
            frame_overview: true,
            active_ship: game_core::travel::ShipId(1),
        }
    }

    pub fn set_preview_route(&mut self, route: Vec<LocalPosition>) {
        self.preview_route = route;
    }

    pub fn system(&self) -> &StarSystem {
        &self.system
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

    fn trade_buttons(&mut self, ui: &mut egui::Ui) {
        if self.trade_options.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for option in &self.trade_options {
                let place = self
                    .system
                    .bodies
                    .iter()
                    .find(|body| body.id == option.body)
                    .map_or("market", |body| short_name(&self.system, body));
                let response = if option.selected {
                    icons::button(ui, Icon::Market, &format!("TRADE · {place}"))
                        .on_hover_text(format!("Open the market at {place}"))
                } else {
                    icons::button(ui, Icon::Fleets, &format!("{} · TRADE", option.fleet))
                        .on_hover_text(format!(
                            "Switch to {} and open the market at {place}",
                            option.fleet
                        ))
                };
                if response.clicked() {
                    self.market_request = Some(option.ship);
                }
            }
        });
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
        let mut back =
            !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.draw_map(ui, ships, fleets))
            .inner;
        crate::theme::metal::card(
            ctx,
            "view-toolbar",
            egui::Align2::CENTER_TOP,
            egui::vec2(0.0, 10.0),
            |ui| self.header(ui, ctx, &mut back, ships),
        );
        let width = (ctx.screen_rect().width() - 560.0).clamp(300.0, 640.0);
        crate::theme::metal::card(
            ctx,
            "view-selection",
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -10.0),
            |ui| {
                ui.set_max_width(width);
                self.trade_buttons(ui);
                self.details(ui, rect);
            },
        );
        back
    }
    fn header(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        back: &mut bool,
        ships: &[game_core::travel::Ship],
    ) {
        ui.horizontal_wrapped(|ui| {
            if icons::icon_button(ui, Icon::Back, "Back to galaxy (Esc)").clicked() {
                *back = true;
            }
            ui.label(
                egui::RichText::new(self.system.name.to_uppercase())
                    .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                    .color(crate::theme::GREEN),
            );
            if icons::button(ui, Icon::DeepSpace, "Depart")
                .on_hover_text("Plot a departure from this system to the outer galaxy")
                .clicked()
            {
                self.move_request =
                    Some(game_core::movement::MoveTarget::LeaveSystem(self.system.id));
            }
        });
        let action = camera_toolbar(
            ui,
            &mut self.camera,
            &mut self.pan_tool,
            &[
                CameraAction::Overview,
                CameraAction::FocusFleet,
                CameraAction::FocusSelection,
                CameraAction::FocusStar,
                CameraAction::Reset,
            ],
            "Inner system (AU)\nClick a body: select destination, then Plot course\nDrag: orbit • Wheel or pinch: zoom • Esc: galaxy\nMarker sizes are symbolic.",
        );
        match action {
            Some(CameraAction::Reset) => {
                self.camera = Self::initial_camera();
                self.frame_overview = true;
            }
            Some(CameraAction::Overview) => self.frame_overview = true,
            Some(CameraAction::FocusSelection) => self.focus(),
            Some(CameraAction::FocusStar) => self.camera.focus_on(DVec3::ZERO),
            Some(CameraAction::FocusFleet) => {
                if let Some(anchor) = ships
                    .iter()
                    .find(|ship| ship.id == self.active_ship)
                    .and_then(|ship| self.ship_anchor(&ship.location))
                {
                    self.camera.focus_on(anchor);
                }
            }
            None => {}
        }
        if !ctx.wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus();
        }
    }

    fn details(&mut self, ui: &mut egui::Ui, rect: Rect) {
        if self.system.bodies.is_empty() {
            ui.label("No local destinations in this test fixture.");
        }
        if self.candidates.len() > 1 {
            ui.group(|ui| {
                ui.label("Overlapping — choose one:");
                for body in self
                    .system
                    .bodies
                    .iter()
                    .filter(|b| self.candidates.contains(&b.id))
                {
                    if ui
                        .selectable_label(
                            Some(body.id) == self.selected,
                            short_name(&self.system, body),
                        )
                        .clicked()
                    {
                        self.selected = Some(body.id);
                        self.move_request = Some(game_core::movement::MoveTarget::Body {
                            system: self.system.id,
                            body: body.id,
                        });
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            for body in &self.system.bodies {
                if self.art.as_ref().is_some_and(|art| {
                    art.tile(
                        ui,
                        ArtKind::for_body(body),
                        short_name(&self.system, body),
                        Some(body.id) == self.selected,
                    )
                    .clicked()
                }) {
                    self.selected = Some(body.id);
                    self.move_request = Some(game_core::movement::MoveTarget::Body {
                        system: self.system.id,
                        body: body.id,
                    });
                }
            }
        });
        if let Some(body) = self
            .system
            .bodies
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            let mut select = false;
            ui.horizontal_wrapped(|ui| {
                ui.strong(short_name(&self.system, body))
                    .on_hover_text(body.kind.label());
                if let Some(profile) = &body.planet {
                    icons::planet_traits(ui, profile);
                    icons::glyph(
                        ui,
                        Icon::Market,
                        crate::theme::GREEN,
                        "Resource market and shipyard at this body's orbital berth",
                    );
                }
                stat(
                    ui,
                    Icon::Distance,
                    crate::theme::CYAN,
                    &format!(
                        "{:.4} AU",
                        crate::units::kilometers_to_au(kilometers(body.position).length())
                    ),
                    &format!("Distance from {} (true 3D)", self.system.name),
                );
                if body.kind == BodyKind::Moon
                    && let Some(parent) = moon_parent(&self.system, body)
                {
                    let distance = kilometers(body.position).distance(kilometers(parent.position));
                    let note = if display_position(&self.system, body) != view_position(body) {
                        " • spacing exaggerated in the diagram"
                    } else {
                        ""
                    };
                    stat(
                        ui,
                        Icon::Orbit,
                        crate::theme::CYAN,
                        &format!("{:.5} AU", crate::units::kilometers_to_au(distance)),
                        &format!("Distance from {}{note}", parent.name),
                    );
                }
                if !rect.contains(
                    self.camera
                        .project(display_position(&self.system, body), rect),
                ) {
                    icons::glyph(
                        ui,
                        Icon::Warning,
                        crate::theme::AMBER,
                        "Destination is outside the map. Use Focus selection to reveal it.",
                    );
                }
                select = icons::button(ui, Icon::Plot, "Set destination").clicked();
            });
            if select {
                self.move_request = Some(game_core::movement::MoveTarget::Body {
                    system: self.system.id,
                    body: body.id,
                });
            }
        }
        if let Some(art) = &self.art {
            if let Some(body) = self
                .system
                .bodies
                .iter()
                .find(|body| Some(body.id) == self.selected)
            {
                egui::CollapsingHeader::new("Sensor image")
                    .show(ui, |ui| art.preview(ArtKind::for_body(body), ui));
            }
            art.warnings(ui);
        }
    }
}

#[cfg(test)]
mod tests;
