use super::{Session, fs};
use serde_json::json;

#[test]
fn malformed_market_identity_inventory_and_planet_metadata_are_rejected_on_load() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tampered.json");
    let valid: serde_json::Value =
        serde_json::from_str(&Session::default().export_json().unwrap()).unwrap();
    for change in [
        "duplicate",
        "body",
        "station",
        "funds",
        "stock",
        "planet",
        "biome",
    ] {
        let mut value = valid.clone();
        match change {
            "duplicate" => {
                value["economy"]["markets"][1]["id"] = value["economy"]["markets"][0]["id"].clone()
            }
            "body" => value["economy"]["markets"][0]["id"]["body"] = json!(999),
            "station" => value["economy"]["markets"][1]["is_station"] = json!(true),
            "funds" => value["economy"]["markets"][0]["credits"] = json!(u64::MAX),
            "stock" => value["economy"]["markets"][0]["ship_stock"][0] = json!(1001),
            "planet" => value["systems"][0]["bodies"][0]["planet"] = json!(null),
            "biome" => value["systems"][0]["bodies"][0]["planet"]["biome"] = json!("gas"),
            _ => unreachable!(),
        }
        fs::write(&path, value.to_string()).unwrap();
        assert!(Session::load(&path).is_err(), "{change}");
    }
}

#[test]
fn saved_planet_definitions_and_treasuries_override_new_game_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("custom.json");
    let mut rules = game_core::economy::EconomyRules::default();
    rules.planet_catalog.templates[0].starting_credits.0 = 12345;
    rules.planet_catalog.templates[0].profile.danger = game_core::planet::Danger::VeryDangerous;
    let session = Session::with_economy(42, rules).unwrap();
    session.save(&path).unwrap();
    let (restored, _) = Session::load(&path).unwrap();
    assert_eq!(restored.economy, session.economy);
    assert_eq!(restored.systems, session.systems);
    assert_ne!(restored.systems, Session::with_seed(42).systems);
}
