//! The main menu: a title page (Continue, New game, Load game, Settings, Quit)
//! and its sub-pages, over an animated starfield. Save slot state and checks
//! live in `recovery`; this module only lays out pages and reports choices.
use super::Map;
use super::recovery::{SaveEntry, SaveRecovery, SaveStatus};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{RichText, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuPage {
    #[default]
    Title,
    NewGame,
    Load,
    Settings,
}

/// What the player picked on a page; the map carries it out after drawing.
pub(super) enum Choice {
    Load(PathBuf),
    StartNew(PathBuf),
    Quit,
}

/// Widest the menu column gets, in points.
const COLUMN_WIDTH: f32 = 440.0;
const BUTTON_HEIGHT: f32 = 42.0;

impl Map {
    pub(super) fn show_main_menu(&mut self, ctx: &egui::Context) {
        let mut menu = self.save_recovery.take().expect("main menu is open");
        let animate = ctx.style().animation_time > 0.0;
        let time = if animate {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
            ctx.input(|input| input.time)
        } else {
            super::menu_backdrop::STILL_TIME
        };
        let art = super::art::Artwork::preload(ctx);
        if menu.page != MenuPage::Title && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            menu.page = MenuPage::Title;
        }
        let mut choice = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                super::menu_backdrop::paint(ui.painter(), ui.max_rect(), time, &art);
                let width = (ui.available_width() - 32.0).min(COLUMN_WIDTH);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let side = ((ui.available_width() - width) / 2.0).max(0.0);
                    ui.horizontal(|ui| {
                        ui.add_space(side);
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            choice = match menu.page {
                                MenuPage::Title => title_page(ui, &mut menu),
                                MenuPage::NewGame => new_game_page(ui, &mut menu),
                                MenuPage::Load => load_page(ui, &mut menu),
                                MenuPage::Settings => {
                                    if page_header(ui, "Settings") {
                                        menu.page = MenuPage::Title;
                                    }
                                    crate::theme::metal::frame(ui, 235, |ui| {
                                        ui.set_width(ui.available_width());
                                        self.session.settings_controls(ui);
                                    });
                                    None
                                }
                            };
                        });
                    });
                });
            });
        if self.carry_out(ctx, &mut menu, choice) {
            return;
        }
        self.save_recovery = Some(menu);
    }

    /// Returns true when the menu closes (a game was loaded or started).
    pub(super) fn carry_out(
        &mut self,
        ctx: &egui::Context,
        menu: &mut SaveRecovery,
        choice: Option<Choice>,
    ) -> bool {
        match choice {
            None => false,
            Some(Choice::Quit) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                false
            }
            Some(Choice::Load(path)) => match self.session.load_path(&path) {
                Ok(message) => {
                    self.notify(message.unwrap_or_else(|| "Saved game loaded.".into()));
                    true
                }
                Err(error) => {
                    menu.message = Some(error);
                    menu.refresh();
                    false
                }
            },
            Some(Choice::StartNew(path)) => {
                let seed = crate::seed::new_seed();
                match self.session.start_new_at(path, seed, menu.setup) {
                    Ok(()) => {
                        let summary = crate::session::setup_summary(seed, &menu.setup);
                        self.notify(format!("{summary} This slot will save on close."));
                        true
                    }
                    Err(error) => {
                        menu.message = Some(error);
                        false
                    }
                }
            }
        }
    }
}

fn wide_button(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    icons::menu_button(ui, icon, label, BUTTON_HEIGHT)
}

/// Back button and page title; true when Back was clicked.
fn page_header(ui: &mut egui::Ui, title: &str) -> bool {
    ui.add_space(24.0);
    let back = ui
        .horizontal(|ui| {
            let back = icons::button(ui, Icon::Back, "Back").clicked();
            ui.label(
                RichText::new(title.to_uppercase())
                    .font(crate::theme::fonts::display(ui.ctx(), 20.0))
                    .color(GREEN),
            );
            back
        })
        .inner;
    ui.add_space(8.0);
    back
}

fn message(ui: &mut egui::Ui, menu: &SaveRecovery) {
    if let Some(message) = &menu.message {
        ui.add_space(6.0);
        ui.label(RichText::new(message).color(AMBER));
    }
}

fn describe(entry: &SaveEntry) -> String {
    match &entry.status {
        SaveStatus::Ready { run, ships } => {
            let plural = if *ships == 1 { "" } else { "s" };
            format!("Run {run} · {ships} ship{plural}")
        }
        SaveStatus::Empty => "Empty slot".into(),
        SaveStatus::Incompatible(_) => "Cannot load this save".into(),
    }
}

