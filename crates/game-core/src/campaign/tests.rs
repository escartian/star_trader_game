use super::{CampaignEnd, Road, check_end, net_worth, progress, score};
use crate::economy::{Commodity, Credits, Economy, EconomyError, MarketId, TradeSide};
use crate::navigation::{GARRISON, haven_site};
use crate::travel::{PlayerId, TravelState};
use crate::{BodyKind, StarSystem, generation::generate_galaxy};

const PLAYER: PlayerId = PlayerId(1);

fn world() -> (Vec<StarSystem>, TravelState, Economy) {
    let mut systems = generate_galaxy(0);
    let rules = crate::economy::EconomyRules::default();
    rules.planet_catalog.populate(&mut systems);
    let economy = Economy::with_rules(&systems, rules).unwrap();
    let mut state = TravelState::default();
    state.spawn_raider_bands(&systems);
    (systems, state, economy)
}

fn total_money(economy: &Economy) -> u64 {
    economy.credits.0
        + economy
            .markets
            .iter()
            .map(|market| market.credits.0)
            .sum::<u64>()
        + economy
            .traders
            .iter()
            .map(|trader| trader.credits.0)
            .sum::<u64>()
}

#[test]
fn net_worth_counts_credits_hulls_at_resale_and_cargo_at_the_best_bid() {
    let (_, mut state, economy) = world();
    let freighter = economy
        .rules
        .ship_catalog
        .spec(state.ships[0].kind)
        .unwrap();
    let hull = freighter.price.0 * u64::from(economy.rules.ship_resale_percent) / 100;
    assert_eq!(
        net_worth(&state, &economy, PLAYER).0,
        economy.credits.0 + hull
    );
    state.ships[0].cargo[Commodity::Ore.index()] = 3;
    let best = economy
        .markets
        .iter()
        .map(|market| {
            let ore = &market.goods[Commodity::Ore.index()];
            ore.price(TradeSide::Sell, ore.stock).0
        })
        .max()
        .unwrap();
    assert_eq!(
        net_worth(&state, &economy, PLAYER).0,
        economy.credits.0 + hull + 3 * best
    );
}

#[test]
fn the_trader_road_is_won_by_net_worth_or_by_charters_and_recorded_once() {
    let (_, mut state, mut economy) = world();
    assert_eq!(check_end(&state, &economy, PLAYER), None);
    let roads = progress(&state, &economy, PLAYER);
    assert_eq!(
        roads.iter().map(|road| road.road).collect::<Vec<_>>(),
        Road::ALL
    );
    assert!(roads[0].open && roads[1].open && !roads[2].open && !roads[3].open);
    // Just under 10% of the galaxy's wealth, then just over it.
    let others =
        super::galaxy_wealth(&state, &economy, PLAYER).0 - net_worth(&state, &economy, PLAYER).0;
    let hull = net_worth(&state, &economy, PLAYER).0 - economy.credits.0;
    economy.credits = Credits(others / 9 - hull - 100);
    assert_eq!(check_end(&state, &economy, PLAYER), None);
    assert!(progress(&state, &economy, PLAYER)[0].percent >= 99);
    economy.credits = Credits(others / 9 - hull + 100);
    assert_eq!(
        super::wealth_share_basis_points(&state, &economy, PLAYER),
        1000
    );
    state.navigation.run = 9;
    let won = check_end(&state, &economy, PLAYER);
    assert_eq!(
        won,
        Some(CampaignEnd::Won {
            road: Road::Trader,
            run: 9
        })
    );
    state.campaign.ended = won;
    assert_eq!(check_end(&state, &economy, PLAYER), None, "recorded once");
    // Charters alone complete the road too.
    let (_, state, mut economy) = world();
    let markets: Vec<MarketId> = economy.markets.iter().take(3).map(|m| m.id).collect();
    for market in markets {
        economy.charters.push(crate::economy::Charter {
            market,
            owner: PLAYER,
        });
    }
    assert!(progress(&state, &economy, PLAYER)[0].complete);
    assert_eq!(
        score(&state, &economy, PLAYER).charters,
        3 * super::CHARTER_POINTS
    );
}

#[test]
fn a_run_limit_ends_the_campaign_on_score() {
    let (_, mut state, economy) = world();
    state.campaign.run_limit = Some(10);
    state.navigation.run = 9;
    assert_eq!(check_end(&state, &economy, PLAYER), None);
    state.navigation.run = 10;
    assert_eq!(
        check_end(&state, &economy, PLAYER),
        Some(CampaignEnd::TimeUp { run: 10 })
    );
    assert!(!CampaignEnd::TimeUp { run: 10 }.playable());
    assert!(
        CampaignEnd::Won {
            road: Road::Soldier,
            run: 1
        }
        .playable()
    );
}

