//! The session's parts of the phone-first shell: the status strip's numbers and
//! alert badge, the action bar (Fleets, Advance, Menu) and the sheet contents
//! that belong to the session (guide, contact decision, fleet list, menu).
//! Layout (which panel goes where) is the map's job; see `map::shell`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{RichText, Vec2};
use game_core::{StarSystem, fleet::FleetId, progress::Feature};
use game_server::LOCAL_PLAYER;

/// Which page the context sheet shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetTab {
    /// Whatever was tapped on the map: a body, a system, a destination.
    #[default]
    Selection,
    Fleets,
    Menu,
}

impl LocalSession {
    /// The sheet page something asked to show, once.
    pub fn take_sheet_request(&mut self) -> Option<SheetTab> {
        self.sheet_request.take()
    }

    /// Shows what a fleet needing attention needs: its contact decision (always
    /// at the top of the sheet) or, for a fleet without orders, the Fleets page.
    pub(super) fn request_sheet_for(&mut self, fleet: FleetId) {
        self.sheet_request = Some(if self.contact_fleets().contains(&fleet) {
            SheetTab::Selection
        } else {
            SheetTab::Fleets
        });
    }

    /// Credits, run number and the alert badge. A tap on the badge selects the
    /// next fleet that needs a decision or orders.
    pub fn status_numbers(&mut self, ui: &mut egui::Ui) {
        let alerts = self.attention_fleets();
        if !alerts.is_empty() {
            let label = RichText::new(format!("! {}", alerts.len())).color(egui::Color32::BLACK);
            let badge = egui::Button::new(label)
                .fill(AMBER)
                .min_size(Vec2::new(40.0, 28.0));
            if ui
                .add(badge)
                .on_hover_text("Fleets that need a decision or orders: show the next")
                .clicked()
            {
                self.next_attention();
            }
        }
        ui.label(RichText::new(format!("Run {}", self.state.navigation.run)).color(CYAN));
        ui.label(RichText::new(format!("{} cr", self.economy.credits)).color(AMBER));
    }

    /// Fleets needing the player, contacts first. Only the flagship can need
    /// orders: secondary fleets follow standing orders.
    pub fn attention_fleets(&self) -> Vec<FleetId> {
        let mut fleets = self.contact_fleets();
        let flagship = self.state.flagship(LOCAL_PLAYER);
        for fleet in &self.state.fleets {
            let idle = Some(fleet.id) == flagship
                && self.state.navigation.order(fleet.id).is_none()
                && !self.state.navigation.holds.contains(&fleet.id)
                && self.state.trade_route(fleet.id).is_none();
            if idle && !fleets.contains(&fleet.id) {
                fleets.push(fleet.id);
            }
        }
        fleets
    }

    /// Selects the attention fleet after the current one (wrapping).
    pub fn next_attention(&mut self) {
        let fleets = self.attention_fleets();
        let current = self
            .state
            .fleet_for_ship(self.active_ship)
            .ok()
            .map(|fleet| fleet.id);
        let next = match current.and_then(|id| fleets.iter().position(|fleet| *fleet == id)) {
            Some(index) => fleets[(index + 1) % fleets.len()],
            None => match fleets.first() {
                Some(fleet) => *fleet,
                None => return,
            },
        };
        self.focus_fleet(next);
        self.request_sheet_for(next);
    }

    /// The bottom action bar: the latest message, then Fleets, Advance (with the
    /// single-run clock) and Menu.
    pub fn action_bar(&mut self, ui: &mut egui::Ui, tab: &mut SheetTab) {
        if !self.notice.is_empty() {
            ui.horizontal(|ui| {
                icons::glyph(ui, Icon::Info, CYAN, "Latest event");
                ui.add(egui::Label::new(RichText::new(&self.notice).small()).truncate());
            });
        }
        ui.columns(3, |columns| {
            let tab_button =
                |ui: &mut egui::Ui, icon, label, page: SheetTab, tab: &mut SheetTab| {
                    ui.vertical_centered(|ui| {
                        let on = *tab == page;
                        if icons::labeled_toggle(ui, icon, label, on).clicked() {
                            *tab = if on { SheetTab::Selection } else { page };
                        }
                    });
                };
            tab_button(
                &mut columns[0],
                Icon::Fleets,
                "Fleets",
                SheetTab::Fleets,
                tab,
            );
            columns[1].vertical_centered(|ui| {
                ui.horizontal(|ui| self.run_controls(ui));
            });
            tab_button(&mut columns[2], Icon::Settings, "Menu", SheetTab::Menu, tab);
        });
    }

    /// Always at the top of the sheet: the guide, and the selected fleet's
    /// contact decision when one is pending.
    pub fn sheet_header(&mut self, ui: &mut egui::Ui) {
        self.unlock_cards(ui);
        self.guide(ui);
        self.contact_panel(ui);
        if self.unlocked(Feature::Victory) {
            self.haven_prompt(ui);
        }
    }

    /// The Fleets page: the ship used for trading, then every fleet.
    pub fn fleets_sheet(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let ships = self.state.ships.clone();
        egui::ComboBox::from_id_salt("active_ship")
            .selected_text(&self.selected_ship().name)
            .width(180.0)
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
        ui.horizontal_wrapped(|ui| {
            let cargo = format!("{}/{}", ship.cargo_used(), ship.cargo_capacity);
            icons::stat(ui, Icon::Cargo, GREEN, &cargo, "Cargo used / capacity");
            let fuel = format!("{}/{}", ship.fuel, ship.fuel_capacity);
            icons::stat(ui, Icon::Fuel, CYAN, &fuel, "Fuel / capacity");
        });
        ui.separator();
        self.map_roster(ui, systems);
    }

    /// The Menu page: the full-screen panels and session tools.
    pub fn menu_sheet(&mut self, ui: &mut egui::Ui) {
        let entries = [
            (Icon::Market, "Trade", "Market, refuel and hold upgrade"),
            (
                Icon::Fleets,
                "Fleets & shipyard",
                "Ships, cargo transfer and shipyard",
            ),
            (
                Icon::Navigation,
                "Navigation",
                "Courses, readiness and contacts",
            ),
            (
                Icon::Star,
                "Victory",
                "Roads to victory, score, the Raider Haven and campaign length",
            ),
            (
                Icon::Save,
                "Session & saves",
                "Save, load, settings, guide and log",
            ),
        ];
        let shown = [
            true,
            self.unlocked(Feature::Fleets),
            true,
            self.unlocked(Feature::Victory),
            true,
        ];
        for (index, (icon, label, tip)) in entries.into_iter().enumerate() {
            if !shown[index] {
                continue;
            }
            let button = ui.add_sized([ui.available_width(), 34.0], |ui: &mut egui::Ui| {
                icons::button(ui, icon, label)
            });
            if button.on_hover_text(tip).clicked() {
                match index {
                    0 => self.market.open = true,
                    1 => self.fleet_panel.open = true,
                    2 => self.open_navigation(),
                    3 => self.victory.open = true,
                    _ => self.saves.open = true,
                }
            }
        }
    }
}
