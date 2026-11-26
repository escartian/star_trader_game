//! Client preferences are separate from authoritative world/player saves.
mod legacy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io::Write, path::Path};

pub const DEFAULT_PATH: &str = "data/settings.json";
/// Largest settings file accepted, in bytes.
const MAX_BYTES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub save_path: PathBuf,
    pub slot: u8,
    #[serde(default = "default_active_ship")]
    pub active_ship: game_core::travel::ShipId,
    pub guide: bool,
    pub autosave: bool,
    /// Audio and display options; files from before they existed get defaults.
    #[serde(default)]
    pub options: ClientOptions,
}

/// Audio and display options from the Settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClientOptions {
    /// Percent, 0-100 (see `audio::music_gain` for the loudness curve).
    pub master_volume: u8,
    /// Percent, 0-100, applied on top of the master volume.
    pub music_volume: u8,
    /// Percent, 0-100, applied on top of the master volume.
    pub effects_volume: u8,
    pub muted: bool,
    /// Keep the theme playing after leaving the main menu.
    pub music_in_game: bool,
    /// Borderless fullscreen on desktop.
    pub fullscreen: bool,
    /// Interface scale in percent, `UI_SCALE_MIN..=UI_SCALE_MAX`.
    pub ui_scale: u8,
    /// Map glides and battle slides; off shows final positions at once.
    pub animations: bool,
    /// How long each navigation run takes to play out on the map.
    pub run_pace: RunPace,
    /// How market orders are split among the fleet's ships at a port.
    pub load_order: game_core::fleet_trade::LoadOrder,
    /// Both maps: draw NPC traders, police patrols and raider bands (the
    /// selected or pursued one stays visible either way).
    pub show_traffic: bool,
    /// Galaxy map: draw faction territory.
    pub show_territory: bool,
}

/// A map layer the player can switch on and off from the map's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapLayer {
    Traffic,
    Territory,
}

/// Seconds per navigation run on the map: the ships' glide and the pause
/// between an advance's runs. Display only; it never changes the rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunPace {
    Relaxed,
    #[default]
    Normal,
    Quick,
    Rapid,
}

impl RunPace {
    pub const ALL: [Self; 4] = [Self::Relaxed, Self::Normal, Self::Quick, Self::Rapid];

    pub const fn seconds(self) -> f64 {
        match self {
            Self::Relaxed => 2.0,
            Self::Normal => 1.2,
            Self::Quick => 0.6,
            Self::Rapid => 0.35,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Relaxed => "Relaxed (2 s per run)",
            Self::Normal => "Normal (1.2 s per run)",
            Self::Quick => "Quick (0.6 s per run)",
            Self::Rapid => "Rapid (0.35 s per run)",
        }
    }
}

pub const UI_SCALE_MIN: u8 = 75;
pub const UI_SCALE_MAX: u8 = 200;
/// The Settings slider moves in these steps; saved values need not match.
pub const UI_SCALE_STEP: u8 = 25;

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            master_volume: 80,
            music_volume: 70,
            effects_volume: 80,
            muted: false,
            music_in_game: false,
            fullscreen: false,
            ui_scale: 100,
            animations: true,
            run_pace: RunPace::default(),
            load_order: Default::default(),
            show_traffic: true,
            show_territory: true,
        }
    }
}

impl ClientOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.master_volume > 100 || self.music_volume > 100 || self.effects_volume > 100 {
            return Err("Volumes must be 0-100 percent".into());
        }
        if !(UI_SCALE_MIN..=UI_SCALE_MAX).contains(&self.ui_scale) {
            return Err(format!(
                "Interface scale must be {UI_SCALE_MIN}-{UI_SCALE_MAX} percent"
            ));
        }
        Ok(())
    }
}

fn default_active_ship() -> game_core::travel::ShipId {
    game_core::travel::ShipId(1)
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 2,
            save_path: "data/slot-1.json".into(),
            slot: 1,
            active_ship: default_active_ship(),
            guide: true,
            autosave: true,
            options: ClientOptions::default(),
        }
    }
}

impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 {
            return Err("Unsupported client settings version".into());
        }
        if self.active_ship.0 == 0 || !(1..=3).contains(&self.slot) {
            return Err("Invalid client save slot".into());
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
        self.options.validate()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &Path) -> Result<Self, String> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.to_string()),
        };
        if metadata.len() > MAX_BYTES as u64 {
            return Err("Client settings file is too large".into());
        }
        let json = fs::read(path).map_err(|error| error.to_string())?;
        Self::from_json(&json)
    }

    /// Settings from JSON (a file's, or a browser storage entry's), migrated
    /// from older versions and validated.
    pub fn from_json(json: &[u8]) -> Result<Self, String> {
        if json.len() > MAX_BYTES {
            return Err("Client settings file is too large".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(json).map_err(|error| error.to_string())?;
        let settings: Self = match value
            .get("schema_version")
            .and_then(|version| version.as_u64())
        {
            Some(1) => serde_json::from_value::<legacy::LegacyPreferences>(value)
                .map_err(|error| error.to_string())?
                .migrate()?,
            Some(2) => serde_json::from_value(value).map_err(|error| error.to_string())?,
            _ => return Err("Unsupported client settings version".into()),
        };
        settings.validate()?;
        Ok(settings)
    }

    /// The settings as pretty JSON, validated first.
    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = self.to_json()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        temporary
            .write_all(json.as_bytes())
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
