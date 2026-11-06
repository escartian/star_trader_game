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
