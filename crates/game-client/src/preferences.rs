//! Client preferences are separate from authoritative world/player saves.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const DEFAULT_PATH: &str = "data/settings.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub save_path: PathBuf,
    pub slot: u8,
    pub speed: u32,
    pub guide: bool,
    pub autosave: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            save_path: "data/slot-1.json".into(),
            slot: 1,
            speed: 1,
            guide: true,
            autosave: true,
        }
    }
}

impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported client settings version".into());
        }
        if !(1..=3).contains(&self.slot) || ![1, 2, 4].contains(&self.speed) {
            return Err("Invalid client save slot or tick speed".into());
        }
        if self.save_path.file_name().is_none() || self.save_path.to_string_lossy().contains('\0') {
            return Err("Invalid save filename in client settings".into());
        }
        if self
            .save_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("settings.json"))
        {
            return Err("settings.json is reserved for client preferences".into());
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.to_string()),
        };
        if metadata.len() > 65_536 {
            return Err("Client settings file is too large".into());
        }
        let json = fs::read(path).map_err(|error| error.to_string())?;
        let settings: Self = serde_json::from_slice(&json).map_err(|error| error.to_string())?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(temporary.as_file_mut(), self)
            .map_err(|error| error.to_string())?;
        temporary
            .write_all(b"\n")
            .map_err(|error| error.to_string())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        temporary.persist(path).map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
