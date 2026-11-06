use super::*;
use game_core::travel::ShipId;
use game_protocol::ClientCommand;

#[test]
fn legacy_save_migrates_cargo_and_active_journey_without_resetting_time_or_money() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let mut session = Session::default();
    session.state.ships[0].cargo[0] = 7;
    crate::test_support::legacy_depart(&mut session);
    session.advance(4);
    let expected = session.state.clone();
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    make_legacy_markets(&mut value);
    value["schema_version"] = 1.into();
    let mut ship = value["player"]["ships"][0].clone();
    for key in ["cargo", "cargo_capacity", "upgraded"] {
        value["economy"][key] = ship.as_object_mut().unwrap().remove(key).unwrap();
    }
    for key in ["name", "kind", "base_cargo_capacity"] {
        ship.as_object_mut().unwrap().remove(key);
    }
    let player = value["player"].as_object_mut().unwrap();
    for key in ["ships", "fleets", "next_ship_id", "next_fleet_id"] {
        player.remove(key);
    }
    player.insert("ship".into(), ship);
    value["economy"]["rules"]
        .as_object_mut()
        .unwrap()
        .remove("ship_catalog");
    fs::write(&path, value.to_string()).unwrap();
    let (mut restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_some());
    assert_eq!(restored.state, expected);
    assert_legacy_assets(&restored, &session);
    for _ in 0..3 {
        crate::test_support::execute_quiet_run(&mut restored);
    }
    assert_eq!(restored.state.tick, 16);
    restored.save(&path).unwrap();
    assert!(Session::load(&path).unwrap().1.is_none());
}

#[test]
fn duplicate_membership_and_invalid_identity_counters_are_rejected() {
    let session = Session::default();
    for key in ["duplicate", "counter", "overflow"] {
        let mut invalid = session.clone();
        if key == "duplicate" {
            invalid.state.fleets[0].ships.push(ShipId(1));
        } else if key == "counter" {
            invalid.state.next_ship_id = 1;
        } else {
            invalid.economy.rules.hold_upgrade_capacity = u32::MAX;
            invalid.state.ships[0].upgraded = true;
        }
        assert!(invalid.export_json().is_err());
    }
}

#[test]
fn schema_two_fleets_migrate_without_losing_locations_or_balances() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fleet-v2.json");
    let mut session = Session::default();
    session.handle_offline(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: game_core::fleet::ShipKind::Scout,
        expected_revision: 0,
    });
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    make_legacy_markets(&mut value);
    value["schema_version"] = 2.into();
    value["player"]
        .as_object_mut()
        .unwrap()
        .remove("movement_rules");
    fs::write(&path, value.to_string()).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_some());
    let mut expected = session.state.clone();
    expected.navigation = game_core::navigation::NavigationState::default();
    assert_eq!(restored.state, expected);
    assert_legacy_assets(&restored, &session);
}

#[test]
fn invalid_movement_rules_and_positions_cannot_be_saved_or_loaded() {
    let mut session = Session::default();
    for location in [
        game_core::travel::Location::InSystem {
            system: game_core::SystemId(1),
            position: game_core::LocalPosition {
                x_km: 0,
                y_km: 0,
                z_km: 0,
            },
        },
        game_core::travel::Location::InSystem {
            system: game_core::SystemId(1),
            position: game_core::LocalPosition {
                x_km: 200_000_001,
                y_km: 0,
                z_km: 0,
            },
        },
        game_core::travel::Location::DeepSpace(game_core::movement::SpacePosition {
            x_km: i64::MAX,
            y_km: 0,
            z_km: 0,
        }),
        game_core::travel::Location::Orbiting {
            system: game_core::SystemId(1),
            body: game_core::BodyId(999),
        },
    ] {
        session.state.ships[0].location = location;
        assert!(session.export_json().is_err());
    }
    session = Session::default();
    session.state.movement_rules.system_half_extent_km = i64::MAX;
    assert!(session.export_json().is_err());
}

#[test]
fn malformed_legacy_player_records_report_load_errors_instead_of_panicking() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("malformed.json");
    for player in [
        serde_json::Value::Null,
        serde_json::json!("invalid"),
        serde_json::json!([]),
        serde_json::json!(42),
    ] {
        fs::write(
            &path,
            serde_json::json!({"schema_version":2,"player":player}).to_string(),
        )
        .unwrap();
        assert!(Session::load(&path).is_err());
    }
}

fn make_legacy_markets(value: &mut serde_json::Value) {
    value["economy"].as_object_mut().unwrap().remove("traders");
    let markets = value["economy"]["markets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|market| market["is_station"] == true)
        .map(|market| serde_json::json!({"system":market["id"]["system"],"goods":market["goods"]}))
        .collect::<Vec<_>>();
    value["economy"]["markets"] = serde_json::Value::Array(markets);
    for system in value["systems"].as_array_mut().unwrap() {
        for body in system["bodies"].as_array_mut().unwrap() {
            body.as_object_mut().unwrap().remove("planet");
        }
    }
    for key in [
        "planet_catalog",
        "market_starting_credits",
        "shipyard_starting_stock",
    ] {
        value["economy"]["rules"]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
}

fn assert_legacy_assets(restored: &Session, original: &Session) {
    assert_eq!(restored.economy.credits, original.economy.credits);
    assert_eq!(restored.economy.rules, original.economy.rules);
    for market in &restored.economy.markets {
        assert_eq!(market.credits.0, 0);
        assert_eq!(market.ship_stock, [0; 3]);
        if market.is_station {
            assert_eq!(
                market.goods,
                original.economy.market(market.id).unwrap().goods
            );
        } else {
            assert!(market.goods.iter().all(|good| good.stock == 0));
        }
    }
}

#[test]
fn schema_three_body_positions_migrate_without_cloning_stock_into_planets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v3.json");
    let mut session = Session::default();
    crate::test_support::move_fleet(
        &mut session,
        ShipId(1),
        game_core::movement::MoveTarget::Body {
            system: game_core::SystemId(1),
            body: game_core::BodyId(1),
        },
    );
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    make_legacy_markets(&mut value);
    value["schema_version"] = 3.into();
    fs::write(&path, value.to_string()).unwrap();
    let (mut restored, notice) = Session::load(&path).unwrap();
    assert!(notice.unwrap().contains("start empty"));
    let mut expected = session.state.clone();
    expected.navigation = game_core::navigation::NavigationState::default();
    assert_eq!(restored.state, expected);
    assert_legacy_assets(&restored, &session);
    let market = restored
        .economy
        .current_market(&restored.state.ships[0])
        .unwrap();
    assert!(!market.is_station);
    assert!(market.goods.iter().all(|good| good.stock == 0));
    restored.advance(1);
    assert!(
        restored
            .economy
            .current_market(&restored.state.ships[0])
            .unwrap()
            .goods
            .iter()
            .any(|good| good.stock > 0)
    );
    assert!(
        restored
            .economy
            .markets
            .iter()
            .all(|market| market.credits.0 == 0)
    );
}
