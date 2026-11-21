//! Client preferences: applying and persisting them, and the Settings page
//! (audio, display, gameplay) shown in game and on the main menu.
use super::LocalSession;
use crate::preferences::{ClientOptions, Preferences, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN};
use egui::RichText;
use std::path::PathBuf;

impl LocalSession {
    pub fn apply_preferences(
        &mut self,
        settings: Preferences,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        settings.validate()?;
        self.select_ship(settings.active_ship);
        self.saves.slot = settings.slot;
        self.set_save_path(settings.save_path);
        self.saves.guide = settings.guide;
        self.saves.autosave = settings.autosave;
        self.saves.options = settings.options;
        self.preferences_path = path;
        Ok(())
    }

    pub(super) fn preferences(&self) -> Preferences {
        Preferences {
            schema_version: 2,
            save_path: self.saves.path.clone(),
            slot: self.saves.slot,
            active_ship: self.selected_ship().id,
            guide: self.saves.guide,
            autosave: self.saves.autosave,
            options: self.saves.options,
        }
    }

    pub(super) fn persist_preferences(&self) -> Result<(), String> {
        if let Some(path) = &self.preferences_path {
            self.preferences().save(path)?;
        }
        Ok(())
    }

    pub(super) fn save_game(&mut self) -> Result<(), String> {
        self.authority.save(&self.saves.path)?;
        self.persist_preferences()
            .map_err(|error| format!("Game saved, but client settings could not be saved: {error}"))
    }

    /// Audio and display options as last chosen.
    pub fn client_options(&self) -> ClientOptions {
        self.saves.options
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_path(&self) -> PathBuf {
        self.saves.path.clone()
    }

    pub fn open_settings(&mut self) {
        self.saves.settings_open = true;
    }

    /// True once after the player chose "Main menu" (and the game was saved,
    /// or they chose to leave without saving).
    pub fn take_main_menu_request(&mut self) -> bool {
        std::mem::take(&mut self.saves.main_menu_requested)
    }

    /// The in-game Settings window.
    pub fn show_settings(&mut self, ctx: &egui::Context) {
        let mut open = self.saves.settings_open;
        crate::theme::metal::window(ctx, "Settings", &mut open, 420.0, |ui| {
            self.settings_controls(ui);
        });
        self.saves.settings_open = open;
    }

    /// Every setting, saved as soon as it changes. Shared with the main menu.
    pub fn settings_controls(&mut self, ui: &mut egui::Ui) {
        let previous = self.preferences();
        options_controls(ui, &mut self.saves.options);
        section(ui, Icon::Settings, "Gameplay");
        ui.checkbox(&mut self.saves.guide, "Guide")
            .on_hover_text("Show the first-delivery guide");
        ui.checkbox(&mut self.saves.autosave, "Autosave")
            .on_hover_text("Save the selected slot on close, suspend and Main menu");
        if self.preferences() != previous
            && let Err(error) = self.persist_preferences()
        {
            self.record(format!("Could not save client settings: {error}"));
        }
    }

    /// "Main menu" from the in-game menu: autosave first, or ask when it is off.
    pub(super) fn main_menu_controls(&mut self, ui: &mut egui::Ui) {
        if !self.saves.confirm_leave {
            let button = ui.add_sized([ui.available_width(), 34.0], |ui: &mut egui::Ui| {
                icons::button(ui, Icon::Back, "Main menu")
            });
            if button
                .on_hover_text("Save and return to the title screen")
                .clicked()
            {
                if self.saves.autosave {
                    self.leave_to_main_menu(true);
                } else {
                    self.saves.confirm_leave = true;
                }
            }
            return;
        }
        ui.label(RichText::new("Autosave is off. Save before leaving?").color(AMBER));
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Save, "Save and leave").clicked() {
                self.leave_to_main_menu(true);
            }
            if icons::button(ui, Icon::Back, "Leave without saving").clicked() {
                self.leave_to_main_menu(false);
            }
            if icons::button(ui, Icon::Cancel, "Stay").clicked() {
                self.saves.confirm_leave = false;
            }
        });
    }

    /// Returns to the main menu, saving first when `save` (a failed save keeps
    /// the game open and says why).
    pub fn leave_to_main_menu(&mut self, save: bool) {
        self.saves.confirm_leave = false;
        if save && let Err(error) = self.save_game() {
            self.record(format!("Save failed, so the game stays open: {error}"));
            return;
        }
        self.saves.main_menu_requested = true;
    }
}

fn section(ui: &mut egui::Ui, icon: Icon, title: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        icons::glyph(ui, icon, CYAN, title);
        ui.label(
            RichText::new(title.to_uppercase())
                .font(crate::theme::fonts::display(ui.ctx(), 13.0))
                .color(CYAN),
        );
    });
}

fn percent_slider(ui: &mut egui::Ui, value: &mut u8, label: &str, enabled: bool) {
    ui.label(label);
    ui.add_enabled(
        enabled,
        egui::Slider::new(value, 0..=100)
            .suffix("%")
            .trailing_fill(true),
    );
    ui.end_row();
}

/// Audio and display controls.
fn options_controls(ui: &mut egui::Ui, options: &mut ClientOptions) {
    section(ui, Icon::Info, "Audio");
    let audible = !options.muted;
    egui::Grid::new("audio-volumes").show(ui, |ui| {
        percent_slider(ui, &mut options.master_volume, "Master volume", audible);
        percent_slider(ui, &mut options.music_volume, "Music", audible);
    });
    ui.checkbox(&mut options.muted, "Mute all sound");
    ui.checkbox(&mut options.music_in_game, "Play music during the game")
        .on_hover_text("Off: the theme plays on the main menu only");
    if cfg!(target_arch = "wasm32") {
        ui.small("The browser build has no audio yet.");
    }
    section(ui, Icon::Overview, "Display");
    if !cfg!(target_arch = "wasm32") {
        ui.checkbox(&mut options.fullscreen, "Fullscreen");
    }
    egui::Grid::new("display-scale").show(ui, |ui| {
        ui.label("Interface scale");
        ui.add(
            egui::Slider::new(&mut options.ui_scale, UI_SCALE_MIN..=UI_SCALE_MAX)
                .step_by(5.0)
                .suffix("%")
                .trailing_fill(true),
        );
    });
    ui.checkbox(&mut options.animations, "Animations")
        .on_hover_text("Fleet glides and battle moves. Off shows results at once.");
}
