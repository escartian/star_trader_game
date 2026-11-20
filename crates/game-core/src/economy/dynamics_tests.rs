use super::{Credits, Economy, MarketDynamics, PRICE_LEVEL_UNIT, TradeSide};
use crate::fleet::ShipKind;

fn economy() -> Economy {
    let mut economy = Economy::new(&crate::demo_galaxy());
    economy.rules.dynamics.inflation_ppm_per_tick = 0;
    economy
}

/// Where a market's treasury settles, after long enough.
fn settle(economy: &mut Economy, ticks: u64) -> u64 {
    economy.advance(ticks);
    economy.markets[0].credits.0
}

#[test]
fn a_treasury_settles_where_income_meets_upkeep_from_any_starting_point() {
    let mut poor = economy();
    poor.markets[0].credits = Credits(0);
    let mut rich = economy();
    rich.markets[0].credits = Credits(100_000_000);
    let (low, high) = (settle(&mut poor, 6_000), settle(&mut rich, 6_000));
    // Same state now, same future: the starting treasury is forgotten.
    assert!(low.abs_diff(high) * 1_000 <= high, "{low} vs {high}");
    let flow = poor.market_flow(&poor.markets[0]);
    assert!(flow.net().abs() <= 1, "{flow:?}");
    assert!(flow.local_income.0 > 0 && flow.trade_income.0 == 0);
}

#[test]
fn recent_trade_raises_a_port_and_fades_when_trade_stops() {
    let mut quiet = economy();
    let idle = settle(&mut quiet, 6_000);
    let mut busy = quiet.clone();
    let port = busy.markets[0].id;
    for _ in 0..2_000 {
        busy.record_activity(port, Credits(25_000));
        busy.advance(1);
    }
    let thriving = busy.markets[0].credits.0;
    assert!(thriving > idle * 13 / 10, "{idle} -> {thriving}");
    assert!(busy.market_flow(&busy.markets[0]).trade_income.0 > 0);
    // Trade stops: activity fades, then the treasury falls back.
    let after = settle(&mut busy, 6_000);
    assert!(after.abs_diff(idle) * 100 <= idle, "{after} vs {idle}");
    assert_eq!(busy.markets[0].activity, 0);
}

#[test]
fn inflation_compounds_into_goods_services_and_hulls() {
    let mut economy = Economy::new(&crate::demo_galaxy());
    let food = economy.markets[0].goods[0].clone();
    let before = food.price(TradeSide::Buy, food.stock);
    let fuel = economy.fuel_price();
    economy.advance(400);
    let level = economy.price_level_ppm;
    assert!(level > PRICE_LEVEL_UNIT * 104 / 100 && level < PRICE_LEVEL_UNIT * 106 / 100);
    let food_now = &economy.markets[0].goods[0];
    assert_eq!(food_now.price_level_ppm, level);
    assert!(food_now.price(TradeSide::Buy, food.stock) > before);
    assert!(economy.fuel_price() > fuel);
    let catalog = economy
        .rules
        .ship_catalog
        .spec(ShipKind::Scout)
        .unwrap()
        .price;
    assert!(economy.ship_price(ShipKind::Scout).unwrap() > catalog);
    assert!(economy.charter_price() > economy.rules.campaign.charter_price);
    assert!(economy.validate_dynamics().is_ok());
    economy.markets[1].goods[2].price_level_ppm = PRICE_LEVEL_UNIT;
    assert!(economy.validate_dynamics().is_err(), "out of step");
}

#[test]
fn frozen_rules_keep_a_static_economy() {
    let mut economy = Economy::new(&crate::demo_galaxy());
    economy.rules.dynamics = MarketDynamics::frozen();
    let before = economy.markets.clone();
    economy.advance(200);
    let credits = |markets: &[super::Market]| markets.iter().map(|m| m.credits).collect::<Vec<_>>();
    assert_eq!(credits(&before), credits(&economy.markets));
    assert_eq!(economy.price_level_ppm, PRICE_LEVEL_UNIT);
}
