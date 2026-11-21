use super::Preferences;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacyPreferences {
    schema_version: u32,
    save_path: PathBuf,
    slot: u8,
    speed: u32,
    guide: bool,
    autosave: bool,
}

impl LegacyPreferences {
    pub(super) fn migrate(self) -> Result<Preferences, String> {
        if self.schema_version != 1 || ![1, 2, 4].contains(&self.speed) {
            return Err("Invalid legacy client settings".into());
        }
        Ok(Preferences {
            schema_version: 2,
            save_path: self.save_path,
            slot: self.slot,
            active_ship: game_core::travel::ShipId(1),
            guide: self.guide,
            autosave: self.autosave,
            options: Default::default(),
        })
    }
}
