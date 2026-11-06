//! Versioned authoritative JSON. Loading never regenerates worlds or prices.
mod fleets;
mod markets;
mod migration;
mod planet_migration;
mod validation;
use crate::Session;
use game_core::{StarSystem, economy::Economy, travel::TravelState};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

const MAX_SAVE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveData {
    schema_version: u32,
    systems: Vec<StarSystem>,
    player: TravelState,
    economy: Economy,
    #[serde(skip)]
    migrated: bool,
}

impl Session {
    fn save_data(&self) -> SaveData {
        SaveData {
            schema_version: 4,
            systems: self.systems.clone(),
            player: self.state.clone(),
            economy: self.economy.clone(),
            migrated: false,
        }
    }

    pub fn export_json(&self) -> Result<String, String> {
        let data = self.save_data();
        data.validate()?;
        serde_json::to_string_pretty(&data).map_err(|error| error.to_string())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let path = path.as_ref();
        let data = self.save_data();
        data.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|error| format!("Create save directory: {error}"))?;
        let mut temporary = NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(temporary.as_file_mut(), &data)
            .map_err(|error| error.to_string())?;
        temporary
            .write_all(b"\n")
            .map_err(|error| error.to_string())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        // Never replace a good backup with a damaged primary file.
        if read(path).is_ok() {
            let mut backup = NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
            let mut old = fs::File::open(path).map_err(|error| error.to_string())?;
            std::io::copy(&mut old, backup.as_file_mut()).map_err(|error| error.to_string())?;
            backup
                .as_file()
                .sync_all()
                .map_err(|error| error.to_string())?;
            backup
                .persist(backup_path(path))
                .map_err(|error| error.to_string())?;
        }
        temporary
            .persist(path)
            .map_err(|error| format!("Replace save: {error}"))?;
        #[cfg(unix)]
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("Sync save directory: {error}"))?;
        Ok(())
    }

    /// Returns a validated session and an optional backup-recovery explanation.
    /// Callers replace their current session only after this operation succeeds.
    pub fn load(path: impl AsRef<Path>) -> Result<(Self, Option<String>), String> {
        let path = path.as_ref();
        let (data, recovery) = match read(path) {
            Ok(data) => (data, None),
            Err(primary) => match read(&backup_path(path)) {
                Ok(data) => (
                    data,
                    Some(format!(
                        "Recovered previous save from backup. Primary save: {primary}"
                    )),
                ),
                Err(backup) => {
                    return Err(format!("Could not load save: {primary}. Backup: {backup}"));
                }
            },
        };
        let recovery = if data.migrated {
            Some(format!(
                "{}Upgraded older save to planetary markets; player assets and station stock preserved. New market treasuries and shipyards start empty.",
                recovery.map_or(String::new(), |message| format!("{message} "))
            ))
        } else {
            recovery
        };
        Ok((
            Self {
                systems: data.systems,
                state: data.player,
                economy: data.economy,
            },
            recovery,
        ))
    }
}

fn read(path: &Path) -> Result<SaveData, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.len() > MAX_SAVE_BYTES {
        return Err("Save exceeds the supported size limit".into());
    }
    let json = fs::read(path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&json).map_err(|error| error.to_string())?;
    let (value, migrated) = migration::upgrade(value)?;
    let mut data: SaveData = serde_json::from_value(value).map_err(|error| error.to_string())?;
    data.migrated = migrated;
    data.validate()?;
    Ok(data)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod migration_tests;

#[cfg(test)]
mod market_tests;
