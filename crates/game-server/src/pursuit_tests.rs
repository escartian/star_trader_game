//! The flagship pursuing a trader or raider band (`game_core::pursuit`).
use super::{
    Session,
    band_tests::{near_band, run},
    combat_tests::reload,
    navigation_tests::succeed,
    piracy_tests::meet_trader,
};
use game_core::{
    LocalPosition, SystemId,
    fleet::FleetId,
    movement::MoveTarget,
    navigation::{EncounterKind, NavigationEvent},
    pursuit::Quarry,
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage};

fn pursue(session: &mut Session, quarry: Quarry) -> ServerMessage {
    session.handle(ClientCommand::Pursue {
        fleet: FleetId(1),
        quarry,
        expected_revision: session.state.revision,
    })
}

fn ok(response: ServerMessage) {
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}

/// The band in system 2, `gap` km from the flagship, too battered to hunt on
/// its own; random contacts off so only the pursuit makes one.
fn quiet_band(gap: i64) -> (Session, u64) {
    let mut session = near_band(gap);
    session.state.movement_rules.encounter_chance_percent = 0;
    session.state.navigation.bands[0].cooldown = game_core::navigation::LOOT_COOLDOWN_RUNS;
    let band = session.state.navigation.bands[0].id;
    (session, band)
}

#[test]
fn a_pursued_band_is_run_down_and_its_fight_opens() {
    let (mut session, band) = quiet_band(100_000_000);
    ok(pursue(&mut session, Quarry::Band(band)));
    assert!(session.state.navigation.pursuit.is_some());
    assert!(session.state.navigation.order(FleetId(1)).is_some());
    let reloaded = reload(&session);
    assert_eq!(
        reloaded.state.navigation.pursuit,
        session.state.navigation.pursuit
    );
    for _ in 0..30 {
        // Keep it from turning to hunt us: only the pursuit closes the gap.
        session.state.navigation.bands[0].cooldown = game_core::navigation::LOOT_COOLDOWN_RUNS;
        let events = run(&mut session);
        if let Some(contact) = events.iter().find_map(|event| match event {
            NavigationEvent::EncounterFound(found) => Some(found.clone()),
            _ => None,
        }) {
            assert_eq!(
                (contact.kind, contact.band),
                (EncounterKind::Raiders, Some(band))
            );
            assert_eq!(session.state.navigation.pursuit, None, "caught");
            return;
        }
        assert!(
            session.state.navigation.pursuit.is_some(),
            "lost the band: {events:?}"
        );
    }
    panic!("the band was never caught");
}

#[test]
fn a_trader_in_reach_is_hailed_at_once_and_far_ones_are_chased() {
    let mut session = Session::default();
    let (_, trader) = meet_trader(&mut session, 0);
    let ServerMessage::SessionSnapshot { .. } = pursue(&mut session, Quarry::Trader(trader)) else {
        panic!("pursuit refused");
    };
    let contact = &session.state.navigation.encounters[0];
    assert_eq!(contact.kind, EncounterKind::Trader);
    assert_eq!(session.state.navigation.pursuit, None);

    // The same trader far out in open space: a course is plotted to it.
    let mut session = Session::default();
    let (_, trader) = meet_trader(&mut session, 0);
    let system = session.state.ships[0].location.system().unwrap();
    let far = LocalPosition {
        x_km: 300_000_000,
        y_km: 0,
        z_km: 0,
    };
    session.economy.traders[0].state.ships[0].location = Location::InSystem {
        system,
        position: far,
    };
    ok(pursue(&mut session, Quarry::Trader(trader)));
    let order = session.state.navigation.order(FleetId(1)).unwrap();
    assert_eq!(
        order.remaining_waypoints.last().unwrap_or(&order.target),
        &MoveTarget::Local {
            system,
            position: far
        }
    );
    // New orders end the chase.
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.navigation.pursuit, None);
}

#[test]
fn only_a_quarry_in_the_flagships_system_can_be_pursued_and_one_that_leaves_is_lost() {
    let (mut session, band) = quiet_band(100_000_000);
    session.state.navigation.bands[0].system = SystemId(3);
    assert!(matches!(
        pursue(&mut session, Quarry::Band(band)),
        ServerMessage::CommandRejected(_)
    ));
    assert!(matches!(
        pursue(&mut session, Quarry::Band(999)),
        ServerMessage::CommandRejected(_)
    ));
    session.state.navigation.bands[0].system = SystemId(2);
    ok(pursue(&mut session, Quarry::Band(band)));
    // The band slips away to another system.
    session.state.navigation.bands[0].system = SystemId(3);
    let events = run(&mut session);
    assert_eq!(session.state.navigation.pursuit, None);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NavigationEvent::PursuitNote { .. })),
        "{events:?}"
    );
}
