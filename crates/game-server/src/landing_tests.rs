use super::{
    Session,
    combat_tests::reload,
    navigation_tests::{plot, run, succeed},
};
use game_core::{
    BodyId, SystemId,
    economy::Commodity,
    fleet::FleetId,
    movement::MoveTarget,
    navigation::{NavigationError, NavigationEvent},
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

const INNER_WORLD: BodyId = BodyId(1);
const GAS_GIANT: BodyId = BodyId(2);
const ASTEROID: BodyId = BodyId(5);
const MOON: BodyId = BodyId(4);

/// The starting freighter, flown into orbit of `body` in Sol without contacts.
fn orbiting(body: BodyId) -> Session {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    plot(
        &mut session,
        ShipId(1),
        MoveTarget::Body {
            system: SystemId(1),
            body,
        },
    );
    while session.state.navigation.order(FleetId(1)).is_some() {
        run(&mut session);
    }
    assert_eq!(
        session.state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body
        }
    );
    session
}

fn land(session: &mut Session) -> ServerMessage {
    session.handle(ClientCommand::Land {
        fleet: FleetId(1),
        expected_revision: session.state.revision,
    })
}

fn lift_off(session: &mut Session) -> ServerMessage {
    session.handle(ClientCommand::LiftOff {
        fleet: FleetId(1),
        expected_revision: session.state.revision,
    })
}

fn rejected(response: ServerMessage, error: NavigationError) {
    assert_eq!(
        response,
        ServerMessage::CommandRejected(CommandError::Navigation(error))
    );
}

#[test]
fn landing_and_lifting_off_cost_fuel_by_gravity() {
    let mut session = orbiting(MOON);
    let fuel = session.state.ships[0].fuel;
    land(&mut session);
    assert_eq!(
        session.state.ships[0].location,
        Location::Landed {
            system: SystemId(1),
            body: MOON
        }
    );
    assert_eq!(session.state.ships[0].fuel, fuel - 1, "moon: 1 fuel");
    reload(&session);
    rejected(land(&mut session), NavigationError::NotInOrbit);
    lift_off(&mut session);
    assert_eq!(session.state.ships[0].fuel, fuel - 2, "and 1 to lift off");
    rejected(lift_off(&mut session), NavigationError::NotLanded);
    let mut giant = orbiting(GAS_GIANT);
    let before = giant.snapshot(Vec::new());
    rejected(land(&mut giant), NavigationError::CannotLand);
    assert_eq!(giant.snapshot(Vec::new()), before);
    // The home port is a station: docked, not in orbit.
    rejected(land(&mut Session::default()), NavigationError::NotInOrbit);
}

#[test]
fn a_course_from_the_surface_includes_the_lift_off_and_planet_markets_stay_open() {
    let mut session = orbiting(INNER_WORLD);
    let target = MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(3),
    };
    let preview = |session: &mut Session| {
        let ServerMessage::MovementPreview(plan) = session.handle(ClientCommand::PreviewMove {
            ship: ShipId(1),
            target: target.clone(),
        }) else {
            panic!("no preview");
        };
        plan.fuel_per_ship
    };
    let from_orbit = preview(&mut session);
    land(&mut session);
    assert!(
        session
            .economy
            .current_market(&session.state.ships[0])
            .is_ok(),
        "the planet market serves landed fleets"
    );
    assert_eq!(
        preview(&mut session),
        from_orbit + 2,
        "rocky planet: 2 fuel"
    );
    plot(&mut session, ShipId(1), target);
    while session.state.navigation.order(FleetId(1)).is_some() {
        run(&mut session);
    }
    assert_eq!(
        session.state.ships[0].location,
        Location::Docked(SystemId(1))
    );
}

#[test]
fn a_waiting_fleet_on_an_asteroid_mines_ore_until_its_holds_are_full() {
    let mut session = orbiting(ASTEROID);
    land(&mut session);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    let capacity = session.state.ships[0].cargo_capacity;
    let mut mined = Vec::new();
    for _ in 0..12 {
        let ServerMessage::SessionSnapshot { events, .. } =
            session.handle(ClientCommand::ExecuteNavigationRun {
                expected_revision: session.state.revision,
            })
        else {
            panic!("run rejected");
        };
        for event in events {
            if let SessionEvent::Navigation(NavigationEvent::Mined { ore, .. }) = event {
                mined.push(ore);
            }
        }
    }
    let ore = session.state.ships[0].cargo[Commodity::Ore.index()];
    assert_eq!(ore, capacity, "mined until the hold was full");
    // Asteroids mine 2 ore per ship per run, scaled by their surface.
    let surface = session.systems[0]
        .bodies
        .iter()
        .find(|body| body.id == ASTEROID)
        .and_then(|body| body.surface.clone())
        .expect("asteroids have a surface");
    assert_eq!(mined[0], surface.ore_per_ship(2));
    assert_eq!(*mined.last().unwrap(), 0, "full holds mine nothing");
    reload(&session);
}

#[test]
fn landing_waits_for_courses_and_contacts() {
    let mut session = orbiting(MOON);
    plot(
        &mut session,
        ShipId(1),
        MoveTarget::Body {
            system: SystemId(1),
            body: INNER_WORLD,
        },
    );
    rejected(land(&mut session), NavigationError::FleetHasCourse);
}

#[test]
fn richer_surfaces_mine_more_ore_and_older_saves_get_surfaces_on_load() {
    use game_core::planet::{Biome, SurfaceProfile};
    let mut session = orbiting(ASTEROID);
    let asteroid = session.systems[0]
        .bodies
        .iter_mut()
        .find(|body| body.id == ASTEROID)
        .unwrap();
    asteroid.surface = Some(SurfaceProfile {
        biome: Biome::Metallic,
        danger: game_core::planet::Danger::Dangerous,
        ore_percent: 200,
    });
    land(&mut session);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    run(&mut session);
    assert_eq!(session.state.ships[0].cargo[Commodity::Ore.index()], 4);
    // A save from before surfaces: every moon and asteroid gets one on load.
    let expected = Session::default().systems.clone();
    let mut value: serde_json::Value =
        serde_json::from_str(&Session::default().export_json().unwrap()).unwrap();
    for system in value["systems"].as_array_mut().unwrap() {
        for body in system["bodies"].as_array_mut().unwrap() {
            body.as_object_mut().unwrap().remove("surface");
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("old.json");
    std::fs::write(&path, value.to_string()).unwrap();
    let (loaded, _) = Session::load(&path).unwrap();
    for (system, original) in loaded.systems.iter().zip(&expected) {
        for (body, before) in system.bodies.iter().zip(&original.bodies) {
            assert_eq!(body.surface, before.surface, "{}", body.name);
        }
    }
}

#[test]
fn surfaces_sit_only_on_moons_and_asteroids_in_saves() {
    let session = Session::default();
    let mut forged = session.clone();
    let surface = forged.systems[0]
        .bodies
        .iter()
        .find_map(|body| body.surface.clone());
    let planet = forged.systems[0]
        .bodies
        .iter_mut()
        .find(|body| body.id == INNER_WORLD)
        .unwrap();
    planet.surface = surface;
    assert!(forged.export_json().is_err());
}
