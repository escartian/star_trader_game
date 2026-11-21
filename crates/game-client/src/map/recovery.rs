//! The main menu's save slots: what each holds, and deleting unloadable saves.
//! The menu keeps every session idle until the player picks a game; its pages
//! are drawn by `main_menu`.
use super::Map;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(super) struct SaveEntry {
    pub path: PathBuf,
    pub label: String,
    pub status: SaveStatus,
}

#[derive(Clone)]
pub(super) enum SaveStatus {
    Empty,
    Ready { run: u64, ships: usize },
    Incompatible(String),
}

impl SaveStatus {
    pub(super) fn inspect(path: &Path) -> Self {
        let backup = backup_path(path);
        if !path.exists() && !backup.exists() {
            return Self::Empty;
        }
        match game_server::Session::load(path) {
            Ok((mut session, _)) => {
                let game_protocol::ServerMessage::SessionSnapshot { state, .. } =
                    session.handle(game_protocol::ClientCommand::GetSession)
                else {
                    unreachable!("a loaded session returns its own snapshot")
                };
                Self::Ready {
                    run: state.navigation.run,
                    ships: state.ships.len(),
                }
            }
            Err(error) => Self::Incompatible(error),
        }
    }
}

pub(super) struct SaveRecovery {
    /// The last used save: Continue loads it.
    pub selected: PathBuf,
    pub startup_error: Option<String>,
    pub entries: Vec<SaveEntry>,
    /// A save awaiting "Confirm delete" (unloadable) or "Confirm" overwrite.
    pub pending_delete: Option<PathBuf>,
    pub pending_overwrite: Option<PathBuf>,
    pub message: Option<String>,
    pub setup: game_server::GameSetup,
    pub page: super::main_menu::MenuPage,
}

impl SaveRecovery {
    #[cfg(not(target_arch = "wasm32"))]
    fn new(selected: PathBuf, error: Option<String>) -> Self {
        let directory = selected
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut paths: Vec<(PathBuf, String)> = (1..=3)
            .map(|number| {
                (
                    directory.join(format!("slot-{number}.json")),
                    format!("Slot {number}"),
                )
            })
            .collect();
        if !paths.iter().any(|(path, _)| *path == selected) {
            paths.insert(0, (selected.clone(), "Last used game".into()));
        }
        let entries = paths
            .into_iter()
            .map(|(path, label)| SaveEntry {
                status: SaveStatus::inspect(&path),
                path,
                label,
            })
            .collect();
        Self {
            selected,
            startup_error: error,
            entries,
            pending_delete: None,
            pending_overwrite: None,
            message: None,
            setup: game_server::GameSetup::default(),
            page: Default::default(),
        }
    }

    pub(super) fn refresh(&mut self) {
        for entry in &mut self.entries {
            entry.status = SaveStatus::inspect(&entry.path);
        }
    }

    pub(super) fn delete(&mut self, path: &Path) -> Result<(), String> {
        // The confirmation UI names this exact path and its backup. Remove the
        // backup first, so a failure leaves the primary intact when possible.
        for file in [backup_path(path), path.to_path_buf()] {
            match fs::remove_file(&file) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(format!("Could not delete {}: {error}", file.display())),
            }
        }
        self.pending_delete = None;
        self.refresh();
        Ok(())
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}

impl Map {
    /// Present on normal desktop launch. No save is modified until a choice.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_save_chooser(&mut self, selected: PathBuf, error: Option<String>) {
        self.save_recovery = Some(SaveRecovery::new(selected, error));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incompatible_save_stays_intact_until_explicit_deletion() {
        let directory = tempfile::tempdir().unwrap();
        let old = directory.path().join("slot-1.json");
        let backup = backup_path(&old);
        fs::write(&old, r#"{"schema_version":6}"#).unwrap();
        fs::write(&backup, r#"{"schema_version":6}"#).unwrap();
        let valid = directory.path().join("slot-2.json");
        game_server::Session::default().save(&valid).unwrap();
        let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
        map.open_save_chooser(old.clone(), Some("Old schema".into()));
        assert!(map.autosave().is_ok());
        assert_eq!(fs::read_to_string(&old).unwrap(), r#"{"schema_version":6}"#);
        let chooser = map.save_recovery.as_ref().unwrap();
        assert!(matches!(
            chooser.entries[0].status,
            SaveStatus::Incompatible(_)
        ));
        assert!(matches!(
            chooser.entries[1].status,
            SaveStatus::Ready { .. }
        ));
        assert!(matches!(chooser.entries[2].status, SaveStatus::Empty));
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(390.0, 844.0),
                )),
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        );
        assert!(
            !ctx.tessellate(output.shapes, output.pixels_per_point)
                .is_empty()
        );
        assert!(map.save_recovery.is_some());
        let mut chooser = map.save_recovery.take().unwrap();
        chooser.delete(&old).unwrap();
        assert!(!old.exists());
        assert!(!backup.exists());
        assert!(matches!(chooser.entries[0].status, SaveStatus::Empty));
        assert!(valid.exists());
    }
}
