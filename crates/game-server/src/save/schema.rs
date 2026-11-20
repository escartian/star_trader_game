//! Save schema gate. Space became flat in schema 7; older saves stand off the
//! map plane and are not migrated (prototype rule: no backward compatibility).
//! Schema 8 moved battles from the 3D grid to the hex board; a schema-7 save is
//! otherwise identical, so it loads, and an open 3D battle is set aside for
//! `legacy_battle` to auto-resolve.
use serde_json::Value;

/// The schema this build writes and reads.
pub(super) const SCHEMA: u64 = 8;
/// The last schema with 3D battle grids.
const GRID_BATTLES: u64 = 7;

/// Accepts the current schema, upgrades schema 7 in place, and rejects the
/// rest. Returns a schema-7 save's open battle, removed from `data`.
pub(super) fn upgrade(data: &mut Value) -> Result<Option<Value>, String> {
    match data.get("schema_version").and_then(Value::as_u64) {
        Some(SCHEMA) => Ok(None),
        Some(GRID_BATTLES) => {
            data["schema_version"] = SCHEMA.into();
            let battle = data
                .pointer_mut("/player/navigation/battle")
                .map(Value::take)
                .filter(|battle| !battle.is_null());
            Ok(battle)
        }
        Some(version) if version < GRID_BATTLES => Err(format!(
            "This save uses schema {version}, from before flat maps; it cannot be loaded. Start a new game."
        )),
        _ => Err("Unsupported save schema version".into()),
    }
}
