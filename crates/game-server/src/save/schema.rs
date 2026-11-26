//! Save schema gate. Space became flat in schema 7; older saves stand off the
//! map plane and are not migrated (prototype rule: no backward compatibility).
//! Schema 8 moved battles from the 3D grid to the hex board, and schema 9 made
//! that board a single layer with terrain. Schema 10 adds two alien powers.
//! Older supported saves keep their original factions and gain empty slots
//! for the new powers. Open legacy battles are auto-resolved.
use serde_json::Value;

/// The schema this build writes and reads.
pub(super) const SCHEMA: u64 = 10;
/// The last schema with 3D battle grids.
const GRID_BATTLES: u64 = 7;
/// The last schema with altitude bands on the hex board.
const BAND_BATTLES: u64 = 8;
const HUMAN_POWERS: usize = 4;

/// Accepts the current schema, upgrades schemas 7 to 9 in place, and rejects
/// the rest. Returns an older save's open battle, removed from `data`.
pub(super) fn upgrade(data: &mut Value) -> Result<Option<Value>, String> {
    match data.get("schema_version").and_then(Value::as_u64) {
        Some(SCHEMA) => Ok(None),
        Some(GRID_BATTLES | BAND_BATTLES | 9) => {
            let old = data["schema_version"].as_u64().unwrap_or(9);
            add_alien_powers(data)?;
            data["schema_version"] = SCHEMA.into();
            let battle = (old <= BAND_BATTLES)
                .then(|| {
                    data.pointer_mut("/player/navigation/battle")
                        .map(Value::take)
                })
                .flatten()
                .filter(|battle| !battle.is_null());
            Ok(battle)
        }
        Some(version) if version < GRID_BATTLES => Err(format!(
            "This save uses schema {version}, from before flat maps; it cannot be loaded. Start a new game."
        )),
        _ => Err("Unsupported save schema version".into()),
    }
}

fn add_alien_powers(data: &mut Value) -> Result<(), String> {
    // Legacy battle fixtures replace the version and battle only; their
    // faction arrays and catalogs already use the current shape.
    if data
        .pointer("/economy/factions/standing")
        .and_then(Value::as_array)
        .is_some_and(|standing| standing.len() == 6)
    {
        return Ok(());
    }
    let catalog = game_core::factions::FactionCatalog::default();
    let quests = game_core::jobs::quests::QuestCatalog::default();
    for (path, fill) in [
        ("/economy/factions/standing", Value::from(0)),
        ("/economy/factions/sales_cents", Value::from(0)),
        ("/economy/factions/wanted", Value::from(0)),
        ("/player/jobs/quest_steps", Value::from(0)),
        ("/player/jobs/quest_retry", Value::from(0)),
    ] {
        let entries = data
            .pointer_mut(path)
            .and_then(Value::as_array_mut)
            .ok_or_else(|| format!("Missing {path} in older save"))?;
        if entries.len() != HUMAN_POWERS {
            return Err(format!("Invalid {path} in older save"));
        }
        entries.extend([fill.clone(), fill]);
    }
    let definitions = data
        .pointer_mut("/economy/rules/faction_catalog/factions")
        .and_then(Value::as_array_mut)
        .ok_or("Missing faction catalog in older save")?;
    if definitions.len() != HUMAN_POWERS {
        return Err("Invalid faction catalog in older save".into());
    }
    for definition in definitions.iter_mut() {
        definition["species"] = "Human".into();
    }
    for alien in catalog.factions.into_iter().skip(HUMAN_POWERS) {
        definitions.push(serde_json::to_value(alien).map_err(|error| error.to_string())?);
    }
    data["economy"]["rules"]["faction_catalog"]["schema_version"] = 2.into();
    let relations = data
        .pointer_mut("/economy/rules/faction_catalog/classic_relations")
        .and_then(Value::as_array_mut)
        .ok_or("Missing classic relations in older save")?;
    for relation in catalog.classic_relations {
        if relation
            .between
            .iter()
            .any(|faction| faction.index() >= HUMAN_POWERS)
        {
            relations.push(serde_json::to_value(relation).map_err(|error| error.to_string())?);
        }
    }
    let chains = data
        .pointer_mut("/economy/rules/quest_catalog/chains")
        .and_then(Value::as_array_mut)
        .ok_or("Missing quest chains in older save")?;
    for chain in quests.chains {
        if chain.faction.index() >= HUMAN_POWERS {
            chains.push(serde_json::to_value(chain).map_err(|error| error.to_string())?);
        }
    }
    Ok(())
}
