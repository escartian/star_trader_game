use super::*;
use game_core::travel::ShipId;
use game_protocol::{ClientCommand, ServerMessage};

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
    assert_eq!(restored.economy, session.economy);
    assert!(matches!(
        restored.handle_offline(ClientCommand::ContinueTravel {
            ship: ShipId(1),
            expected_revision: expected.revision
        }),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert_eq!(restored.state.tick, 15);
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
    value["schema_version"] = 2.into();
    value["player"]
        .as_object_mut()
        .unwrap()
        .remove("movement_rules");
    fs::write(&path, value.to_string()).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_some());
    assert_eq!(restored.state, session.state);
    assert_eq!(restored.economy, session.economy);
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
