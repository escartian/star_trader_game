//! Saves are flat (since schema 7; schema 8 moved battles to the hex board):
//! older schemas and anything off the map plane are rejected; identity, rules
//! and positions are validated before writing.
use super::*;
use game_core::travel::ShipId;
use game_core::{
    campaign::{CampaignEnd, Road},
    fleet::ShipKind,
};

fn load_value(value: &serde_json::Value) -> Result<(Session, Option<String>), String> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    std::fs::write(&path, value.to_string()).unwrap();
    Session::load(&path)
}

#[test]
fn saves_from_before_flat_maps_are_rejected_with_a_clear_message() {
    let session = Session::default();
    let value: serde_json::Value = serde_json::from_str(&session.export_json().unwrap()).unwrap();
    assert_eq!(value["schema_version"], 10);
    assert!(load_value(&value).is_ok());
    for older in [7, 8, 9] {
        let mut legacy = value.clone();
        legacy["schema_version"] = older.into();
        for path in [
            "/economy/factions/standing",
            "/economy/factions/sales_cents",
            "/economy/factions/wanted",
            "/player/jobs/quest_steps",
            "/player/jobs/quest_retry",
            "/economy/rules/faction_catalog/factions",
        ] {
            legacy
                .pointer_mut(path)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .truncate(4);
        }
        for definition in legacy["economy"]["rules"]["faction_catalog"]["factions"]
            .as_array_mut()
            .unwrap()
        {
            definition.as_object_mut().unwrap().remove("species");
        }
        legacy["economy"]["rules"]["faction_catalog"]["classic_relations"]
            .as_array_mut()
            .unwrap()
            .retain(|relation| {
                relation["between"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|faction| faction != "Miri" && faction != "Auralith")
            });
        legacy["economy"]["rules"]["quest_catalog"]["chains"]
            .as_array_mut()
            .unwrap()
            .retain(|chain| chain["faction"] != "Miri" && chain["faction"] != "Auralith");
        let (_, notice) = load_value(&legacy).unwrap();
        assert!(
            notice.is_none(),
            "schema {older} without a battle loads as is"
        );
    }
    for old in [1, 5, 6] {
        let mut legacy = value.clone();
        legacy["schema_version"] = old.into();
        let error = load_value(&legacy).map(|_| ()).unwrap_err();
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

#[test]
fn forged_campaign_ends_and_garrisons_cannot_be_saved() {
    let mut session = Session::default();
    session.state.campaign.ended = Some(CampaignEnd::Won {
        road: Road::Pirate,
        run: 0,
    });
    assert!(session.export_json().is_err(), "locked road");
    session.state.campaign.ended = Some(CampaignEnd::Won {
        road: Road::Soldier,
        run: 0,
    });
    assert!(session.export_json().is_err(), "standing Haven");
    session.state.campaign.ended = Some(CampaignEnd::Lost { run: 0 });
    assert!(session.export_json().is_err(), "living ship");
    session.state.campaign.ended = Some(CampaignEnd::TimeUp { run: 0 });
    assert!(session.export_json().is_err(), "no campaign length");
    session.state.campaign.ended = None;
    session.state.campaign.run_limit = Some(1);
    session.state.navigation.run = 1;
    assert!(
        session.export_json().is_err(),
        "expired campaign still live"
    );

    let mut session = Session::default();
    session.state.navigation.haven.as_mut().unwrap().garrison[0] = ShipKind::Freighter;
    assert!(session.export_json().is_err(), "foreign garrison hull");
    session.state.navigation.haven.as_mut().unwrap().garrison[0] = ShipKind::Scout;
    assert!(session.export_json().is_err(), "too many scouts");
}
