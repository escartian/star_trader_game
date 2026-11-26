//! Versioned authoritative JSON. Loading never regenerates worlds or prices.
mod fleets;
mod legacy_battle;
mod markets;
mod schema;
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
}

impl Session {
    fn save_data(&self) -> SaveData {
        SaveData {
            schema_version: schema::SCHEMA as u32,
            systems: self.systems.clone(),
            player: self.state.clone(),
            economy: self.economy.clone(),
        }
    }

    pub fn export_json(&self) -> Result<String, String> {
        let data = self.save_data();
        data.validate()?;
        serde_json::to_string_pretty(&data).map_err(|error| error.to_string())
    }

    /// The save as compact JSON (no indentation), for size-limited storage
    /// such as a browser's.
    pub fn export_compact_json(&self) -> Result<String, String> {
        let data = self.save_data();
        data.validate()?;
        serde_json::to_string(&data).map_err(|error| error.to_string())
    }

    /// A session from save JSON held in memory (a browser's storage), checked
    /// and upgraded exactly as [`Self::load`] checks a file; there is no
    /// backup to fall back on here, the caller keeps its own.
    pub fn import_json(json: &str) -> Result<(Self, Option<String>), String> {
        let (data, upgraded) = parse(json.as_bytes())?;
        Ok((
            Self {
                systems: data.systems,
                state: data.player,
                economy: data.economy,
            },
            upgraded.map(String::from),
        ))
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

    /// Returns a validated session and an optional notice for the player:
    /// recovery from the backup, or an old battle auto-resolved on load.
    /// Callers replace their current session only after this operation succeeds.
    pub fn load(path: impl AsRef<Path>) -> Result<(Self, Option<String>), String> {
        let path = path.as_ref();
        let ((data, upgraded), recovery) = match read(path) {
            Ok(loaded) => (loaded, None),
            Err(primary) => match read(&backup_path(path)) {
                Ok(loaded) => (
                    loaded,
                    Some(format!(
                        "Recovered previous save from backup. Primary save: {primary}"
                    )),
                ),
                Err(backup) => {
                    return Err(format!("Could not load save: {primary}. Backup: {backup}"));
                }
            },
        };
        let notice = match (recovery, upgraded) {
            (Some(recovery), Some(upgraded)) => Some(format!("{recovery} {upgraded}")),
            (recovery, upgraded) => recovery.or(upgraded.map(String::from)),
        };
        Ok((
            Self {
                systems: data.systems,
                state: data.player,
                economy: data.economy,
            },
            notice,
        ))
    }
}

/// A validated save, with a note when loading it changed something (an old
/// battle auto-resolved).
fn read(path: &Path) -> Result<(SaveData, Option<&'static str>), String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.len() > MAX_SAVE_BYTES {
        return Err("Save exceeds the supported size limit".into());
    }
    let json = fs::read(path).map_err(|error| error.to_string())?;
    parse(&json)
}

/// [`read`] for a save already in memory: upgraded, placed and validated.
fn parse(json: &[u8]) -> Result<(SaveData, Option<&'static str>), String> {
    if json.len() as u64 > MAX_SAVE_BYTES {
        return Err("Save exceeds the supported size limit".into());
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(json).map_err(|error| error.to_string())?;
    let grid_battle = schema::upgrade(&mut value)?;
    let mut data: SaveData = serde_json::from_value(value).map_err(|error| error.to_string())?;
    // Reject malformed orbit definitions before `place` divides by a period
    // or adds a moon offset to its parent.
    data.player
        .movement_rules
        .validate_world(&data.systems)
        .map_err(|error| error.to_string())?;
    // Bodies stand where their orbits put them at the saved tick.
    game_core::orbits::place(&mut data.systems, data.player.tick);
    // Saves from before the Raider Haven get the one a new game would place.
    data.player.place_haven(&data.systems);
    // Saves from before moon and asteroid surfaces get the ones a new game
    // would give them.
    data.economy
        .rules
        .planet_catalog
        .populate_surfaces(&mut data.systems);
    let notice = match grid_battle {
        Some(battle) => {
            legacy_battle::resolve(&mut data, battle)?;
            Some(legacy_battle::NOTICE)
        }
        None => None,
    };
    data.validate()?;
    Ok((data, notice))
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod validation_tests;

#[cfg(test)]
mod market_tests;

#[cfg(test)]
mod navigation_tests;
