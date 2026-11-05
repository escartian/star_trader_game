use super::*;
use game_core::{SystemId, travel::ShipId};
use game_protocol::{ClientCommand, ServerMessage};

#[test]
fn legacy_save_migrates_cargo_and_active_journey_without_resetting_time_or_money() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let mut session = Session::default();
    session.state.ships[0].cargo[0] = 7;
    session.handle(ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
        expected_revision: 0,
    });
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
