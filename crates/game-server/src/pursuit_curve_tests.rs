//! In-system chases on a pursuit curve (`game_core::pursuit::curve`,
//! `game_core::navigation::chase`) end within a few runs.
use super::{Session, police_tests::policed, test_support::execute_quiet_run};
use game_core::{LocalPosition, SystemId, fleet::FleetId, pursuit::Quarry, travel::Location};
use game_protocol::{ClientCommand, ServerMessage};

/// Puts your fleet in `system`, `gap` km from `quarry_at` (on the far side
/// from the star where it fits), with no orders.
fn place_fleet(session: &mut Session, system: SystemId, quarry_at: LocalPosition, gap: i64) {
    let half = session.state.movement_rules.system_half_extent_km;
    let x_km = if quarry_at.x_km > 0 {
        quarry_at.x_km - gap
    } else {
        quarry_at.x_km + gap
    };
    let x_km = x_km.clamp(-half + 1, half - 1);
    for ship in &mut session.state.ships {
        ship.location = Location::InSystem {
            system,
            position: LocalPosition { x_km, ..quarry_at },
        };
    }
    session.state.navigation.orders.clear();
    session.state.navigation.holds.clear();
}

/// Runs from Pursue to contact (`None`: no contact within 12 runs).
fn runs_to_catch(session: &mut Session, quarry: Quarry) -> Option<u32> {
    let reply = session.handle(ClientCommand::Pursue {
        fleet: FleetId(1),
        quarry,
        expected_revision: session.state.revision,
    });
    if !matches!(reply, ServerMessage::SessionSnapshot { .. }) {
        return None;
    }
    for runs in 0..12 {
        if !session.state.navigation.encounters.is_empty() {
            return Some(runs);
        }
        let reply = session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision,
        });
        if !matches!(reply, ServerMessage::SessionSnapshot { .. }) {
            return None;
        }
    }
    None
}

/// Every trader resting or flying in a system, chased from 300 million km
/// (with the pursuit suite when `suite`).
fn trader_chases(suite: bool) -> Vec<Option<u32>> {
    let mut results = Vec::new();
    for warmup in 1..=8 {
        let mut base = Session::default();
        base.state.movement_rules.encounter_chance_percent = 0;
        for _ in 0..warmup {
            execute_quiet_run(&mut base);
        }
        if suite {
            base.state.fleets[0]
                .software
                .push(game_core::fleet::software::FleetSoftware::PursuitSuite);
        }
        for index in 0..base.economy.traders.len() {
            let mut session = base.clone();
            let npc = &session.economy.traders[index];
            let ship = &npc.state.ships[0];
            let Some(system) = ship.location.system() else {
                continue;
            };
            let world = session
                .systems
                .iter()
                .find(|known| known.id == system)
                .unwrap();
            let Some(at) = npc
                .state
                .navigation_position(ship.id, &session.systems)
                .and_then(|at| {
                    let centre =
                        game_core::movement::SpacePosition::from_galaxy(world.position).ok()?;
                    Some(LocalPosition {
                        x_km: at.x_km - centre.x_km,
                        y_km: at.y_km - centre.y_km,
                        z_km: at.z_km - centre.z_km,
                    })
                })
            else {
                continue;
            };
            let id = npc.id;
            place_fleet(&mut session, system, at, 300_000_000);
            results.push(runs_to_catch(&mut session, Quarry::Trader(id)));
        }
    }
    results
}

/// The system 2 raider band, chased from `gap` km; it lies low (cooldown),
/// so only your chase closes the distance.
fn band_chase(gap: i64) -> Option<u32> {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    execute_quiet_run(&mut session);
    session.state.movement_rules.encounter_chance_percent = 12;
    let band = session.state.navigation.bands[0].clone();
    session.state.navigation.bands[0].cooldown = 30;
    place_fleet(&mut session, band.system, band.position, gap);
    runs_to_catch(&mut session, Quarry::Band(band.id))
}

/// The system 2 police patrol on its rounds, chased from `gap` km.
fn patrol_chase(gap: i64) -> Option<u32> {
    let mut session = policed(Session::default(), FleetId(1), 0);
    session.state.movement_rules.encounter_chance_percent = 0;
    execute_quiet_run(&mut session);
    let patrol = session.state.navigation.police.patrols[0].clone();
    place_fleet(&mut session, patrol.system, patrol.position, gap);
    runs_to_catch(&mut session, Quarry::Patrol(patrol.id))
}

/// The system 2 band hunting your fleet, which waits in the open `gap` km
/// away: runs until it strikes.
fn band_hunt(gap: i64) -> Option<u32> {
    let mut session = Session::default();
    let band = session.state.navigation.bands[0].clone();
    place_fleet(&mut session, band.system, band.position, gap);
    session.handle(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: session.state.revision,
    });
    for runs in 1..=12 {
        let reply = session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision,
        });
        if !matches!(reply, ServerMessage::SessionSnapshot { .. }) {
            return None;
        }
        if session
            .state
            .navigation
            .encounters
            .iter()
            .any(|contact| contact.band == Some(band.id))
        {
            return Some(runs);
        }
    }
    None
}

const GAPS: [i64; 4] = [50_000_000, 150_000_000, 250_000_000, 350_000_000];

#[test]
fn bands_and_patrols_in_your_system_are_run_down_within_two_runs() {
    for gap in GAPS {
        assert!(
            band_chase(gap).is_some_and(|runs| runs <= 2),
            "band from {gap} km"
        );
        assert!(
            patrol_chase(gap).is_some_and(|runs| runs <= 2),
            "patrol from {gap} km"
        );
    }
}

#[test]
fn traders_are_caught_in_system_or_followed_and_caught_after_they_leave() {
    // From 300 million km. One that stays is caught within two runs; one
    // already leaving at the same speed reaches the edge first and is caught
    // in the next system. The pursuit suite's speed shortens both.
    let plain = trader_chases(false);
    let suite = trader_chases(true);
    assert!(plain.len() >= 8, "{plain:?}");
    assert!(
        plain.iter().all(|runs| runs.is_some_and(|runs| runs <= 10)),
        "{plain:?}"
    );
    let quick = plain
        .iter()
        .filter(|runs| runs.is_some_and(|runs| runs <= 2))
        .count();
    assert!(
        quick * 3 >= plain.len() * 2,
        "two thirds within two runs: {plain:?}"
    );
    assert!(
        suite.iter().all(|runs| runs.is_some_and(|runs| runs <= 6)),
        "{suite:?}"
    );
}

#[test]
fn a_band_hunting_you_strikes_a_waiting_fleet_within_two_runs() {
    for gap in [20_000_000, 60_000_000, 100_000_000, 140_000_000] {
        assert!(
            band_hunt(gap).is_some_and(|runs| runs <= 2),
            "from {gap} km"
        );
    }
}
