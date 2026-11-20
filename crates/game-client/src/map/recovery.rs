//! The startup save chooser keeps every session idle until the player picks a game.
use super::Map;
use game_core::generation::GalaxyConfig;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct SaveEntry {
    path: PathBuf,
    label: String,
    status: SaveStatus,
}

#[derive(Clone)]
enum SaveStatus {
    Empty,
    Ready { run: u64, ships: usize },
    Incompatible(String),
}

impl SaveStatus {
    fn inspect(path: &Path) -> Self {
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
    selected: PathBuf,
    startup_error: Option<String>,
    entries: Vec<SaveEntry>,
    pending_delete: Option<PathBuf>,
    message: Option<String>,
    galaxy: GalaxyConfig,
}

impl SaveRecovery {
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
            message: None,
            galaxy: GalaxyConfig::default(),
        }
    }

    fn refresh(&mut self) {
        for entry in &mut self.entries {
            entry.status = SaveStatus::inspect(&entry.path);
        }
    }

    fn delete(&mut self, path: &Path) -> Result<(), String> {
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
    pub fn open_save_chooser(&mut self, selected: PathBuf, error: Option<String>) {
        self.save_recovery = Some(SaveRecovery::new(selected, error));
    }

    pub(super) fn show_save_recovery(&mut self, ctx: &egui::Context) {
        let mut recovery = self.save_recovery.take().expect("recovery is open");
        let mut finished = false;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Choose a game");
                if let Some(error) = &recovery.startup_error {
                    ui.label("The last used save could not be loaded. Your files are still intact.");
                    ui.label(error);
                } else {
                    ui.label("Load a saved game or start a new one in an empty slot.");
                }
                ui.separator();
                ui.heading("New map size");
                crate::session::galaxy_controls(ui, &mut recovery.galaxy);
                ui.separator();
                for entry in recovery.entries.clone() {
                    ui.group(|ui| {
                        ui.strong(&entry.label);
                        if entry.path == recovery.selected {
                            ui.label("Last used");
                        }
                        ui.small(entry.path.display().to_string());
                        match &entry.status {
                            SaveStatus::Ready { run, ships } => {
                                ui.label(format!("Ready · run {run} · {ships} ships"));
                                if ui.button("Load game").clicked() {
                                    match self.session.load_path(&entry.path) {
                                        Ok(message) => {
                                            self.notify(message.unwrap_or_else(|| "Saved game loaded.".into()));
                                            finished = true;
                                        }
                                        Err(error) => {
                                            recovery.message = Some(error);
                                            recovery.refresh();
                                        }
                                    }
                                }
                            }
                            SaveStatus::Empty => {
                                ui.label("Empty slot");
                                if ui.button("Start new game here").clicked() {
                                    if !matches!(SaveStatus::inspect(&entry.path), SaveStatus::Empty) {
                                        recovery.refresh();
                                        recovery.message = Some("This slot changed. Choose it again.".into());
                                    } else {
                                        let seed = crate::seed::new_seed();
                                        match self.session.start_new_at(entry.path.clone(), seed, recovery.galaxy) {
                                            Ok(()) => {
                                                self.notify(format!("New map (seed {seed}, {} systems, {} ly radius, up to {} objects per system). This slot will save on close.", recovery.galaxy.systems, recovery.galaxy.radius_ly, recovery.galaxy.max_objects));
                                                finished = true;
                                            }
                                            Err(error) => recovery.message = Some(error),
                                        }
                                    }
                                }
                            }
                            SaveStatus::Incompatible(error) => {
                                ui.label("Cannot load this save");
                                ui.small(error);
                                if recovery.pending_delete.as_deref() == Some(entry.path.as_path()) {
                                    ui.label("Delete this save and its backup permanently?");
                                    ui.horizontal(|ui| {
                                        if ui.button("Confirm delete").clicked() {
                                            recovery.message = Some(match recovery.delete(&entry.path) {
                                                Ok(()) => "Save and backup deleted. You can start a new game in this slot.".into(),
                                                Err(error) => error,
                                            });
                                        }
                                        if ui.button("Cancel").clicked() {
                                            recovery.pending_delete = None;
                                        }
                                    });
                                } else if ui.button("Delete save and backup…").clicked() {
                                    recovery.pending_delete = Some(entry.path.clone());
                                }
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
                if let Some(message) = &recovery.message {
                    ui.separator();
                    ui.label(message);
                }
            });
        });
        if !finished {
            self.save_recovery = Some(recovery);
        }
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
