use super::{
    Credits, Economy, MarketDynamics, PRICE_LEVEL_UNIT, TradeSide,
    dynamics::{ACTIVITY_PER_POINT, MAX_SIZE_PERCENT},
};
use crate::fleet::ShipKind;

fn economy() -> Economy {
    Economy::new(&crate::demo_galaxy())
}

#[test]
fn idle_ports_shrink_and_busy_ports_grow_one_point_per_tick_within_bounds() {
    let mut economy = economy();
    let busy = economy.markets[0].id;
    economy.advance(10);
    assert!(
        economy
            .markets
            .iter()
            .all(|market| market.size_percent == 90)
    );
    economy.advance(100);
    assert!(
        economy
            .markets
            .iter()
            .skip(1)
            .all(|market| market.size_percent == 75)
    );
    // Heavy, steady trade at one port.
    for _ in 0..600 {
        economy.record_activity(busy, Credits(ACTIVITY_PER_POINT * 20));
        economy.advance(1);
    }
    let size = economy.market(busy).unwrap().size_percent;
    assert!(size > 200 && size <= MAX_SIZE_PERCENT, "{size}");
    assert!(economy.validate_dynamics().is_ok());
}

#[test]
fn treasuries_drift_toward_founding_size_and_price_level() {
    let mut economy = economy();
    economy.rules.dynamics.inflation_ppm_per_tick = 0;
    let founding = economy.markets[0].founding_credits.0;
    economy.markets[0].credits = Credits(0);
    // 2% of the gap per tick: an exponential approach, then a cent at a time.
    economy.advance(1_000);
    // Idle: 75% of the founding treasury, reached within a cent.
    let settled = economy.markets[0].credits.0;
    assert!(settled.abs_diff(founding * 75 / 100) <= 1, "{settled}");
    economy.markets[0].credits = Credits(founding * 10);
    economy.advance(1);
    assert!(
        economy.markets[0].credits.0 < founding * 10,
        "a rich port pays out"
    );
}

#[test]
fn inflation_compounds_into_goods_services_hulls_and_treasuries() {
    let mut economy = economy();
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
    // Out-of-step price levels are rejected.
    economy.markets[1].goods[2].price_level_ppm = PRICE_LEVEL_UNIT;
    assert!(economy.validate_dynamics().is_err());
}

#[test]
fn frozen_rules_keep_the_static_economy_and_old_markets_adopt_their_treasury() {
    let mut economy = economy();
    economy.rules.dynamics = MarketDynamics::frozen();
    let credits: Vec<_> = economy
        .markets
        .iter()
        .map(|market| market.credits)
        .collect();
    economy.advance(200);
    let after: Vec<_> = economy
        .markets
        .iter()
        .map(|market| market.credits)
        .collect();
    assert_eq!(credits, after);
    assert_eq!(economy.price_level_ppm, PRICE_LEVEL_UNIT);
    // A market from before living markets has no founding treasury yet.
    let mut json = serde_json::to_value(&economy.markets[0]).unwrap();
    let fields = json.as_object_mut().unwrap();
    for field in ["founding_credits", "size_percent", "activity"] {
        fields.remove(field);
    }
    let old: super::Market = serde_json::from_value(json).unwrap();
    assert_eq!((old.founding_credits, old.size_percent), (Credits(0), 100));
    economy.markets[0] = old;
    economy.advance(1);
    assert_eq!(economy.markets[0].founding_credits, credits[0]);
}
