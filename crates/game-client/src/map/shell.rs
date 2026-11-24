//! Phone-first shell layout: a status strip at the top holding the title, the
//! actions (Fleets, Advance, Menu) and the numbers, and one context sheet at
//! the bottom. On a window wider than tall the sheet docks on the right
//! instead, giving the map the full height. A narrow strip puts the actions on
//! a second row. The map fills whatever is left, with camera buttons at its corner.
use super::{Map, camera::Camera, toolbar::CameraAction};
use crate::session::SheetTab;
use crate::theme::icons::{self, Icon};
use egui::{Color32, RichText};

/// From this width-to-height ratio, the sheet docks on the right.
const WIDE: f32 = 1.15;
const SIDE_WIDTH: f32 = 360.0;
/// Share of the screen height the bottom sheet takes.
const SHEET_SHARE: f32 = 0.38;
/// Below this strip width (points) the actions take a second row.
const ONE_ROW_STRIP: f32 = 720.0;

fn frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Color32::from_rgb(14, 20, 24))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(40, 56, 60)))
}

impl Map {
    /// Lays out the strip and (unless closed) the sheet; must run before the
    /// map's central panel so the map gets the remaining space.
    pub(super) fn shell(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("status-strip")
            .frame(frame())
            .show(ctx, |ui| self.status_strip(ui));
        self.reopen_card_on_change();
        if !self.card.open {
            self.card.map_area = ctx.available_rect();
            return;
        }
        let screen = ctx.screen_rect();
        let sheet = |ui: &mut egui::Ui, map: &mut Map| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| map.sheet_ui(ui));
        };
        if screen.width() >= screen.height() * WIDE {
            egui::SidePanel::right("context-sheet")
                .resizable(false)
                .exact_width(SIDE_WIDTH)
                .frame(frame())
                .show(ctx, |ui| sheet(ui, self));
        } else {
            egui::TopBottomPanel::bottom("context-sheet")
                .resizable(false)
                .exact_height(screen.height() * SHEET_SHARE)
                .frame(frame())
                .show(ctx, |ui| sheet(ui, self));
        }
        self.card.map_area = ctx.available_rect();
    }

    fn status_strip(&mut self, ui: &mut egui::Ui) {
        let one_row = ui.available_width() >= ONE_ROW_STRIP;
        ui.horizontal(|ui| {
            self.strip_title(ui);
            if one_row {
                ui.add_space(12.0);
                self.session
                    .strip_actions(ui, &mut self.sheet, &mut self.card.open);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.session.status_numbers(ui);
            });
        });
        if !one_row {
            ui.horizontal(|ui| {
                self.session
                    .strip_actions(ui, &mut self.sheet, &mut self.card.open)
            });
        }
    }

    fn strip_title(&mut self, ui: &mut egui::Ui) {
        if self.local.is_some()
            && icons::icon_button(ui, Icon::Back, "Back to the galaxy (Esc)").clicked()
        {
            self.local = None;
        }
        let title = self.local.as_ref().map_or_else(
            || "GALAXY".to_owned(),
            |local| local.system().name.to_uppercase(),
        );
        ui.label(
            RichText::new(title)
                .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                .color(crate::theme::GREEN),
        );
    }

    /// The Songbook opened from the in-game Menu page.
    pub(super) fn show_game_songbook(&mut self, ctx: &egui::Context) {
        let (open, view) = &mut self.songbook;
        let request = super::songbook::show(
            ctx,
            open,
            view,
            self.music.current_block(),
            self.music.next_block(),
        );
        match request {
            Some(super::songbook::Request::Play(part)) => self.music.select_part(part),
            Some(super::songbook::Request::Queue(part)) => self.music.queue_part(part),
            None => return,
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    /// The card's Close button: hides the whole card for the most map room.
    /// It reopens on the strip's card button, a tab, a tap on the map or a
    /// contact.
    fn card_close(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if icons::button(ui, Icon::Cancel, "Close")
                .on_hover_text("Hide this card; the map takes its room")
                .clicked()
            {
                self.close_card();
            }
        });
    }

    fn sheet_ui(&mut self, ui: &mut egui::Ui) {
        self.card_close(ui);
        self.session.sheet_header(ui);
        match self.sheet {
            SheetTab::Selection => {
                if let Some(local) = &mut self.local {
                    local.selection_sheet(ui);
                } else {
                    let rect = self.last_map_rect;
                    self.details(ui, rect);
                    self.session.enter_button(ui, self.selected);
                }
                ui.separator();
                self.session.destination_card(ui, &self.session_systems);
            }
            SheetTab::Fleets => self.session.fleets_sheet(ui, &self.session_systems),
            SheetTab::Menu => {
                if ui
                    .add_sized([ui.available_width(), 34.0], |ui: &mut egui::Ui| {
                        icons::button(ui, Icon::Log, "Songbook · Music")
                    })
                    .on_hover_text("Every part of the soundtrack; play or queue one")
                    .clicked()
                {
                    self.songbook.0 = true;
                }
                self.session.menu_sheet(ui);
                #[cfg(any(test, feature = "dev-tools"))]
                self.test_scenes(ui);
            }
        }
    }

    /// Reset, focus selection and focus fleet at the galaxy map's corner.
    pub(super) fn galaxy_buttons(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let action = super::toolbar::map_buttons(
            ctx,
            rect,
            "galaxy-map-buttons",
            &[
                CameraAction::Reset,
                CameraAction::FocusSelection,
                CameraAction::FocusFleet,
            ],
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
    }
}
