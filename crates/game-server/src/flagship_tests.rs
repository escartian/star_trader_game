use super::{
    LOCAL_PLAYER, Session,
    combat_tests::reload,
    navigation_tests::{run, succeed, two_fleets},
};
use game_core::{
    LocalPosition, SystemId,
    fleet::{FleetId, ShipKind},
    navigation::NavigationError,
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn patrol(session: &mut Session, fleet: FleetId, system: SystemId) -> ServerMessage {
    session.handle(ClientCommand::SetPatrol {
        fleet,
        system,
        expected_revision: session.state.revision,
    })
}

fn wait(session: &mut Session, fleet: FleetId) {
    let revision = session.state.revision;
    succeed(
        session,
        ClientCommand::HoldFleet {
            fleet,
            expected_revision: revision,
        },
    );
}

#[test]
fn a_patrol_tours_its_system_body_by_body_and_wait_ends_it() {
    let mut session = two_fleets();
    assert_eq!(
        patrol(&mut session, FleetId(1), SystemId(1)),
        ServerMessage::CommandRejected(NavigationError::FlagshipCommanded.into())
    );
    assert!(matches!(
        patrol(&mut session, FleetId(2), SystemId(1)),
        ServerMessage::SessionSnapshot { .. }
    ));
    wait(&mut session, FleetId(1));
    let mut visited = std::collections::HashSet::new();
    for _ in 0..24 {
        run(&mut session);
        let at = session.state.ships[1].location.clone();
        if matches!(at, Location::Orbiting { .. } | Location::Docked(_)) {
            visited.insert(format!("{at:?}"));
        }
    }
    let stops = session.state.patrol(FleetId(2)).unwrap().stops;
    assert!(stops >= 3, "{stops} stops");
    assert!(visited.len() >= 2, "{visited:?}");
    reload(&session);
    // A Wait order replaces the patrol.
    wait(&mut session, FleetId(2));
    assert!(session.state.patrol(FleetId(2)).is_none());
    // Taking command of a patrolling fleet ends its patrol too.
    patrol(&mut session, FleetId(2), SystemId(1));
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SetFlagship {
            fleet: FleetId(2),
            expected_revision: revision,
        },
    );
    assert!(session.state.patrol(FleetId(2)).is_none());
    assert_eq!(session.state.flagship(LOCAL_PLAYER), Some(FleetId(2)));
}

#[test]
fn a_secondary_fleet_fights_raiders_by_itself_and_earns_the_bounty() {
    let mut session = two_fleets();
    session.state.movement_rules.encounter_chance_percent = 100;
    session.state.ships[1].kind = ShipKind::Patrol;
    // Fleet 2 waits in the open in system 2, right beside the band there.
    let band = session.state.navigation.bands[0].clone();
    session.state.ships[1].location = Location::InSystem {
        system: band.system,
        position: LocalPosition {
            z_km: band.position.z_km + 1_000_000,
            ..band.position
        },
    };
    wait(&mut session, FleetId(1));
    wait(&mut session, FleetId(2));
    let credits = session.economy.credits;
    let mut fought = false;
    for _ in 0..4 {
        run(&mut session);
        assert!(
            session
                .state
                .navigation
                .encounters
                .iter()
                .all(|contact| contact.fleet == FleetId(1)),
            "secondary contacts never wait for the player"
        );
        if let Some(report) = &session.state.navigation.last_combat {
            assert_eq!(report.fleet, FleetId(2));
            let destroyed = report
                .combatants
                .iter()
                .filter(|ship| ship.destroyed)
                .count();
            if destroyed > 0 && report.bounty.0 > 0 {
                assert!(session.economy.credits.0 >= credits.0 + report.bounty.0);
            }
            fought = true;
            break;
        }
    }
    assert!(fought, "the band struck and the fight resolved itself");
    reload(&session);
}

#[test]
fn losing_the_flagship_hands_command_to_another_fleet() {
    let mut session = two_fleets();
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SetFlagship {
            fleet: FleetId(2),
            expected_revision: revision,
        },
    );
    session.state.fleets.retain(|fleet| fleet.id != FleetId(2));
    session.state.ships.retain(|ship| ship.id != ShipId(2));
    session.state.prune_standing_orders();
    assert_eq!(session.state.navigation.flagship, None);
    assert_eq!(session.state.flagship(LOCAL_PLAYER), Some(FleetId(1)));
}
