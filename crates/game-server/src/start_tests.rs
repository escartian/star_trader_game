//! A new game never starts with an NPC within contact reach of your fleet:
//! the first contact needs at least one Advance.
use super::{GameSetup, Session};
use game_core::{
    StarSystem, SystemId, economy::EconomyRules, generation::GalaxyConfig, movement::MovementRules,
    movement::SpacePosition, piracy::HAIL_KM, pursuit::ENGAGE_KM,
};

fn gap(a: SpacePosition, b: SpacePosition) -> i64 {
    let d = |x: i64, y: i64| (x - y) as f64;
    (d(a.x_km, b.x_km).powi(2) + d(a.y_km, b.y_km).powi(2) + d(a.z_km, b.z_km).powi(2)).sqrt()
        as i64
}

fn absolute(
    systems: &[StarSystem],
    system: SystemId,
    at: game_core::LocalPosition,
) -> SpacePosition {
    let world = systems.iter().find(|known| known.id == system).unwrap();
    let centre = SpacePosition::from_galaxy(world.position).unwrap();
    SpacePosition {
        x_km: centre.x_km + at.x_km,
        y_km: centre.y_km + at.y_km,
        z_km: centre.z_km + at.z_km,
    }
}

/// Every NPC (traders, police patrols, raider bands) within contact reach of
/// your starting fleet, as (kind, km).
fn near_start(session: &Session) -> Vec<(String, i64)> {
    let systems = game_core::orbits::placed(&session.systems, session.state.tick);
    let ours = session
        .state
        .navigation_position(session.state.ships[0].id, &systems)
        .unwrap();
    let mut near = Vec::new();
    for trader in &session.economy.traders {
        let ship = &trader.state.ships[0];
        if let Some(at) = trader.state.navigation_position(ship.id, &systems)
            && gap(at, ours) <= HAIL_KM
        {
            near.push((format!("trader {}", trader.name), gap(at, ours)));
        }
    }
    for patrol in &session.state.navigation.police.patrols {
        let at = absolute(&systems, patrol.system, patrol.position);
        if gap(at, ours) <= ENGAGE_KM {
            near.push((format!("patrol {}", patrol.id), gap(at, ours)));
        }
    }
    for band in &session.state.navigation.bands {
        let at = absolute(&systems, band.system, band.position);
        if gap(at, ours) <= ENGAGE_KM {
            near.push((format!("band {}", band.id), gap(at, ours)));
        }
    }
    near
}

#[test]
fn no_npc_starts_within_contact_reach_of_your_fleet() {
    let mut checked = 0;
    for seed in 0..12 {
        for (systems, radius_ly) in [(12, 30), (60, 70), (150, 95)] {
            let setup = GameSetup {
                galaxy: GalaxyConfig {
                    systems,
                    radius_ly,
                    ..GalaxyConfig::default()
                },
                ..GameSetup::default()
            };
            let session = Session::new_game(
                seed,
                EconomyRules::default(),
                MovementRules::default(),
                &setup,
            )
            .unwrap();
            // Nothing within reach, and no trader at the port or planet you start at.
            assert_eq!(
                near_start(&session),
                Vec::new(),
                "seed {seed}, {systems} systems"
            );
            let start = &session.state.ships[0].location;
            assert!(
                session
                    .economy
                    .traders
                    .iter()
                    .all(|trader| trader.state.ships[0].location != *start),
                "seed {seed}, {systems} systems"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 36);
    assert_eq!(near_start(&Session::default()), Vec::new());
}
