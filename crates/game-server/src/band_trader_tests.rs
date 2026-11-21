//! Raider bands stalking and striking NPC traders (`npc::band_raids`).
use super::{Session, combat_tests::reload, test_support::execute_quiet_run};
use game_core::{
    LocalPosition, SystemId,
    economy::{Commodity, Credits},
    fleet::ShipKind,
    navigation::space_position,
    npc::market_location,
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage};

/// Trader 0 (cautious) stranded in orbit at a planet market in system 2 with
/// 10 ore it cannot sell for fuel money in time, and system 2's band, of
/// `ships`, 2 million km away. The player's
/// freighter waits docked at home, out of the bands' reach.
fn stalked_trader(ships: Vec<ShipKind>) -> Session {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 0;
    let market = session
        .economy
        .markets
        .iter()
        .find(|market| !market.is_station && market.id.system == SystemId(2))
        .expect("system 2 has a planet market")
        .id;
    let at = market_location(market, &session.systems);
    let trader = &mut session.economy.traders[0];
    trader.state.navigation.orders.clear();
    trader.state.ships[0].location = at.clone();
    trader.state.ships[0].cargo = [0; 6];
    trader.state.ships[0].cargo[Commodity::Ore.index()] = 10;
    trader.state.ships[0].fuel = 0;
    trader.credits = Credits(0);
    let world = session
        .systems
        .iter()
        .find(|s| s.id == SystemId(2))
        .unwrap();
    let centre = game_core::movement::SpacePosition::from_galaxy(world.position).unwrap();
    let place = space_position(&at, &session.systems).unwrap();
    let band = &mut session.state.navigation.bands[0];
    assert_eq!(band.system, SystemId(2));
    band.position = LocalPosition {
        x_km: place.x_km - centre.x_km + 2_000_000,
        y_km: 0,
        z_km: place.z_km - centre.z_km,
    };
    band.heading = band.position;
    band.ships = ships;
    assert!(matches!(
        session.state.ships[0].location,
        Location::Docked(_)
    ));
    session
}

fn run(session: &mut Session) {
    let fleet = session.state.fleets[0].id;
    session.handle(ClientCommand::HoldFleet {
        fleet,
        expected_revision: session.state.revision,
    });
    let result = session.handle(ClientCommand::ExecuteNavigationRun {
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(result, ServerMessage::SessionSnapshot { .. }),
        "{result:?}"
    );
}

#[test]
fn a_band_fights_a_trader_it_cannot_cow_and_then_lies_low() {
    // Two scouts (90) against a freighter (70): short of a cautious 150%.
    let mut session = stalked_trader(vec![ShipKind::Scout; 2]);
    let ships = 2;
    run(&mut session);
    let news = &session.economy.traders[0].last_action;
    assert!(
        news.contains("a raider band") && !news.starts_with("Handed"),
        "{news}"
    );
    let band = session
        .state
        .navigation
        .bands
        .iter()
        .find(|b| b.system == SystemId(2));
    if let Some(band) = band {
        assert!(band.ships.len() <= ships);
        assert_eq!(band.cooldown, game_core::navigation::LOOT_COOLDOWN_RUNS);
        assert_eq!(band.stalking, None);
    }
    reload(&session);
}

#[test]
fn an_outgunned_trader_hands_half_its_cargo_to_a_band_without_a_fight() {
    let mut session = stalked_trader(vec![ShipKind::Patrol, ShipKind::Scout]);
    run(&mut session);
    let trader = &session.economy.traders[0];
    assert!(
        trader.last_action.starts_with("Handed"),
        "{}",
        trader.last_action
    );
    assert_eq!(trader.state.ships[0].hull_damage, 0, "no shots fired");
    let band = &session.state.navigation.bands[0];
    assert_eq!(band.ships, vec![ShipKind::Patrol, ShipKind::Scout]);
    assert_eq!(band.cooldown, game_core::navigation::LOOT_COOLDOWN_RUNS);
    reload(&session);
}

#[test]
fn bands_stalk_traders_but_never_strike_in_a_peaceful_game() {
    let mut session = stalked_trader(vec![ShipKind::Scout; 2]);
    // Quiet runs set the contact chance to 0 while they execute.
    execute_quiet_run(&mut session);
    let band = &session.state.navigation.bands[0];
    assert_eq!(band.stalking, Some(session.economy.traders[0].id));
    assert_eq!(band.hunting, None);
    assert!(!session.economy.traders[0].last_action.contains("raider"));
    reload(&session);
}

#[test]
fn a_nearer_player_fleet_draws_the_band_off_the_trader() {
    let mut session = stalked_trader(vec![ShipKind::Scout; 2]);
    let band = session.state.navigation.bands[0].clone();
    session.state.ships[0].location = Location::InSystem {
        system: SystemId(2),
        position: band.position,
    };
    run(&mut session);
    let band = &session.state.navigation.bands[0];
    assert_eq!(band.stalking, None);
    assert!(!session.economy.traders[0].last_action.contains("raider"));
    assert!(
        session
            .state
            .navigation
            .encounters
            .iter()
            .any(|c| c.band.is_some())
            || band.hunting.is_some()
    );
}
