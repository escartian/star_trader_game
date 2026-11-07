//! Schema 5 → 6: saved ship catalogs gain per-hull combat profiles.
//! Prices, holds, fuel and range stay as saved; combat stats come from the
//! embedded catalog for the same class. Ships start intact (`hull_damage` 0,
//! supplied by its serde default) and the weapon catalog defaults when absent.
use serde_json::Value;

pub(super) fn upgrade(data: &mut Value) -> Result<(), String> {
    let defaults = game_core::fleet::ShipCatalog::default();
    if let Some(catalog) = data
        .pointer_mut("/economy/rules/ship_catalog")
        .and_then(Value::as_object_mut)
    {
        let ships = catalog
            .get_mut("ships")
            .and_then(Value::as_array_mut)
            .ok_or("Saved ship catalog has no ships")?;
        for ship in ships {
            let kind: game_core::fleet::ShipKind = serde_json::from_value(
                ship.get("kind")
                    .cloned()
                    .ok_or("Saved ship class has no kind")?,
            )
            .map_err(|error| error.to_string())?;
            let combat = &defaults
                .spec(kind)
                .ok_or("Embedded catalog lacks a saved ship class")?
                .combat;
            ship.as_object_mut()
                .ok_or("Invalid saved ship class")?
                .insert(
                    "combat".into(),
                    serde_json::to_value(combat).map_err(|error| error.to_string())?,
                );
        }
        catalog.insert(
            "schema_version".into(),
            game_core::fleet::SHIP_CATALOG_SCHEMA.into(),
        );
    }
    data["schema_version"] = 6.into();
    Ok(())
}
