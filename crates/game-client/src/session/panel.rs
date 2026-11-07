//! Floating HUD cards: ship status (top-right) and the navigation run (bottom-left).
use super::{LocalSession, navigation::location_stat};
use crate::theme::icons::{Icon, button, icon_button, stat};
use crate::theme::{AMBER, CYAN, GREEN, metal};
use egui::{Align2, Vec2};
use game_core::{SystemId, travel::Location};

#[derive(Clone, Copy)]
enum Panel {
    Fleets,
    Navigation,
    Market,
    Saves,
}

impl LocalSession {
    pub fn hud(
        &mut self,
        ctx: &egui::Context,
        target: Option<SystemId>,
        systems: &[game_core::StarSystem],
    ) {
        metal::card(
            ctx,
            "hud-status",
            Align2::RIGHT_TOP,
            Vec2::new(-10.0, 10.0),
            |ui| {
                ui.set_width(180.0);
                self.status_ui(ui, systems);
            },
        );
        metal::card(
            ctx,
            "hud-run",
            Align2::LEFT_BOTTOM,
            Vec2::new(10.0, -10.0),
            |ui| {
                ui.set_max_width(250.0);
                self.run_ui(ui, target);
                self.contact_panel(ui);
            },
        );
    }

    fn status_ui(&mut self, ui: &mut egui::Ui, systems: &[game_core::StarSystem]) {
        egui::Grid::new("hud-panels")
            .num_columns(2)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for (index, (icon, label, tip, panel)) in [
                    (
                        Icon::Fleets,
                        "Fleets",
                        "Fleets, cargo transfer and shipyard",
                        Panel::Fleets,
                    ),
                    (
                        Icon::Navigation,
                        "Nav",
                        "Navigation: courses, readiness and contacts",
                        Panel::Navigation,
                    ),
                    (
                        Icon::Market,
                        "Trade",
                        "Market, refuel and hold upgrade",
                        Panel::Market,
                    ),
                    (Icon::Save, "Save", "Session, saves and log", Panel::Saves),
                ]
                .into_iter()
                .enumerate()
                {
                    let response =
                        ui.add_sized([86.0, 30.0], |ui: &mut egui::Ui| button(ui, icon, label));
                    if response.on_hover_text(tip).clicked() {
                        match panel {
                            Panel::Fleets => self.fleet_panel.open = true,
                            Panel::Navigation => self.open_navigation(),
                            Panel::Market => self.market.open = true,
                            Panel::Saves => self.saves.open = true,
                        }
                    }
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        ui.separator();
        let ships = self.state.ships.clone();
        egui::ComboBox::from_id_salt("active_ship")
            .selected_text(&self.selected_ship().name)
            .width(150.0)
            .show_ui(ui, |ui| {
                for ship in ships {
                    if ui
                        .selectable_label(ship.id == self.active_ship, &ship.name)
                        .clicked()
                    {
                        self.select_ship(ship.id);
                    }
                }
            })
            .response
            .on_hover_text("Ship used for trading");
        let ship = self.selected_ship().clone();
        stat(
            ui,
            Icon::Credits,
            AMBER,
            &self.economy.credits.to_string(),
            "Credits",
        );
        ui.horizontal(|ui| {
            stat(
                ui,
                Icon::Cargo,
                GREEN,
                &format!("{}/{}", ship.cargo_used(), ship.cargo_capacity),
                "Cargo used / capacity",
            );
            stat(
                ui,
                Icon::Fuel,
                CYAN,
                &format!("{}/{}", ship.fuel, ship.fuel_capacity),
                "Fuel / capacity",
            );
        });
        ui.horizontal(|ui| {
            stat(
                ui,
                Icon::Clock,
                CYAN,
                &self.state.tick.to_string(),
                "Simulation tick",
            );
            location_stat(ui, &ship.location, systems);
        });
    }

    fn run_ui(&mut self, ui: &mut egui::Ui, target: Option<SystemId>) {
        ui.horizontal(|ui| {
            let run = self.state.navigation.run.saturating_add(1);
            if button(ui, Icon::Execute, &format!("EXECUTE RUN {run}"))
                .on_hover_text(
                    "Advances every fleet and market together.\nUnassigned fleets must plot a course or Hold position.",
                )
                .clicked()
            {
                self.request_execute_run();
            }
            if matches!(self.selected_ship().location, Location::DeepSpace(_))
                && let Some(system) = target
                && icon_button(ui, Icon::Enter, "Preview entry into selected system").clicked()
            {
                self.request_movement(game_core::movement::MoveTarget::EnterSystem(system));
            }
        });
        if !self.notice.is_empty() {
            ui.horizontal_wrapped(|ui| {
                crate::theme::icons::glyph(ui, Icon::Info, CYAN, "Latest event");
                ui.small(&self.notice);
            });
        }
    }

    /// Roster of fleets and NPC traffic, floating at the top-left.
    pub fn roster_card(&mut self, ctx: &egui::Context, systems: &[game_core::StarSystem]) {
        let max_height = (ctx.screen_rect().height() - 190.0).max(160.0);
        metal::card(
            ctx,
            "hud-roster",
            Align2::LEFT_TOP,
            Vec2::new(10.0, 10.0),
            |ui| {
                ui.set_width(220.0);
                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .show(ui, |ui| self.map_roster(ui, systems));
            },
        );
    }
}
