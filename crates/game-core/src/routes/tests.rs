use super::{TradeRoute, cancel_route, cargo, service_routes, start_route};
use crate::{
    StarSystem, SystemId,
    economy::{Commodity, Credits, Economy, MarketId, TradeSide},
    navigation::{NavigationError, NavigationEvent},
    travel::{Location, TravelState},
};

struct World {
    systems: Vec<StarSystem>,
    economy: Economy,
    state: TravelState,
}

fn world() -> World {
    let systems = crate::demo_galaxy();
    let economy = Economy::new(&systems);
    let mut state = TravelState::default();
    state.movement_rules.encounter_chance_percent = 0;
    World {
        systems,
        economy,
        state,
    }
}

fn station(economy: &Economy, system: u64) -> MarketId {
    economy
        .markets
        .iter()
        .find(|market| market.id.system == SystemId(system) && market.is_station)
        .unwrap()
        .id
}

#[test]
fn cargo_plan_buys_only_profitable_units_within_space_and_credits() {
    let economy = world().economy;
    let (sol, meridian) = (station(&economy, 1), station(&economy, 2));
    let here = economy.market(sol).unwrap();
    let there = economy.market(meridian).unwrap();
    let plan = cargo::plan(here, there, 30, 10_000_000);
    assert!(plan.iter().sum::<u32>() <= 30);
    for good in Commodity::ALL {
        if plan[good.index()] > 0 {
            let local = &here.goods[good.index()];
            let remote = &there.goods[good.index()];
            assert!(
                remote.price(TradeSide::Sell, remote.stock)
                    > local.price(TradeSide::Buy, local.stock),
                "{good:?} has no margin"
            );
        }
    }
    assert_eq!(cargo::plan(here, there, 30, 0), [0; 6], "no credits");
    assert_eq!(cargo::plan(here, there, 0, 10_000_000), [0; 6], "no space");
    // Selling back where you bought never pays: buy prices exceed sell prices.
    assert_eq!(cargo::plan(here, here, 30, 10_000_000), [0; 6]);
}

#[test]
fn starting_a_route_keeps_cargo_aboard_buys_for_the_far_end_and_plots_it() {
    let mut world = world();
    let (sol, meridian) = (station(&world.economy, 1), station(&world.economy, 2));
    let fleet = world.state.fleets[0].id;
    let player = world.state.ships[0].owner;
    world.state.ships[0].cargo[Commodity::Medicine.index()] = 2;
    let event = start_route(
        &mut world.state,
        &mut world.economy,
        player,
        fleet,
        meridian,
        &world.systems,
    )
    .unwrap();
    assert!(matches!(event, NavigationEvent::RouteServiced { .. }));
    assert_eq!(
        world.state.trade_route(fleet).unwrap().stops,
        [sol, meridian]
    );
    assert!(world.state.ships[0].cargo[Commodity::Medicine.index()] >= 2);
    assert!(world.state.navigation.order(fleet).is_some());
    assert_eq!(
        start_route(
            &mut world.state,
            &mut world.economy,
            player,
            fleet,
            sol,
            &world.systems,
        ),
        Err(NavigationError::InvalidRouteStop.into()),
        "the far end must differ from the market the fleet is at"
    );
    assert!(cancel_route(&mut world.state, player, fleet).is_ok());
    assert!(world.state.trade_route(fleet).is_none());
    assert_eq!(
        cancel_route(&mut world.state, player, fleet),
        Err(NavigationError::NoRoute.into())
    );
}

