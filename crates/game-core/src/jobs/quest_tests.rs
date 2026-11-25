//! Story quests: offers by standing, acceptance anywhere, the treaty and the Diplomat road.
use super::quests::{self, QuestCatalog};
use super::*;
use crate::campaign::{self, CampaignEnd, Road};
use crate::economy::{Economy, EconomyRules};
use crate::factions::{Faction, FactionSetup};
use crate::generation::{GalaxyConfig, generate_galaxy_with};
use crate::travel::{Location, PlayerId, TravelState};

const PLAYER: PlayerId = PlayerId(1);
const FACTION: Faction = Faction::Federation;

fn world() -> (Vec<crate::StarSystem>, TravelState, Economy) {
    let rules = EconomyRules::default();
    let mut systems = generate_galaxy_with(5, GalaxyConfig::default()).unwrap();
    rules.planet_catalog.populate(&mut systems);
    let mut economy = Economy::with_rules(&systems, rules).unwrap();
    economy.factions = FactionSetup::default()
        .generate(5, &systems, &economy.rules.faction_catalog)
        .unwrap();
    let mut state = TravelState::default();
    state.navigation.rng_state = 5;
    state.spawn_raider_bands(&systems);
    (systems, state, economy)
}

fn quest_offer(state: &TravelState) -> Option<&Job> {
    state.jobs.quest_offers.iter().find(|job| {
        job.quest
            .as_ref()
            .is_some_and(|quest| quest.faction == FACTION)
    })
}

#[test]
fn the_embedded_quests_give_each_faction_a_chain_ending_in_a_treaty() {
    let catalog = QuestCatalog::default();
    for faction in Faction::ALL {
        let chain = catalog.chain(faction).expect("a chain per faction");
        assert!(chain.steps.last().unwrap().treaty, "{}", faction.label());
    }
}

#[test]
fn a_faction_offers_its_first_step_only_from_friendly_standing() {
    let (systems, mut state, mut economy) = world();
    assert!(quests::offer(&mut state, &economy, &systems).is_empty());
    economy.factions.standing[FACTION.index()] = 20;
    let offered = quests::offer(&mut state, &economy, &systems);
    assert_eq!(offered.len(), 1);
    let job = quest_offer(&state).unwrap().clone();
    assert_eq!(job.faction, Some(FACTION));
    assert_eq!(economy.factions.owner(job.issuer.system), Some(FACTION));
    assert!(!job.title.contains('{') && !job.brief.contains('{'));
    // Not offered twice while it waits.
    assert!(quests::offer(&mut state, &economy, &systems).is_empty());
    // Accepted anywhere (no port), its deadline counted from acceptance.
    state.navigation.run = 9;
    let runs = job.quest.as_ref().unwrap().runs;
    let accepted = state.jobs.accept(job.id, None, 0, 9).unwrap();
    assert_eq!(accepted.deadline, 9 + runs);
}

#[test]
fn the_treaty_waits_for_honored_standing_and_carrying_it_wins_the_diplomat_road() {
    let (systems, mut state, mut economy) = world();
    let steps = economy
        .rules
        .quest_catalog
        .chain(FACTION)
        .unwrap()
        .steps
        .len() as u32;
    state.jobs.quest_steps[FACTION.index()] = steps - 1;
    economy.factions.standing[FACTION.index()] = 30;
    assert!(
        quests::offer(&mut state, &economy, &systems).is_empty(),
        "Friendly is not enough"
    );
    economy.factions.standing[FACTION.index()] = 60;
    quests::offer(&mut state, &economy, &systems);
    let job = quest_offer(&state).unwrap().clone();
    assert!(job.quest.as_ref().unwrap().treaty);
    state.jobs.accept(job.id, None, 0, 0).unwrap();
    let Task::Deliver {
        commodity,
        quantity,
        market,
    } = job.task
    else {
        panic!("the treaty is a delivery");
    };
    let port = economy.market(market).unwrap().clone();
    state.ships[0].location = if port.is_station {
        Location::Docked(market.system)
    } else {
        Location::Orbiting {
            system: market.system,
            body: market.body,
        }
    };
    state.ships[0].cargo[commodity.index()] = quantity;
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(outcomes[0].success);
    assert_eq!(state.jobs.treaty, Some(FACTION));
    let diplomat = campaign::progress(&state, &economy, PLAYER)
        .into_iter()
        .find(|road| road.road == Road::Diplomat)
        .unwrap();
    assert!(diplomat.open && diplomat.complete);
    assert!(matches!(
        campaign::check_end(&state, &economy, PLAYER),
        Some(CampaignEnd::Won {
            road: Road::Diplomat,
            ..
        })
    ));
}

#[test]
fn a_failed_step_returns_after_the_retry_pause() {
    let (systems, mut state, mut economy) = world();
    economy.factions.standing[FACTION.index()] = 20;
    quests::offer(&mut state, &economy, &systems);
    let job = quest_offer(&state).unwrap().id;
    state.jobs.accept(job, None, 0, 0).unwrap();
    abandon(&mut state, &mut economy, job).unwrap();
    assert_eq!(state.jobs.quest_steps[FACTION.index()], 0);
    assert!(quests::offer(&mut state, &economy, &systems).is_empty());
    state.navigation.run = economy.rules.quest_catalog.retry_runs;
    assert_eq!(quests::offer(&mut state, &economy, &systems).len(), 1);
}
