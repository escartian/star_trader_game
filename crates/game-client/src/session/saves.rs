//! Explicit save/load controls. Failed loads never replace the live authority.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{CYAN, GREEN};
use game_protocol::{ClientCommand, ServerMessage};
use std::path::Path;
use std::path::PathBuf;

pub(super) struct SavePanel {
    pub open: bool,
    pub path: PathBuf,
    pub(super) slot: u8,
    seed: u64,
    fixed_seed: bool,
    setup: game_server::GameSetup,
    pub guide: bool,
    pub autosave: bool,
    /// Audio and display options (see `session::settings`).
    pub options: crate::preferences::ClientOptions,
    pub settings_open: bool,
    /// The player asked to return to the main menu; the map opens it.
    pub main_menu_requested: bool,
    /// Autosave is off: "Main menu" asks whether to save first.
    pub confirm_leave: bool,
}

impl Default for SavePanel {
    fn default() -> Self {
        Self {
            open: false,
            path: "data/slot-1.json".into(),
            slot: 1,
            seed: 0,
            fixed_seed: false,
            setup: game_server::GameSetup::default(),
            guide: true,
            autosave: true,
            options: Default::default(),
            settings_open: false,
            main_menu_requested: false,
            confirm_leave: false,
        }
    }
}

impl LocalSession {
    pub fn set_save_path(&mut self, path: PathBuf) {
        if let Some(slot) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("slot-"))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|slot| (1..=3).contains(slot))
        {
            self.saves.slot = slot;
        }
        self.saves.path = path;
    }

    pub(super) fn load_slot(&mut self) {
        let path = self.saves.path.clone();
        match self.load_path(&path) {
            Ok(recovery) => {
                self.record(recovery.unwrap_or_else(|| "Saved session restored.".into()))
            }
            Err(error) => self.record(error),
        }
    }

    /// Select and load another game only after its save has been validated.
    pub fn load_path(&mut self, path: &Path) -> Result<Option<String>, String> {
        let (authority, recovery) = game_server::Session::load(path)?;
        self.replace_authority(authority);
        self.set_save_path(path.to_path_buf());
        if let Err(error) = self.persist_preferences() {
            self.record(format!(
                "Game loaded, but settings could not be saved: {error}"
            ));
        }
        Ok(recovery)
    }

    pub fn start_new_at(
        &mut self,
        path: PathBuf,
        seed: u64,
        setup: game_server::GameSetup,
    ) -> Result<(), String> {
        // The current game's rules are the base; the setup overrides its choices.
        let authority = game_server::Session::new_game(
            seed,
            self.economy.rules.clone(),
            self.state.movement_rules.clone(),
            &setup,
        )?;
        self.replace_authority(authority);
        self.set_save_path(path);
        if let Err(error) = self.persist_preferences() {
            self.record(format!(
                "New game started, but settings could not be saved: {error}"
            ));
        }
        Ok(())
    }

    fn replace_authority(&mut self, authority: game_server::Session) {
        // A loaded or new session must never inherit an advance or glide from the old one.
        self.advance = None;
        self.motion = None;
        self.navigation = super::navigation::NavigationPanel::default();
        self.authority = authority;
        let message = self.authority.handle(ClientCommand::GetSession);
        self.accept(message);
        // A loaded game's existing end is not announced again.
        self.victory = super::victory::VictoryPanel::seen(&self.state);
        if let ServerMessage::GalaxySnapshot { systems, .. } =
            self.authority.handle(ClientCommand::GetGalaxy)
        {
            self.loaded_galaxy = Some(systems);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn autosave(&mut self) -> Result<(), String> {
        if !self.saves.autosave {
            if let Err(error) = self.persist_preferences() {
                self.record(format!("Could not save client settings: {error}"));
            }
            return Ok(());
        }
        let result = self.save_game();
        if let Err(error) = &result {
            self.record(format!(
                "Autosave failed: {error}. Retry Save or disable autosave before closing."
            ));
        }
        result
    }

    pub fn show_session_controls(&mut self, ctx: &egui::Context) {
        let previous = self.preferences();
        let mut open = self.saves.open;
        crate::theme::metal::window(ctx, "Session", &mut open, 460.0, |ui| self.controls_ui(ui));
        self.saves.open = open;
        if self.preferences() != previous
            && let Err(error) = self.persist_preferences()
        {
            self.record(format!("Could not save client settings: {error}"));
        }
    }

    fn controls_ui(&mut self, ui: &mut egui::Ui) {
        self.slot_controls(ui);
        ui.separator();
        super::setup::setup_controls(ui, &mut self.saves.setup);
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Seed, CYAN, "Map seed for a new session");
            ui.checkbox(&mut self.saves.fixed_seed, "Use fixed seed");
            ui.add_enabled(
                self.saves.fixed_seed,
                egui::DragValue::new(&mut self.saves.seed),
            )
            .on_hover_text("The same seed and map size make the same map. Leave this off for a new map each time.");
            if icons::button(ui, Icon::New, "New session")
                .on_hover_text("Start a new map. Autosave uses the selected slot.")
                .clicked()
            {
                let seed = if self.saves.fixed_seed {
                    self.saves.seed
                } else {
                    crate::seed::new_seed()
                };
                let setup = self.saves.setup;
                match self.start_new_at(self.saves.path.clone(), seed, setup) {
                    Ok(()) => {
                        self.record(format!(
                            "{} Autosave uses the selected slot; choose another slot to preserve an older run.",
                            super::setup::setup_summary(seed, &setup)
                        ));
                        ui.ctx().request_repaint();
                    }
                    Err(error) => self.record(error),
                }
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Settings, "Settings")
                .on_hover_text("Volume, display, guide and autosave")
                .clicked()
            {
                self.saves.settings_open = true;
            }
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                "Plotting, reading, trading and fleet management take no time.\nOnly navigation runs (Advance or the clock button) move fleets and markets.",
            );
        });
    }

    fn slot_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icons::glyph(
                ui,
                Icon::Save,
                GREEN,
                "Save slot. Selecting a slot does not load it; autosave writes to the selected slot and keeps the previous valid save as a backup.",
            );
            for slot in 1..=3_u8 {
                if ui
                    .selectable_label(self.saves.slot == slot, format!(" {slot} "))
                    .on_hover_text(format!("Slot {slot}"))
                    .clicked()
                {
                    self.saves.slot = slot;
                    self.saves
                        .path
                        .set_file_name(format!("slot-{}.json", self.saves.slot));
                }
            }
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                &self.saves.path.display().to_string(),
            );
            ui.separator();
            if icons::button(ui, Icon::Save, "Save").clicked() {
                let result = self.save_game();
                self.record(result.map_or_else(
                    |error| format!("Save failed: {error}"),
                    |_| "Session saved.".into(),
                ));
            }
            if icons::button(ui, Icon::Load, "Load")
                .on_hover_text("Replace the current session with the selected save. Save first to keep current progress.")
                .clicked()
            {
                self.load_slot();
                ui.ctx().request_repaint();
            }
        });
        if !self.notice.is_empty() {
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Info, CYAN, "Latest event");
                ui.label(&self.notice);
            });
        }
    }
}
