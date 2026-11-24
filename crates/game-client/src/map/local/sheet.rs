//! The system map's parts of the phone-first shell: its Selection sheet
//! (trade shortcuts, destinations and the selected body) and the small
//! floating camera buttons. Tapping a body tile or the body on the map plots
//! the course; leaving the system is planned by picking a destination elsewhere. The sheet is drawn before the map each frame, so
//! it works from the map's area of the previous frame.
use super::{LocalView, display_position, kilometers, moon_parent, short_name, view_position};
use crate::map::art::ArtKind;
use crate::map::toolbar::{CameraAction, map_buttons};
use crate::theme::icons::{self, Icon, stat};
use egui::Rect;
use game_core::BodyKind;

impl LocalView {
    /// The Selection sheet while this system is shown.
    pub fn selection_sheet(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.system.name.to_uppercase())
                .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                .color(crate::theme::GREEN),
        );
        self.trade_buttons(ui);
        self.details(ui, self.last_rect);
    }

    /// Overview, focus and reset buttons at the map's corner; F focuses the
    /// selection.
    pub(super) fn map_buttons(
        &mut self,
        ctx: &egui::Context,
        rect: Rect,
        ships: &[game_core::travel::Ship],
    ) {
        let action = map_buttons(
            ctx,
            rect,
            "local-map-buttons",
            &[
                CameraAction::Overview,
                CameraAction::FocusFleet,
                CameraAction::Reset,
            ],
        );
        match action {
            Some(CameraAction::Reset) => {
                self.camera = Self::initial_camera();
                self.frame_overview = true;
            }
            Some(CameraAction::Overview) => self.frame_overview = true,
            Some(CameraAction::FocusSelection) => self.focus(),
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
        if !ctx.wants_keyboard_input()
            && !crate::battle::keyboard_claimed(ctx)
            && ctx.input(|i| i.key_pressed(egui::Key::F))
        {
            self.focus();
        }
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
            ui.horizontal_wrapped(|ui| {
                ui.strong(short_name(&self.system, body));
                ui.label(body.kind.label());
                if let Some(surface) = &body.surface {
                    let base = self.landing.ore_per_ship(body.kind);
                    icons::surface_traits(ui, surface, surface.ore_per_ship(base));
                }
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
            });
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