#[test]
fn the_haven_stands_in_the_farthest_system_and_falls_with_its_garrison() {
    let (systems, mut state, economy) = world();
    let (system, body) = haven_site(&systems).unwrap();
    let home = systems[0].position;
    let gap = |s: &StarSystem| (s.position.x - home.x).pow(2) + (s.position.z - home.z).pow(2);
    let farthest = systems.iter().skip(1).map(gap).max().unwrap();
    let site = systems.iter().find(|s| s.id == system).unwrap();
    assert_eq!(gap(site), farthest);
    let anchor = site.bodies.iter().find(|b| b.id == body).unwrap();
    assert_ne!(anchor.kind, BodyKind::Station);
    let haven = state.navigation.haven.clone().unwrap();
    assert_eq!((haven.system, haven.body), (system, body));
    assert_eq!(haven.garrison, GARRISON);
    assert!(state.validate_navigation(&systems).is_ok());
    // Losses stay; the last garrison ship's loss destroys the Haven.
    state.navigation.run = 4;
    state.tick = 4 * state.movement_rules.ticks_per_navigation_run;
    state.haven_after_contact(&GARRISON[..3]);
    let haven = state.navigation.haven.clone().unwrap();
    assert_eq!((haven.garrison.len(), haven.sunk), (5, 3));
    assert!(haven.standing());
    assert_eq!(
        score(&state, &economy, PLAYER).haven,
        3 * super::HAVEN_SHIP_POINTS
    );
    state.haven_after_contact(&GARRISON[3..]);
    assert_eq!(
        state.navigation.haven.as_ref().unwrap().destroyed_run,
        Some(4)
    );
    assert_eq!(
        check_end(&state, &economy, PLAYER),
        Some(CampaignEnd::Won {
            road: Road::Soldier,
            run: 4
        })
    );
    assert_eq!(state.validate_navigation(&systems), Ok(()));
}

#[test]
fn charters_sell_once_pay_a_port_share_and_conserve_credits() {
    let (systems, mut state, mut economy) = world();
    economy.spawn_traders(&systems, &state.movement_rules);
    let money = total_money(&economy);
    let ship = &mut state.ships[0];
    let market = economy.current_market(ship).unwrap().id;
    economy.credits = Credits(economy.rules.campaign.charter_price.0 - 1);
    assert_eq!(
        economy.buy_charter(ship),
        Err(EconomyError::InsufficientCredits)
    );
    economy.credits = Credits(1_000_000);
    let money = money - 100_000 + 1_000_000;
    let price = economy.buy_charter(ship).unwrap();
    assert_eq!(price, economy.rules.campaign.charter_price);
    assert_eq!(economy.charter_holder(market), Some(PLAYER));
    assert_eq!(
        economy.buy_charter(ship),
        Err(EconomyError::AlreadyChartered)
    );
    assert_eq!(total_money(&economy), money);
    // Buying 5 food at the chartered port refunds 2% of the bill.
    let before = economy.credits.0;
    let quote = economy
        .trade(ship, Commodity::Food, 5, TradeSide::Buy, 0)
        .unwrap();
    assert_eq!(
        before - economy.credits.0,
        quote.total.0 - quote.total.0 * 2 / 100
    );
    assert_eq!(total_money(&economy), money);
    // NPC traders pay the share too, after the player's wallet is restored.
    for _ in 0..40 {
        economy.advance_traders(&systems).unwrap();
    }
    assert_eq!(total_money(&economy), money);
    assert!(economy.validate_charters().is_ok());
    assert!(!economy.on_lane(PLAYER, [market, economy.markets[8].id]));
    let other = economy.markets.iter().find(|m| m.id != market).unwrap().id;
    economy.charters.push(crate::economy::Charter {
        market: other,
        owner: PLAYER,
    });
    assert!(economy.on_lane(PLAYER, [market, other]));
    economy.charters.push(crate::economy::Charter {
        market: other,
        owner: PLAYER,
    });
    assert!(economy.validate_charters().is_err(), "duplicate charter");
}

#[test]
fn rules_from_the_fixed_goal_version_still_load_with_the_share_goal() {
    let old = r#"{"net_worth_goal":2500000,"charter_price":500000,"charters_to_win":3,"port_share_percent":2}"#;
    let rules: crate::economy::CampaignRules = serde_json::from_str(old).unwrap();
    assert_eq!(rules.wealth_share_percent, 10);
    let saved = serde_json::to_value(&rules).unwrap();
    assert!(saved.get("net_worth_goal").is_none(), "never written again");
}
