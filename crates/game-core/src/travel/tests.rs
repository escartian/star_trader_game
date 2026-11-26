use super::*;

fn position(x: i64, y: i64, z: i64) -> GalaxyPosition {
    GalaxyPosition { x, y, z }
}

#[test]
fn range_and_fuel_boundaries_use_exact_integer_distance() {
    let mut state = TravelState::default();
    let origin = position(0, 0, 0);
    let destination = SystemId(2);
    assert_eq!(
        state
            .quote(destination, origin, position(12, 16, 0))
            .unwrap()
            .fuel,
        20
    );
    assert_eq!(
        state.quote(destination, origin, position(12, 16, 1)),
        Err(TravelError::OutOfRange)
    );
    assert_eq!(
        state
            .quote(destination, origin, position(8, 4, 12))
            .unwrap()
            .fuel,
        15
    );
    state.ships[0].fuel = 14;
    assert_eq!(
        state.quote(destination, origin, position(8, 4, 12)),
        Err(TravelError::InsufficientFuel)
    );
    state.ships[0].fuel = 15;
    assert!(state.quote(destination, origin, position(8, 4, 12)).is_ok());
}

#[test]
fn departure_and_arrival_preserve_fuel_and_time_invariants() {
    let mut state = TravelState::default();
    let quote = state
        .quote(SystemId(2), position(0, 0, 0), position(8, 4, 12))
        .unwrap();
    state.depart(quote).unwrap();
    assert_eq!(state.tick, 0);
    assert_eq!(state.ships[0].fuel, 25);
    assert_eq!(state.advance(14), Ok(Vec::new()));
    assert!(matches!(state.ships[0].location, Location::Travelling(_)));
    assert_eq!(state.advance(1), Ok(vec![(ShipId(1), SystemId(2))]));
    assert_eq!(state.ships[0].location, Location::Docked(SystemId(2)));
    assert_eq!(state.ships[0].fuel, 25);
    assert_eq!(state.advance(100), Ok(Vec::new()));
}

#[test]
fn overflow_and_stale_departures_do_not_partially_mutate_state() {
    let mut state = TravelState::default();
    let quote = state
        .quote(SystemId(2), position(0, 0, 0), position(8, 4, 12))
        .unwrap();
    state.tick = u64::MAX - 1;
    let original = state.clone();
    assert_eq!(state.depart(quote.clone()), Err(TravelError::TimeOverflow));
    assert_eq!(state, original);
    assert_eq!(state.advance(2), Err(TravelError::TimeOverflow));
    assert_eq!(state, original);
    state.tick = 0;
    state.revision = 1;
    let original = state.clone();
    assert_eq!(state.depart(quote), Err(TravelError::StaleQuote));
    assert_eq!(state, original);
}

#[test]
fn extreme_coordinates_fail_without_overflow() {
    let state = TravelState::default();
    assert_eq!(
        state.quote(
            SystemId(2),
            position(i64::MIN, i64::MIN, i64::MIN),
            position(i64::MAX, i64::MAX, i64::MAX)
        ),
        Err(TravelError::InvalidCoordinates)
    );
}

#[test]
fn captain_names_are_cleaned_and_empty_reads_as_plain_captain() {
    assert_eq!(clean_captain_name("  Ilsa\t\tVey \n"), "Ilsa Vey");
    assert_eq!(clean_captain_name("A\u{7}B"), "A B");
    let long = "x".repeat(40);
    assert_eq!(
        clean_captain_name(&long).chars().count(),
        MAX_CAPTAIN_NAME_CHARS
    );
    // Cut at the limit, then trimmed: no trailing space.
    let edge = format!("{} y", "x".repeat(MAX_CAPTAIN_NAME_CHARS - 1));
    assert_eq!(
        clean_captain_name(&edge),
        "x".repeat(MAX_CAPTAIN_NAME_CHARS - 1)
    );
    let mut state = TravelState::default();
    assert_eq!(state.captain_title(), "Captain");
    state.captain_name = "Vey".into();
    assert_eq!(state.captain_title(), "Captain Vey");
}
