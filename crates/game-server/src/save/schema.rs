//! Save schema gate. Space became flat in schema 7; older saves stand off the
//! map plane and are not migrated (prototype rule: no backward compatibility).
use serde_json::Value;

/// The schema this build writes and reads.
pub(super) const SCHEMA: u64 = 7;

pub(super) fn check(data: &Value) -> Result<(), String> {
    match data.get("schema_version").and_then(Value::as_u64) {
        Some(SCHEMA) => Ok(()),
        Some(version) if version < SCHEMA => Err(format!(
            "This save uses schema {version}, from before flat maps; it cannot be loaded. Start a new game."
        )),
        _ => Err("Unsupported save schema version".into()),
    }
}
