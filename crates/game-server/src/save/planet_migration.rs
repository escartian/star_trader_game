//! Preserve legacy station stock and player assets; never duplicate them into planets.
use game_core::{
    BodyKind, StarSystem,
    economy::{Economy, EconomyRules, MarketGood},
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyMarket {
    system: game_core::SystemId,
    goods: [MarketGood; 6],
}

pub(super) fn upgrade(data: &mut Value) -> Result<(), String> {
    let world = data
        .get_mut("systems")
        .and_then(Value::as_array_mut)
        .ok_or("Missing saved systems")?;
    for system in world.iter_mut() {
        let bodies = system
            .get_mut("bodies")
            .and_then(Value::as_array_mut)
            .ok_or("Missing saved bodies")?;
        for body in bodies {
            body.as_object_mut()
                .ok_or("Invalid saved body")?
                .insert("planet".into(), Value::Null);
        }
    }
    let mut systems: Vec<StarSystem> =
        serde_json::from_value(Value::Array(world.clone())).map_err(|error| error.to_string())?;
    let economy = data
        .get_mut("economy")
        .and_then(Value::as_object_mut)
        .ok_or("Missing saved economy")?;
    let rules: EconomyRules =
        serde_json::from_value(economy.get("rules").ok_or("Missing economy rules")?.clone())
            .map_err(|error| error.to_string())?;
    rules.validate()?;
    rules.planet_catalog.populate(&mut systems);
    let legacy: Vec<LegacyMarket> = serde_json::from_value(
        economy
            .get("markets")
            .ok_or("Missing saved markets")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    if legacy.len() != systems.len() {
        return Err("Missing legacy market".into());
    }
    let mut expanded = Economy::with_rules(&systems, rules)?;
    for market in &mut expanded.markets {
        market.credits.0 = 0;
        market.ship_stock = [0; 3];
        for good in &mut market.goods {
            good.stock = 0;
        }
    }
    let mut seen = HashSet::new();
    for market in legacy {
        if !seen.insert(market.system) {
            return Err("Duplicate legacy market".into());
        }
        let station = systems
            .iter()
            .find(|system| system.id == market.system)
            .and_then(|system| {
                system
                    .bodies
                    .iter()
                    .find(|body| body.kind == BodyKind::Station)
            })
            .ok_or("Unknown legacy market system")?;
        let id = game_core::economy::MarketId {
            system: market.system,
            body: station.id,
        };
        expanded
            .market_mut(id)
            .map_err(|error| error.to_string())?
            .goods = market.goods;
    }
    economy.insert(
        "markets".into(),
        serde_json::to_value(expanded.markets).map_err(|error| error.to_string())?,
    );
    economy.insert(
        "rules".into(),
        serde_json::to_value(expanded.rules).map_err(|error| error.to_string())?,
    );
    data["systems"] = serde_json::to_value(systems).map_err(|error| error.to_string())?;
    data["schema_version"] = 4.into();
    Ok(())
}
