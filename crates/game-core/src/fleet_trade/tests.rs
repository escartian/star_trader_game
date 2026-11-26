//! Splitting a fleet order among its holds.
use super::*;
use crate::economy::{Commodity, Economy, EconomyRules, Market};
use crate::fleet::ShipKind;
use crate::travel::{Location, PlayerId, TravelState};

const PLAYER: PlayerId = PlayerId(1);
const GOOD: Commodity = Commodity::Food;

fn at(market: &Market) -> Location {
    if market.is_station {
        Location::Docked(market.id.system)
    } else {
        Location::Orbiting {
            system: market.id.system,
            body: market.id.body,
        }
    }
}

/// The starting ship plus two more in its fleet at the first market, with
/// holds of 20, 40 and 10 units: ships 0, 1 and 2.
fn fleet() -> (TravelState, Economy, Vec<ShipId>) {
    let rules = EconomyRules::default();
    let mut systems = crate::generation::generate_galaxy(3);
    rules.planet_catalog.populate(&mut systems);
    let mut economy = Economy::with_rules(&systems, rules).unwrap();
    economy.credits = crate::economy::Credits(100_000_000);
    let mut state = TravelState::default();
    let market = economy.markets[0].clone();
    state.ships[0].location = at(&market);
    let fleet = state.fleets[0].id;
    for kind in [ShipKind::Freighter, ShipKind::Scout] {
        let spec = economy.rules.ship_catalog.spec(kind).unwrap().clone();
        state.add_ship(PLAYER, fleet, &spec).unwrap();
    }
    for (ship, capacity) in state.ships.iter_mut().zip([20, 40, 10]) {
        ship.cargo = [0; 6];
        ship.cargo_capacity = capacity;
    }
    let ids = state.ships.iter().map(|ship| ship.id).collect();
    (state, economy, ids)
}

/// Prices and applies an order like the host does; returns the split.
fn trade(
    state: &mut TravelState,
    economy: &mut Economy,
    quantity: u32,
    side: TradeSide,
    order: LoadOrder,
) -> Vec<(ShipId, u32)> {
    let lead = state.ships[0].id;
    let ships = partners(state, economy, lead, order).unwrap();
    let mut pool = pooled(state, &ships).unwrap();
    economy.trade(&mut pool, GOOD, quantity, side, 0).unwrap();
    let catalog = economy.rules.ship_catalog.clone();
    let split = plan(state, &catalog, &ships, GOOD, quantity, side, order);
    apply(state, &split, GOOD, side).unwrap();
    split
}

fn held(state: &TravelState) -> Vec<u32> {
    state
        .ships
        .iter()
        .map(|ship| ship.cargo[GOOD.index()])
        .collect()
}

#[test]
fn the_fleet_at_a_port_trades_as_one_pooled_hold() {
    let (mut state, economy, ids) = fleet();
    assert_eq!(
        partners(&state, &economy, ids[0], LoadOrder::Even).unwrap(),
        ids
    );
    assert_eq!(
        partners(&state, &economy, ids[0], LoadOrder::Selected).unwrap(),
        vec![ids[0]]
    );
    state.ships[1].cargo[Commodity::Ore.index()] = 5;
    let pool = pooled(&state, &ids).unwrap();
    assert_eq!(pool.cargo_capacity, 70);
    assert_eq!(pool.cargo[Commodity::Ore.index()], 5);
    // A ship elsewhere does not share the order.
    state.ships[2].location = Location::Docked(crate::SystemId(9_999));
    assert_eq!(
        partners(&state, &economy, ids[0], LoadOrder::Even).unwrap(),
        ids[..2]
    );
}

#[test]
fn an_even_spread_fills_every_hold_to_the_same_share() {
    let (mut state, mut economy, _) = fleet();
    let stock = economy.markets[0].goods[GOOD.index()].stock;
    let quantity = 35.min(stock);
    trade(
        &mut state,
        &mut economy,
        quantity,
        TradeSide::Buy,
        LoadOrder::Even,
    );
    let held = held(&state);
    assert_eq!(held.iter().sum::<u32>(), quantity);
    // Within a unit of each hold's proportional share.
    for (units, capacity) in held.iter().zip([20, 40, 10]) {
        let share = f64::from(quantity) * f64::from(capacity) / 70.0;
        assert!((f64::from(*units) - share).abs() <= 1.0, "{held:?}");
    }
    // Selling evenly drains the fullest share first, keeping it even.
    trade(
        &mut state,
        &mut economy,
        quantity / 2,
        TradeSide::Sell,
        LoadOrder::Even,
    );
    let after = self::held(&state);
    assert_eq!(after.iter().sum::<u32>(), quantity - quantity / 2);
    for (units, capacity) in after.iter().zip([20, 40, 10]) {
        let share = f64::from(quantity - quantity / 2) * f64::from(capacity) / 70.0;
        assert!((f64::from(*units) - share).abs() <= 1.0, "{after:?}");
    }
}

#[test]
fn largest_first_fills_the_biggest_hold_and_sells_from_the_smallest() {
    let (mut state, mut economy, ids) = fleet();
    let split = trade(
        &mut state,
        &mut economy,
        45,
        TradeSide::Buy,
        LoadOrder::Largest,
    );
    assert_eq!(split, vec![(ids[1], 40), (ids[0], 5)]);
    let split = trade(
        &mut state,
        &mut economy,
        10,
        TradeSide::Sell,
        LoadOrder::Largest,
    );
    assert_eq!(split, vec![(ids[0], 5), (ids[1], 5)]);
    assert_eq!(held(&state), vec![0, 35, 0]);
}

#[test]
fn toughest_first_loads_the_ship_with_the_most_hull_and_shields_left() {
    let (mut state, mut economy, ids) = fleet();
    let catalog = economy.rules.ship_catalog.clone();
    let mut ranked = ids.clone();
    ranked.sort_by_key(|id| std::cmp::Reverse(toughness(state.ship(*id).unwrap(), &catalog)));
    let toughest = ranked[0];
    let room = state.ship(toughest).unwrap().cargo_capacity;
    let split = trade(
        &mut state,
        &mut economy,
        room,
        TradeSide::Buy,
        LoadOrder::Toughest,
    );
    assert_eq!(split, vec![(toughest, room)]);
    // Battle damage can change the order.
    let ship = state.ship_mut(toughest).unwrap();
    ship.hull_damage = catalog.spec(ship.kind).unwrap().combat.hull;
    let split = plan(
        &state,
        &catalog,
        &ids,
        GOOD,
        1,
        TradeSide::Sell,
        LoadOrder::Toughest,
    );
    assert_eq!(split, vec![(toughest, 1)], "the wreck sells first");
}

#[test]
fn a_plan_that_does_not_fit_is_refused() {
    let (mut state, _, ids) = fleet();
    assert_eq!(
        apply(&mut state, &[(ids[2], 11)], GOOD, TradeSide::Buy),
        Err(EconomyError::CargoFull)
    );
    assert_eq!(
        apply(&mut state, &[(ids[2], 1)], GOOD, TradeSide::Sell),
        Err(EconomyError::InsufficientCargo)
    );
}
