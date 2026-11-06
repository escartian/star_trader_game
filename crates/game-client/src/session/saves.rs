//! Explicit save/load controls. Failed loads never replace the live authority.
use super::LocalSession;
use game_protocol::{ClientCommand, ServerMessage};
use std::path::PathBuf;

pub(super) struct SavePanel {
    pub open: bool,
    pub path: PathBuf,
    pub(super) slot: u8,
    seed: u64,
    pub guide: bool,
    pub autosave: bool,
}

impl Default for SavePanel {
    fn default() -> Self {
        Self {
            open: false,
            path: "data/slot-1.json".into(),
            slot: 1,
            seed: 0,
            guide: true,
            autosave: true,
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
        match game_server::Session::load(&self.saves.path) {
            Ok((authority, recovery)) => {
                self.replace_authority(authority);
                self.record(recovery.unwrap_or_else(|| "Saved session restored.".into()));
            }
            Err(error) => self.record(error),
        }
    }

    fn replace_authority(&mut self, authority: game_server::Session) {
        self.navigation = super::navigation::NavigationPanel::default();
        self.authority = authority;
        let message = self.authority.handle(ClientCommand::GetSession);
        self.accept(message);
        if let ServerMessage::GalaxySnapshot { systems, .. } =
            self.authority.handle(ClientCommand::GetGalaxy)
        {
            self.loaded_galaxy = Some(systems);
        }
    }

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
        egui::Window::new("Session and saves")
            .open(&mut open)
            .vscroll(true)
            .show(ctx, |ui| self.controls_ui(ui));
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
        ui.horizontal(|ui| {
            ui.label("New galaxy seed");
            ui.add(egui::DragValue::new(&mut self.saves.seed));
            if ui.button("New session").clicked() {
                match game_server::Session::with_economy(
                    self.saves.seed,
                    self.economy.rules.clone(),
                ).and_then(|mut authority| {
                    authority.set_movement_rules(self.state.movement_rules.clone())?;
                    Ok(authority)
                }) {
                    Ok(authority) => {
                        self.replace_authority(authority);
                        self.record(
                            "New session started. Autosave uses the selected slot; choose another slot to preserve an older run.".into(),
                        );
                        ui.ctx().request_repaint();
                    }
                    Err(error) => self.record(error),
                }
            }
        });
        ui.separator();
        ui.label("Plotting, reading, trading and fleet management take no time. Execute navigation run advances fleets and markets together.");
        ui.checkbox(&mut self.saves.guide, "Show first-delivery guide");
        ui.checkbox(&mut self.saves.autosave, "Autosave on close / suspend");
        ui.label("Save slot, guide and autosave choices are remembered.");
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.separator();
        ui.heading("Session log");
        for message in self.log.iter().rev() {
            ui.label(message);
        }
    }

    fn slot_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Save slot");
            if ui
                .add(egui::DragValue::new(&mut self.saves.slot).range(1..=3))
                .changed()
            {
                self.saves
                    .path
                    .set_file_name(format!("slot-{}.json", self.saves.slot));
            }
        });
        ui.label(self.saves.path.display().to_string());
        ui.label("Selecting a slot does not load it. Autosave writes the current session to the selected slot; the previous valid save becomes its backup.");
        ui.horizontal(|ui| {
            if ui.button("Save current session").clicked() {
                let result = self.save_game();
                self.record(result.map_or_else(
                    |error| format!("Save failed: {error}"),
                    |_| "Session saved.".into(),
                ));
            }
            if ui.button("Continue from slot").clicked() {
                self.load_slot();
                ui.ctx().request_repaint();
            }
        });
        ui.label("Continue replaces the current session with the selected save. Save first to keep current progress.");
    }
}
