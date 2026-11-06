use crate::{economy::EconomyRules, planet::PlanetCatalog};

#[test]
fn planet_configuration_rejects_missing_classes_invalid_biomes_and_excessive_allocations() {
    let valid = serde_json::to_value(PlanetCatalog::default()).unwrap();
    for (field, value) in [
        ("starting_credits", serde_json::json!(u64::MAX)),
        ("market_template", serde_json::json!(64)),
        ("ship_stock", serde_json::json!([1001, 0, 0])),
        ("kind", serde_json::json!("Moon")),
    ] {
        let mut invalid = valid.clone();
        invalid["templates"][0][field] = value;
        assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    }
    let mut invalid = valid.clone();
    invalid["templates"]
        .as_array_mut()
        .unwrap()
        .retain(|template| template["kind"] == "RockyPlanet");
    assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    let mut invalid = valid;
    invalid["templates"][0]["profile"]["biome"] = serde_json::json!("gas");
    assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    let mut rules = EconomyRules {
        ship_resale_percent: 101,
        ..EconomyRules::default()
    };
    assert!(rules.validate().is_err());
    rules.ship_resale_percent = 60;
    rules.markets.truncate(1);
    assert!(rules.validate().is_ok());
    assert!(crate::economy::Economy::with_rules(&crate::demo_galaxy(), rules).is_ok());
}
