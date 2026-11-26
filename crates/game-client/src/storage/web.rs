//! The browser side of `storage`: entries in `localStorage` under
//! `star-trader/<path>`, saves compressed by `codec`. Storage belongs to the
//! site's origin and survives reloads and visits until the player clears the
//! site's data. Writes that exceed the browser's quota fail with a message.
use super::{backup_path, codec};
use crate::preferences::Preferences;
use game_server::Session;
use std::path::Path;

fn key(path: &Path) -> String {
    format!("star-trader/{}", path.to_string_lossy())
}

fn store() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| "This browser does not allow saving (storage is off)".to_owned())
}

fn get(path: &Path) -> Option<String> {
    store().ok()?.get_item(&key(path)).ok().flatten()
}

fn set(path: &Path, value: &str) -> Result<(), String> {
    store()?
        .set_item(&key(path), value)
        .map_err(|_| "The browser refused to store the save (storage full?)".to_owned())
}

fn remove(path: &Path) -> Result<(), String> {
    store()?
        .remove_item(&key(path))
        .map_err(|_| "The browser refused to remove the save".to_owned())
}

fn read_session(path: &Path) -> Result<(Session, Option<String>), String> {
    let stored = get(path).ok_or("No save here")?;
    Session::import_json(&codec::decode(&stored)?)
}

pub fn load_session(path: &Path) -> Result<(Session, Option<String>), String> {
    match read_session(path) {
        Ok(loaded) => Ok(loaded),
        Err(primary) => match read_session(&backup_path(path)) {
            Ok((session, notice)) => {
                let recovered =
                    format!("Recovered previous save from backup. Primary save: {primary}");
                Ok((
                    session,
                    Some(
                        notice.map_or(recovered.clone(), |notice| format!("{recovered} {notice}")),
                    ),
                ))
            }
            Err(backup) => Err(format!("Could not load save: {primary}. Backup: {backup}")),
        },
    }
}

pub fn save_session(session: &Session, path: &Path) -> Result<(), String> {
    let stored = codec::encode(&session.export_compact_json()?)?;
    // Never replace a good backup with a damaged primary entry.
    if let Some(previous) = get(path)
        && codec::decode(&previous).is_ok()
    {
        set(&backup_path(path), &previous)?;
    }
    set(path, &stored)
}

pub fn has_save(path: &Path) -> bool {
    get(path).is_some() || get(&backup_path(path)).is_some()
}

pub fn delete_save(path: &Path) -> Result<(), String> {
    remove(&backup_path(path))?;
    remove(path)
}

pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    match get(path) {
        Some(json) => Preferences::from_json(json.as_bytes()),
        None => Ok(Preferences::default()),
    }
}

pub fn save_preferences(preferences: &Preferences, path: &Path) -> Result<(), String> {
    set(path, &preferences.to_json()?)
}