#[test]
fn a_route_fleet_trades_at_each_end_and_keeps_looping() {
    let mut world = world();
    let meridian = station(&world.economy, 2);
    let fleet = world.state.fleets[0].id;
    let player = world.state.ships[0].owner;
    start_route(
        &mut world.state,
        &mut world.economy,
        player,
        fleet,
        meridian,
        &world.systems,
    )
    .unwrap();
    let mut stops = Vec::new();
    for _ in 0..60 {
        world
            .state
            .execute_navigation_run(player, &world.systems)
            .unwrap();
        let events =
            service_routes(&mut world.state, &mut world.economy, player, &world.systems).unwrap();
        for event in events {
            match event {
                NavigationEvent::RouteServiced { .. } => {
                    stops.push(world.state.ships[0].location.system().unwrap());
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!(
            world.state.navigation.order(fleet).is_some(),
            "always underway"
        );
        if stops.len() == 2 {
            break;
        }
    }
    assert_eq!(
        stops,
        [SystemId(2), SystemId(1)],
        "Meridian, then back to Sol"
    );
    assert!(world.state.trade_route(fleet).is_some());
}

#[test]
fn a_stop_that_cannot_continue_ends_the_route_and_needs_orders() {
    let mut world = world();
    let (sol, meridian) = (station(&world.economy, 1), station(&world.economy, 2));
    let fleet = world.state.fleets[0].id;
    let player = world.state.ships[0].owner;
    world.state.navigation.routes.push(TradeRoute {
        fleet,
        stops: [sol, meridian],
        paused: false,
    });
    world.state.ships[0].fuel = 0;
    world.economy.credits = Credits(0);
    let events =
        service_routes(&mut world.state, &mut world.economy, player, &world.systems).unwrap();
    assert!(
        matches!(&events[..], [NavigationEvent::RouteEnded { reason, .. }] if reason.starts_with("Route ended")),
        "{events:?}"
    );
    assert!(world.state.navigation.routes.is_empty());
    assert_eq!(world.state.navigation_attention(player), Some(fleet));
}

#[test]
fn a_route_fleet_away_from_its_stops_leaves_the_route() {
    let mut world = world();
    let (meridian, far_reach) = (station(&world.economy, 2), station(&world.economy, 3));
    let fleet = world.state.fleets[0].id;
    let player = world.state.ships[0].owner;
    world.state.navigation.routes.push(TradeRoute {
        fleet,
        stops: [meridian, far_reach],
        paused: false,
    });
    assert_eq!(world.state.ships[0].location, Location::Docked(SystemId(1)));
    let events =
        service_routes(&mut world.state, &mut world.economy, player, &world.systems).unwrap();
    assert!(matches!(&events[..], [NavigationEvent::RouteEnded { .. }]));
    assert!(world.state.navigation.routes.is_empty());
}

#[test]
fn removed_fleets_lose_their_routes_and_saved_routes_are_validated() {
    let mut world = world();
    let (sol, meridian) = (station(&world.economy, 1), station(&world.economy, 2));
    let fleet = world.state.fleets[0].id;
    world.state.navigation.routes.push(TradeRoute {
        fleet,
        stops: [sol, meridian],
        paused: false,
    });
    world.state.validate_navigation(&world.systems).unwrap();
    let mut duplicate = world.state.clone();
    duplicate
        .navigation
        .routes
        .push(duplicate.navigation.routes[0].clone());
    assert!(duplicate.validate_navigation(&world.systems).is_err());
    let mut looping = world.state.clone();
    looping.navigation.routes[0].stops = [sol, sol];
    assert!(looping.validate_navigation(&world.systems).is_err());
    world.state.fleets.clear();
    world.state.prune_standing_orders();
    assert!(world.state.navigation.routes.is_empty());
}

#[test]
fn nothing_profitable_pauses_the_route_until_a_resume_finds_cargo() {
    let mut world = world();
    let (sol, meridian) = (station(&world.economy, 1), station(&world.economy, 2));
    let fleet = world.state.fleets[0].id;
    let player = world.state.ships[0].owner;
    world.state.navigation.routes.push(TradeRoute {
        fleet,
        stops: [sol, meridian],
        paused: false,
    });
    // A far market that cannot pay makes every cargo unprofitable.
    let funds = world.economy.market(meridian).unwrap().credits;
    world.economy.market_mut(meridian).unwrap().credits = Credits(0);
    let events =
        service_routes(&mut world.state, &mut world.economy, player, &world.systems).unwrap();
    assert!(
        matches!(&events[..], [NavigationEvent::RoutePaused { reason, .. }] if reason.contains("paused")),
        "{events:?}"
    );
    assert!(world.state.trade_route(fleet).unwrap().paused);
    assert_eq!(world.state.ships[0].cargo_used(), 0);
    assert_eq!(world.state.navigation_attention(player), Some(fleet));
    let again =
        service_routes(&mut world.state, &mut world.economy, player, &world.systems).unwrap();
    assert!(again.is_empty(), "a paused route waits for the player");
    let resume = |world: &mut World| {
        start_route(
            &mut world.state,
            &mut world.economy,
            player,
            fleet,
            meridian,
            &world.systems,
        )
    };
    assert_eq!(
        resume(&mut world),
        Err(NavigationError::NothingProfitable.into())
    );
    world.economy.market_mut(meridian).unwrap().credits = funds;
    assert!(resume(&mut world).is_ok());
    assert!(!world.state.trade_route(fleet).unwrap().paused);
    assert!(world.state.navigation.order(fleet).is_some());
}
