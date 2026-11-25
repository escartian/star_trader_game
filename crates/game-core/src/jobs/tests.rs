use super::*;
use crate::economy::{Economy, EconomyRules, Market};
use crate::travel::{Location, PlayerId, TravelState};

const PLAYER: PlayerId = PlayerId(1);

fn world() -> (Vec<crate::StarSystem>, TravelState, Economy) {
    let rules = EconomyRules::default();
    let mut systems = crate::generation::generate_galaxy(7);
    rules.planet_catalog.populate(&mut systems);
    let mut economy = Economy::with_rules(&systems, rules).unwrap();
    let mut state = TravelState::default();
    state.navigation.rng_state = 7;
    economy.spawn_traders(&systems, &state.movement_rules);
    state.spawn_raider_bands(&systems);
    (systems, state, economy)
}

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

/// A delivery job for `quantity` of the first commodity at `market`, due on run 5.
fn delivery(id: u64, market: MarketId, quantity: u32) -> Job {
    Job {
        id,
        kind: JobKind::Deliver,
        issuer: market,
        faction: None,
        task: Task::Deliver {
            commodity: crate::economy::Commodity::ALL[0],
            quantity,
            market,
        },
        reward: Credits(50_000),
        standing: 3,
        penalty: 4,
        deadline: 5,
        title: "Supply run".into(),
        brief: String::new(),
    }
}

#[test]
fn the_embedded_catalog_validates() {
    let catalog = JobCatalog::default();
    assert_eq!(catalog.kinds.len(), JobKind::ALL.len());
}

#[test]
fn a_port_posts_offers_while_you_are_there_reproducibly_until_they_expire() {
    let (systems, mut state, economy) = world();
    let home = economy.current_market(&state.ships[0]).unwrap().id;
    assert!(refresh_offers(&mut state, &economy, &systems));
    let per_port = economy.rules.job_catalog.offers_per_port as usize;
    assert_eq!(state.jobs.offers.len(), per_port);
    assert!(state.jobs.offers.iter().all(|job| job.issuer == home));
    assert!(
        state
            .jobs
            .offers
            .iter()
            .all(|job| !job.title.contains('{') && !job.brief.contains('{'))
    );
    let mut again = state.clone();
    again.jobs = Jobs::default();
    refresh_offers(&mut again, &economy, &systems);
    assert_eq!(again.jobs.offers, state.jobs.offers);
    // The board stands until `refresh_runs` have passed, then is replaced.
    assert!(!refresh_offers(&mut state, &economy, &systems));
    let first = state.jobs.offers.clone();
    state.navigation.run += economy.rules.job_catalog.refresh_runs;
    assert!(refresh_offers(&mut state, &economy, &systems));
    assert_eq!(state.jobs.offers.len(), per_port);
    assert!(state.jobs.offers.iter().all(|job| !first.contains(job)));
}

#[test]
fn boards_where_you_are_not_expire_and_post_nothing_new() {
    let (systems, mut state, economy) = world();
    refresh_offers(&mut state, &economy, &systems);
    state.ships[0].location = Location::DeepSpace(crate::movement::SpacePosition {
        x_km: 0,
        y_km: 0,
        z_km: 0,
    });
    state.navigation.run += economy.rules.job_catalog.refresh_runs;
    assert!(refresh_offers(&mut state, &economy, &systems));
    assert!(state.jobs.offers.is_empty());
    assert!(state.jobs.boards.is_empty());
}

#[test]
fn accepting_needs_a_ship_at_the_issuing_port() {
    let (systems, mut state, economy) = world();
    refresh_offers(&mut state, &economy, &systems);
    let job = state.jobs.offers[0].clone();
    let elsewhere = economy
        .markets
        .iter()
        .find(|market| market.id != job.issuer)
        .unwrap();
    assert_eq!(
        state
            .jobs
            .accept(job.id, Some(elsewhere.id), 5)
            .unwrap_err(),
        JobError::NotAtPort
    );
    state.jobs.accept(job.id, Some(job.issuer), 5).unwrap();
    assert_eq!(state.jobs.active, vec![job.clone()]);
    assert_eq!(
        state.jobs.accept(job.id, Some(job.issuer), 5).unwrap_err(),
        JobError::UnknownJob
    );
}

#[test]
fn a_delivery_pays_once_takes_the_goods_and_stocks_the_port() {
    let (_, mut state, mut economy) = world();
    let market = economy.markets[0].clone();
    state.jobs.active.push(delivery(1, market.id, 6));
    state.ships[0].location = at(&market);
    state.ships[0].cargo[0] = 4;
    assert!(
        settle(&mut state, &mut economy, PLAYER).is_empty(),
        "4 of 6 is not enough"
    );
    state.ships[0].cargo[0] = 9;
    let credits = economy.credits;
    let stock = economy.market(market.id).unwrap().goods[0].stock;
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].success);
    assert_eq!(economy.credits.0, credits.0 + 50_000);
    assert_eq!(state.ships[0].cargo[0], 3);
    let capacity = economy.market(market.id).unwrap().goods[0].capacity();
    assert_eq!(
        economy.market(market.id).unwrap().goods[0].stock,
        (stock + 6).min(capacity)
    );
    assert!(state.jobs.active.is_empty());
    assert!(
        settle(&mut state, &mut economy, PLAYER).is_empty(),
        "paid once"
    );
    assert_eq!(state.jobs.completed, 1);
}

#[test]
fn a_missed_deadline_fails_without_pay() {
    let (_, mut state, mut economy) = world();
    let market = economy.markets[0].id;
    state.jobs.active.push(delivery(1, market, 6));
    state.navigation.run = 6;
    let credits = economy.credits;
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(!outcomes[0].success);
    assert_eq!(economy.credits, credits);
    assert_eq!(state.jobs.failed, 1);
}

#[test]
fn abandoning_costs_standing_with_the_issuer() {
    let (_, mut state, mut economy) = world();
    let faction = crate::factions::Faction::ALL[0];
    economy.factions.in_play = vec![faction];
    let mut job = delivery(1, economy.markets[0].id, 6);
    job.faction = Some(faction);
    state.jobs.active.push(job);
    let before = economy.factions.standing(faction);
    let outcome = abandon(&mut state, &mut economy, 1).unwrap();
    assert!(!outcome.success);
    assert_eq!(economy.factions.standing(faction), before - 4);
    assert!(abandon(&mut state, &mut economy, 1).is_none());
}

#[test]
fn a_survey_completes_in_orbit_and_a_hunt_needs_your_own_kill() {
    let (systems, mut state, mut economy) = world();
    let system = &systems[0];
    let body = system.bodies[0].id;
    let mut survey = delivery(1, economy.markets[0].id, 1);
    survey.task = Task::Survey {
        system: system.id,
        body,
    };
    let band = state.navigation.bands[0].clone();
    let mut hunt = delivery(2, economy.markets[0].id, 1);
    hunt.task = Task::Hunt {
        band: band.id,
        system: band.system,
    };
    let mut lost = hunt.clone();
    lost.id = 3;
    lost.task = Task::Hunt {
        band: u64::MAX,
        system: band.system,
    };
    state.jobs.active = vec![survey, hunt, lost];
    state.ships[0].location = Location::Orbiting {
        system: system.id,
        body,
    };
    state.jobs.record_cleared_band(band.id);
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    let results: Vec<bool> = outcomes.iter().map(|outcome| outcome.success).collect();
    assert_eq!(results, vec![true, true, false]);
}
