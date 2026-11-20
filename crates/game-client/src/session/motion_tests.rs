use super::{LocalSession, light_years, motion::GLIDE_SECONDS};
use game_core::{
    BodyId, StarSystem, SystemId,
    fleet::FleetId,
    movement::{MoveTarget, SpacePosition},
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn session() -> (LocalSession, Vec<StarSystem>) {
    let mut authority = game_server::Session::default();
    authority.set_encounter_chance(0);
    let ServerMessage::GalaxySnapshot { systems, .. } = authority.handle(ClientCommand::GetGalaxy)
    else {
        unreachable!("the galaxy request always answers with a snapshot")
    };
    (LocalSession::new(authority), systems)
}

fn plot_to_neighbour(host: &mut LocalSession) {
    host.command(ClientCommand::PlotDestination {
        ship: ShipId(1),
        system: SystemId(2),
        body: None,
        expected_revision: host.state.revision,
    });
}

fn position(host: &LocalSession, systems: &[StarSystem]) -> SpacePosition {
    host.state.navigation_position(ShipId(1), systems).unwrap()
}

#[test]
fn a_run_glides_ships_from_their_previous_position_to_the_new_one() {
    let (mut host, systems) = session();
    plot_to_neighbour(&mut host);
    host.request_execute_run();
    host.frame(0.0, true);
    assert_eq!(host.frame(10.0, true), None, "the first glide has finished");
    let start = position(&host, &systems);
    host.request_execute_run();
    let end = position(&host, &systems);
    assert_ne!(start, end);
    // No jump between the run and the glide's first frame.
    assert_eq!(host.display_position(ShipId(1), &systems), Some(start));
    assert_eq!(
        host.frame(20.0, true),
        Some(0.0),
        "gliding needs every frame"
    );
    assert_eq!(host.display_position(ShipId(1), &systems), Some(start));
    host.frame(20.0 + GLIDE_SECONDS / 2.0, true);
    let middle = host.display_position(ShipId(1), &systems).unwrap();
    for (a, m, b) in [
        (start.x_km, middle.x_km, end.x_km),
        (start.y_km, middle.y_km, end.y_km),
        (start.z_km, middle.z_km, end.z_km),
    ] {
        let halfway = a + (b - a) / 2;
        assert!(
            (m - halfway).abs() <= (b - a).abs() / 100 + 1,
            "{a} {m} {b}"
        );
    }
    assert_eq!(host.frame(20.0 + GLIDE_SECONDS, true), None);
    assert_eq!(host.display_position(ShipId(1), &systems), Some(end));
}

#[test]
fn with_animations_off_ships_show_their_final_positions_at_once() {
    let (mut host, systems) = session();
    plot_to_neighbour(&mut host);
    host.request_execute_run();
    assert_eq!(host.frame(5.0, false), None);
    assert_eq!(
        host.display_position(ShipId(1), &systems),
        Some(position(&host, &systems))
    );
}

#[test]
fn a_ship_arriving_at_a_body_flies_in_before_taking_its_berth_on_the_system_map() {
    let (mut host, systems) = session();
    host.command(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
        expected_revision: host.state.revision,
    });
    host.request_execute_run();
    assert!(matches!(
        host.state.ships[0].location,
        Location::Orbiting { .. }
    ));
    let sol = systems
        .iter()
        .find(|system| system.id == SystemId(1))
        .unwrap();
    host.frame(1.0, true);
    host.frame(1.0 + GLIDE_SECONDS / 2.0, true);
    let drawn = host.ships_for_map(sol, &systems);
    assert!(
        matches!(
            drawn[0].location,
            Location::InSystem {
                system: SystemId(1),
                ..
            }
        ),
        "{:?}",
        drawn[0].location
    );
    host.frame(1.0 + GLIDE_SECONDS, true);
    assert_eq!(
        host.ships_for_map(sol, &systems)[0].location,
        host.state.ships[0].location
    );
}

#[test]
fn npc_traders_glide_with_the_run() {
    let (mut host, systems) = session();
    host.command(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: host.state.revision,
    });
    let traders_at = |host: &LocalSession| -> Vec<_> {
        host.traffic()
            .iter()
            .map(|trader| {
                let ship = trader.state.ships[0].id;
                trader
                    .state
                    .navigation_position(ship, &systems)
                    .map(light_years)
            })
            .collect()
    };
    for run in 0..8 {
        let before = traders_at(&host);
        host.request_execute_run();
        let after = traders_at(&host);
        let clock = f64::from(run) * 10.0;
        host.frame(clock, true);
        for (trader, prior) in host.traffic().iter().zip(&before) {
            assert_eq!(host.trader_display_position(trader, &systems), *prior);
        }
        host.frame(clock + GLIDE_SECONDS, true);
        if before != after {
            for (trader, now) in host.traffic().iter().zip(&after) {
                assert_eq!(host.trader_display_position(trader, &systems), *now);
            }
            return;
        }
    }
    panic!("no NPC trader moved within eight runs");
}
