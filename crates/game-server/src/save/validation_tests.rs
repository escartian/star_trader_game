//! Saves are flat (schema 7): older schemas and anything off the map plane are
//! rejected; identity, rules and positions are validated before writing.
use super::*;
use game_core::travel::ShipId;

fn load_value(value: &serde_json::Value) -> Result<(Session, Option<String>), String> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    std::fs::write(&path, value.to_string()).unwrap();
    Session::load(&path)
}

#[test]
fn saves_from_before_flat_maps_are_rejected_with_a_clear_message() {
    let session = Session::default();
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    assert_eq!(value["schema_version"], 7);
    assert!(load_value(&value).is_ok());
    for old in [1, 5, 6] {
        value["schema_version"] = old.into();
        let error = load_value(&value).map(|_| ()).unwrap_err();
        assert!(error.contains("before flat maps"), "{old}: {error}");
    }
}

#[test]
fn systems_bodies_and_positions_off_the_plane_are_rejected() {
    let session = Session::default();
    let mut raised = session.clone();
    raised.systems[1].position.y = 3;
    assert!(raised.export_json().is_err(), "system off the plane");
    let value: serde_json::Value = serde_json::from_str(&session.export_json().unwrap()).unwrap();
    let mut body = value.clone();
    body["systems"][0]["bodies"][0]["orbit"]["epoch"]["y_km"] = 5_000_000.into();
    assert!(load_value(&body).is_err(), "orbit off the plane");
    let mut ship = session.clone();
    ship.state.ships[0].location = game_core::travel::Location::InSystem {
        system: game_core::SystemId(1),
        position: game_core::LocalPosition {
            x_km: 90_000_000,
            y_km: 1,
            z_km: 0,
        },
    };
    assert!(ship.export_json().is_err(), "fleet off the plane");
    let mut band = session.clone();
    band.state.navigation.bands[0].position.y_km = 1;
    assert!(band.export_json().is_err(), "raider band off the plane");
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
fn hull_damage_at_or_beyond_the_class_hull_is_rejected() {
    let mut session = Session::default();
    session.state.ships[0].hull_damage = 59;
    assert!(session.export_json().is_ok(), "damaged but alive");
    session.state.ships[0].hull_damage = 60;
    assert!(
        session.export_json().is_err(),
        "a destroyed ship cannot remain in a fleet"
    );
}