fn title_page(ui: &mut egui::Ui, menu: &mut SaveRecovery) -> Option<Choice> {
    let mut choice = None;
    ui.add_space((ui.ctx().screen_rect().height() * 0.12).clamp(16.0, 120.0));
    ui.vertical_centered(|ui| {
        if let Some(icon) = crate::app_icon::texture(ui.ctx()) {
            ui.add(egui::Image::new((icon.id(), Vec2::splat(96.0))));
        }
        ui.label(
            RichText::new("STAR TRADER")
                .font(crate::theme::fonts::display(ui.ctx(), 38.0))
                .color(GREEN),
        );
        ui.label(RichText::new("Haul cargo · Outrun raiders · Build a fleet").color(CYAN));
    });
    ui.add_space(18.0);
    if let Some(error) = &menu.startup_error {
        crate::theme::metal::frame(ui, 235, |ui| {
            ui.label(
                RichText::new("The last used save could not be loaded. Your files are intact.")
                    .color(AMBER),
            );
            ui.small(error);
        });
    }
    crate::theme::metal::frame(ui, 235, |ui| {
        ui.set_width(ui.available_width());
        let last = menu
            .entries
            .iter()
            .find(|entry| entry.path == menu.selected)
            .filter(|entry| matches!(entry.status, SaveStatus::Ready { .. }));
        if let Some(entry) = last {
            let label = format!("Continue · {} · {}", entry.label, describe(entry));
            if wide_button(ui, Icon::Enter, &label)
                .on_hover_text("Load the last game you played")
                .clicked()
            {
                choice = Some(Choice::Load(entry.path.clone()));
            }
        }
        for (icon, label, page) in [
            (Icon::New, "New game", MenuPage::NewGame),
            (Icon::Load, "Load game", MenuPage::Load),
            (Icon::Settings, "Settings", MenuPage::Settings),
        ] {
            if wide_button(ui, icon, label).clicked() {
                menu.page = page;
                menu.message = None;
                menu.pending_delete = None;
                menu.pending_overwrite = None;
            }
        }
        if wide_button(ui, Icon::Cancel, "Quit").clicked() {
            choice = Some(Choice::Quit);
        }
    });
    message(ui, menu);
    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        ui.small(format!(
            "v{} · offline prototype · theme “Long Haul”",
            env!("CARGO_PKG_VERSION")
        ));
    });
    choice
}

fn new_game_page(ui: &mut egui::Ui, menu: &mut SaveRecovery) -> Option<Choice> {
    if page_header(ui, "New game") {
        menu.page = MenuPage::Title;
    }
    let mut choice = None;
    crate::theme::metal::frame(ui, 235, |ui| {
        ui.set_width(ui.available_width());
        crate::session::setup_controls(ui, &mut menu.setup);
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Choose a slot").color(CYAN));
    for entry in menu.entries.clone() {
        crate::theme::metal::frame(ui, 235, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.strong(&entry.label);
                ui.label(describe(&entry));
            });
            match &entry.status {
                SaveStatus::Empty => {
                    if icons::button(ui, Icon::New, "Start here").clicked() {
                        // Another window may have saved here since the menu opened.
                        if matches!(SaveStatus::inspect(&entry.path), SaveStatus::Empty) {
                            choice = Some(Choice::StartNew(entry.path.clone()));
                        } else {
                            menu.refresh();
                            menu.message = Some("This slot changed. Choose it again.".into());
                        }
                    }
                }
                SaveStatus::Ready { .. }
                    if menu.pending_overwrite.as_ref() == Some(&entry.path) =>
                {
                    ui.label(RichText::new("Its current game will be overwritten.").color(AMBER));
                    ui.horizontal(|ui| {
                        if icons::button(ui, Icon::Confirm, "Start over here").clicked() {
                            choice = Some(Choice::StartNew(entry.path.clone()));
                        }
                        if icons::button(ui, Icon::Cancel, "Cancel").clicked() {
                            menu.pending_overwrite = None;
                        }
                    });
                }
                SaveStatus::Ready { .. } => {
                    if icons::button(ui, Icon::Reset, "Start over here…").clicked() {
                        menu.pending_overwrite = Some(entry.path.clone());
                    }
                }
                SaveStatus::Incompatible(_) => {
                    ui.small("Delete it from Load game first.");
                }
            }
        });
    }
    message(ui, menu);
    choice
}

fn load_page(ui: &mut egui::Ui, menu: &mut SaveRecovery) -> Option<Choice> {
    if page_header(ui, "Load game") {
        menu.page = MenuPage::Title;
    }
    let mut choice = None;
    for entry in menu.entries.clone() {
        crate::theme::metal::frame(ui, 235, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.strong(&entry.label);
                if entry.path == menu.selected {
                    ui.label(RichText::new("Last used").color(CYAN));
                }
            });
            ui.small(entry.path.display().to_string());
            ui.label(describe(&entry));
            match &entry.status {
                SaveStatus::Ready { .. } => {
                    if icons::button(ui, Icon::Load, "Load").clicked() {
                        choice = Some(Choice::Load(entry.path.clone()));
                    }
                }
                SaveStatus::Empty => {}
                SaveStatus::Incompatible(error) => {
                    ui.small(error);
                    if menu.pending_delete.as_ref() == Some(&entry.path) {
                        ui.label("Delete this save and its backup permanently?");
                        ui.horizontal(|ui| {
                            if icons::button(ui, Icon::Confirm, "Confirm delete").clicked() {
                                menu.message = Some(match menu.delete(&entry.path) {
                                    Ok(()) => "Save and backup deleted. You can start a new game in this slot.".into(),
                                    Err(error) => error,
                                });
                            }
                            if icons::button(ui, Icon::Cancel, "Cancel").clicked() {
                                menu.pending_delete = None;
                            }
                        });
                    } else if icons::button(ui, Icon::Cancel, "Delete save and backup…").clicked()
                    {
                        menu.pending_delete = Some(entry.path.clone());
                    }
                }
            }
        });
    }
    message(ui, menu);
    choice
}

#[cfg(test)]
#[path = "main_menu_tests.rs"]
mod tests;
