use super::{Session, fs};
use game_core::{BodyId, SystemId, movement::MoveTarget, travel::ShipId};
use serde_json::json;

#[test]
fn forged_course_geometry_counters_membership_and_encounters_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-course.json");
    let mut session = Session::default();
    crate::navigation_tests::plot(
        &mut session,
        ShipId(1),
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
    );
    let pending: serde_json::Value = serde_json::from_str(&session.export_json().unwrap()).unwrap();
    for key in [
        "fuel",
        "ticks",
        "fleet",
        "destination",
        "duplicate",
        "hold",
        "run",
    ] {
        let mut invalid = pending.clone();
        match key {
            "fuel" => {
                invalid["player"]["navigation"]["orders"][0]["plan"]["fuel_per_ship"] = json!(0)
            }
            "ticks" => invalid["player"]["navigation"]["orders"][0]["remaining_ticks"] = json!(0),
            "fleet" => invalid["player"]["navigation"]["orders"][0]["plan"]["fleet"] = json!(99),
            "destination" => {
                invalid["player"]["navigation"]["orders"][0]["plan"]["destination"] =
                    json!({"Docked":2})
            }
            "duplicate" => {
                let order = invalid["player"]["navigation"]["orders"][0].clone();
                invalid["player"]["navigation"]["orders"]
                    .as_array_mut()
                    .unwrap()
                    .push(order);
            }
            "hold" => invalid["player"]["navigation"]["holds"] = json!([1]),
            "run" => invalid["player"]["navigation"]["run"] = json!(u64::MAX),
            _ => unreachable!(),
        }
        fs::write(&path, invalid.to_string()).unwrap();
        assert!(Session::load(&path).is_err(), "{key}");
    }
    session.state.movement_rules.encounter_chance_percent = 100;
    crate::navigation_tests::run(&mut session);
    let mut invalid: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    invalid["player"]["navigation"]["orders"] = json!([]);
    fs::write(&path, invalid.to_string()).unwrap();
    assert!(Session::load(&path).is_err());
}
