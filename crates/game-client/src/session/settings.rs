use super::LocalSession;
use crate::preferences::Preferences;
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
}
