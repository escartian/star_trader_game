use super::*;
use crate::travel::{Location, TravelState};

fn economy() -> Economy {
    Economy::new(&crate::demo_galaxy())
}

#[test]
fn immediate_buy_sell_is_never_profitable_even_for_bulk_orders() {
    let mut ship = TravelState::default().ships[0].clone();
    for commodity in Commodity::ALL {
        for target in [1, 2, 10, 100] {
            for stock in [1, target, target * 2] {
                for quantity in [1, stock.min(10), stock] {
                    let mut market = economy();
                    market.credits = Credits(10_000_000);
                    ship.cargo_capacity = 200;
                    market.markets[0].goods[commodity.index()].target = target;
                    market.markets[0].goods[commodity.index()].stock = stock;
                    let original = market.clone();
                    market
                        .trade(&mut ship, commodity, quantity, TradeSide::Buy, 0)
                        .unwrap();
                    market
                        .trade(&mut ship, commodity, quantity, TradeSide::Sell, 1)
                        .unwrap();
                    assert!(market.credits < original.credits);
                    assert_eq!(ship.cargo, [0; 6]);
                    assert_eq!(market.markets[0].goods, original.markets[0].goods);
                    assert_eq!(
                        market.credits.0 + market.markets[0].credits.0,
                        original.credits.0 + original.markets[0].credits.0
                    );
                }
            }
        }
    }
}

#[test]
fn rejected_trade_and_service_operations_preserve_all_balances() {
    let mut economy = economy();
    let mut ship = TravelState::default().ships[0].clone();
    let original = economy.clone();
    assert_eq!(
        economy.trade(&mut ship, Commodity::Food, 0, TradeSide::Buy, 0),
        Err(EconomyError::InvalidQuantity)
    );
    assert_eq!(
        economy.trade(&mut ship, Commodity::Food, 21, TradeSide::Buy, 0),
        Err(EconomyError::CargoFull)
    );
    assert_eq!(
        economy.trade(&mut ship, Commodity::Food, 1, TradeSide::Sell, 0),
        Err(EconomyError::InsufficientCargo)
    );
    assert_eq!(economy.refuel(&mut ship, 1), Err(EconomyError::FuelFull));
    assert_eq!(
        economy.upgrade_hold(&mut ship),
        Err(EconomyError::InsufficientCredits)
    );
    assert_eq!(economy, original);
    assert_eq!(ship, TravelState::default().ships[0]);
}

#[test]
fn max_trade_quantity_is_the_largest_order_a_quote_accepts() {
    let food = Commodity::Food;
    let largest = |economy: &Economy, ship: &crate::travel::Ship, side| {
        let max = economy.max_trade_quantity(ship, food, side);
        if max > 0 {
            assert!(economy.quote(ship, food, max, side, 0).is_ok());
        }
        assert!(economy.quote(ship, food, max + 1, side, 0).is_err());
        max
    };
    let mut ship = TravelState::default().ships[0].clone();
    let mut economy = economy();
    economy.credits = Credits(10_000_000);
    assert_eq!(
        largest(&economy, &ship, TradeSide::Buy),
        20,
        "fills the hold"
    );
    ship.cargo_capacity = 200;
    economy.markets[0].goods[food.index()].stock = 7;
    assert_eq!(
        largest(&economy, &ship, TradeSide::Buy),
        7,
        "limited by stock"
    );
    economy.markets[0].goods[food.index()].stock = 100;
    economy.credits = Credits(2_500);
    let affordable = largest(&economy, &ship, TradeSide::Buy);
    assert!(
        (1..10).contains(&affordable),
        "limited by credits: {affordable}"
    );
    economy.credits = Credits(0);
    assert_eq!(largest(&economy, &ship, TradeSide::Buy), 0);

    ship.cargo[food.index()] = 12;
    assert_eq!(
        largest(&economy, &ship, TradeSide::Sell),
        12,
        "sells everything held"
    );
    economy.markets[0].credits = Credits(2_000);
    let funded = largest(&economy, &ship, TradeSide::Sell);
    assert!(
        (1..12).contains(&funded),
        "limited by buyer funds: {funded}"
    );
    ship.location = Location::DeepSpace(crate::movement::SpacePosition {
        x_km: 0,
        y_km: 0,
        z_km: 0,
    });
    assert_eq!(economy.max_trade_quantity(&ship, food, TradeSide::Sell), 0);
}

#[test]
fn full_refuel_fills_the_tank_as_far_as_credits_allow() {
    let mut economy = economy();
    let mut ship = TravelState::default().ships[0].clone();
    assert_eq!(economy.max_refuel_units(&ship), Err(EconomyError::FuelFull));
    ship.fuel = 10;
    let missing = ship.fuel_capacity - 10;
    assert_eq!(economy.max_refuel_units(&ship), Ok(missing));
    economy.credits = Credits(economy.rules.fuel_unit_price.0 * 3 + 1);
    assert_eq!(economy.max_refuel_units(&ship), Ok(3));
    economy.refuel(&mut ship, 3).unwrap();
    assert_eq!(
        economy.max_refuel_units(&ship),
        Err(EconomyError::InsufficientCredits)
    );
}

#[test]
fn market_time_is_batch_independent_and_price_curves_are_bounded() {
    let mut batched = economy();
    let mut stepped = batched.clone();
    batched.advance(500);
    for _ in 0..500 {
        stepped.advance(1);
    }
    assert_eq!(batched, stepped);
    for market in &batched.markets {
        for good in &market.goods {
            assert!(good.stock <= good.capacity());
            for stock in 0..=good.capacity() {
                assert!(good.price(TradeSide::Sell, stock) < good.price(TradeSide::Buy, stock));
                assert!(good.price(TradeSide::Buy, stock).0 <= good.base_price.0 * 165 / 100 + 1);
            }
        }
    }
}

#[test]
fn representative_route_pays_for_fuel_and_makes_progress_to_upgrade() {
    let mut economy = economy();
    let mut ship = TravelState::default().ships[0].clone();
    economy
        .trade(&mut ship, Commodity::Food, 20, TradeSide::Buy, 0)
        .unwrap();
    ship.fuel -= 15;
    ship.location = Location::Docked(SystemId(2));
    economy.advance(15);
    economy
        .trade(&mut ship, Commodity::Food, 20, TradeSide::Sell, 1)
        .unwrap();
    economy.refuel(&mut ship, 15).unwrap();
    assert!(economy.credits > Credits(100_000));
    assert_eq!(ship.cargo_used(), 0);
    assert_eq!(ship.fuel, 40);
    economy.credits = economy.rules.hold_upgrade_price;
    economy.upgrade_hold(&mut ship).unwrap();
    assert_eq!(ship.cargo_capacity, 40);
    assert_eq!(economy.credits, Credits(0));
    assert!(ship.upgraded);
}

#[test]
fn json_rules_and_cargo_are_validated_and_human_readable() {
    let json = serde_json::to_string(&EconomyRules::default()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["schema_version"] = 999.into();
    assert!(EconomyRules::from_json(&value.to_string()).is_err());
    value["schema_version"] = 1.into();
    value["markets"][0]["target"] = 0.into();
    assert!(EconomyRules::from_json(&value.to_string()).is_err());
    let mut ship = TravelState::default().ships[0].clone();
    ship.cargo[Commodity::Medicine.index()] = 3;
    let json = serde_json::to_value(&ship).unwrap();
    assert_eq!(json["cargo"]["medicine"], 3);
    assert_eq!(
        serde_json::from_value::<crate::travel::Ship>(json).unwrap(),
        ship
    );
}
