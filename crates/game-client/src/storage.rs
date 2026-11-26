//! Where saves and client settings live: files on desktop, the browser's
//! `localStorage` on the web. Both keep the same names (`data/slot-1.json`,
//! `data/settings.json`) and a backup of each save (`…json.bak`), so slots,
//! the main menu and autosave work the same everywhere. Saves are checked
//! and upgraded by `game_server` on every load; a damaged save falls back to
//! its backup.
#[cfg(any(target_arch = "wasm32", test))]
mod codec;
#[cfg(target_arch = "wasm32")]
mod web;

use crate::preferences::Preferences;
use game_server::Session;
use std::path::Path;

/// Loads a save (or its backup), with a notice when the backup was used or
/// loading upgraded something.
pub fn load_session(path: &Path) -> Result<(Session, Option<String>), String> {
    #[cfg(target_arch = "wasm32")]
    return web::load_session(path);
    #[cfg(not(target_arch = "wasm32"))]
    return Session::load(path);
}

/// Saves `session` at `path`, keeping the previous good save as its backup.
pub fn save_session(session: &Session, path: &Path) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    return web::save_session(session, path);
    #[cfg(not(target_arch = "wasm32"))]
    return session.save(path);
}

/// Whether anything is saved at `path` (the save or its backup).
pub fn has_save(path: &Path) -> bool {
    #[cfg(target_arch = "wasm32")]
    return web::has_save(path);
    #[cfg(not(target_arch = "wasm32"))]
    return path.exists() || backup_path(path).exists();
}

/// Removes the save at `path` and its backup (the backup first, so a failure
/// leaves the primary in place where possible).
pub fn delete_save(path: &Path) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    return web::delete_save(path);
    #[cfg(not(target_arch = "wasm32"))]
    {
        for file in [backup_path(path), path.to_path_buf()] {
            match std::fs::remove_file(&file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("Could not delete {}: {error}", file.display())),
            }
        }
        Ok(())
    }
}

/// Client settings at `path`; defaults when none were saved yet.
pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    #[cfg(target_arch = "wasm32")]
    return web::load_preferences(path);
    #[cfg(not(target_arch = "wasm32"))]
    return Preferences::load(path);
}

pub fn save_preferences(preferences: &Preferences, path: &Path) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    return web::save_preferences(preferences, path);
    #[cfg(not(target_arch = "wasm32"))]
    return preferences.save(path);
}

/// `path` with `.bak` appended: where a save's backup is kept.
pub fn backup_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    std::path::PathBuf::from(name)
}
