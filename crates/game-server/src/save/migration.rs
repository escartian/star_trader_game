//! Upgrade the original single-ship JSON without regenerating balances or positions.
use serde_json::{Value, json};

pub(super) fn upgrade(mut data: Value) -> Result<(Value, bool), String> {
    match data.get("schema_version").and_then(Value::as_u64) {
        Some(3) => Ok((data, false)),
        Some(2) => {
            let player = data
                .get_mut("player")
                .and_then(Value::as_object_mut)
                .ok_or("Missing or malformed saved player")?;
            player.insert(
                "movement_rules".into(),
                serde_json::to_value(game_core::movement::MovementRules::default())
                    .map_err(|error| error.to_string())?,
            );
            data["schema_version"] = 3.into();
            Ok((data, true))
        }
        Some(1) => {
            let economy = data
                .get_mut("economy")
                .and_then(Value::as_object_mut)
                .ok_or("Missing saved economy")?;
            let cargo = economy.remove("cargo").ok_or("Missing legacy cargo")?;
            let capacity = economy
                .remove("cargo_capacity")
                .and_then(|value| value.as_u64())
                .ok_or("Missing cargo capacity")?;
            let upgraded = economy
                .remove("upgraded")
                .and_then(|value| value.as_bool())
                .ok_or("Missing upgrade state")?;
            let extra = economy
                .get("rules")
                .and_then(|rules| rules.get("hold_upgrade_capacity"))
                .and_then(Value::as_u64)
                .ok_or("Missing upgrade rules")?;
            let base = capacity
                .checked_sub(if upgraded { extra } else { 0 })
                .ok_or("Invalid legacy cargo capacity")?;
            let player = data
                .get_mut("player")
                .and_then(Value::as_object_mut)
                .ok_or("Missing saved player")?;
            let mut ship = player.remove("ship").ok_or("Missing legacy ship")?;
            let id = ship
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("Missing legacy ship ID")?;
            let next = id.checked_add(1).ok_or("Legacy ship ID exhausted")?;
            let record = ship.as_object_mut().ok_or("Invalid legacy ship")?;
            record.insert("name".into(), format!("Freighter {id}").into());
            record.insert("kind".into(), "freighter".into());
            record.insert("cargo".into(), cargo);
            record.insert("cargo_capacity".into(), capacity.into());
            record.insert("base_cargo_capacity".into(), base.into());
            record.insert("upgraded".into(), upgraded.into());
            player.insert("ships".into(), json!([ship]));
            player.insert(
                "fleets".into(),
                json!([{"id":1,"name":"Fleet 1","owner":1,"ships":[id]}]),
            );
            player.insert("next_ship_id".into(), next.into());
            player.insert("next_fleet_id".into(), 2.into());
            data["schema_version"] = 2.into();
            let (data, _) = upgrade(data)?;
            Ok((data, true))
        }
        _ => Err("Unsupported save schema version".into()),
    }
}
