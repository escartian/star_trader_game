use super::*;

#[test]
fn json_round_trip_preserves_journey_markets_money_and_ticks() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    let mut session = Session::with_seed(42);
    crate::test_support::legacy_depart(&mut session);
    session.advance(4);
    session.save(&path).unwrap();
    let (restored, recovered) = Session::load(&path).unwrap();
    assert_eq!(restored.systems, session.systems);
    assert_eq!(restored.state, session.state);
    assert_eq!(restored.economy, session.economy);
    assert!(recovered.is_none());
    let json: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert!(json["player"]["ships"][0]["cargo"].is_object());
}

#[test]
fn damaged_primary_recovers_previous_save_and_invalid_schema_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    let mut session = Session::default();
    session.save(&path).unwrap();
    let original = session.state.clone();
    session.advance(5);
    session.save(&path).unwrap();
    fs::write(&path, b"broken JSON").unwrap();
    let (restored, recovered) = Session::load(&path).unwrap();
    assert!(recovered.is_some());
    assert_eq!(restored.state, original);
    // A successful save over a damaged primary must preserve the good backup.
    session.save(&path).unwrap();
    assert_eq!(read(&backup_path(&path)).unwrap().0.player, original);
    fs::remove_file(backup_path(&path)).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["schema_version"] = 99.into();
    fs::write(&path, value.to_string()).unwrap();
    assert!(
        Session::load(&path)
            .err()
            .unwrap()
            .contains("schema version")
    );
}

#[test]
fn invalid_balances_and_failed_writes_cannot_replace_good_saves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    let session = Session::default();
    session.save(&path).unwrap();
    let original = fs::read(&path).unwrap();
    let mut invalid = Session::default();
    invalid.state.ships[0].fuel = 41;
    assert!(invalid.save(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(session.save(directory.path()).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["player"]["ships"][0]["cargo"]["food"] = 999.into();
    fs::write(&path, value.to_string()).unwrap();
    assert!(Session::load(&path).is_err());
}
