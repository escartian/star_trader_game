//! Fence work (raids, smuggling) and the Free Captains' chain.
use super::*;
use crate::campaign::{self, CampaignEnd, Road};
use crate::economy::{Economy, EconomyRules, Market};
use crate::factions::FactionSetup;
use crate::generation::{GalaxyConfig, generate_galaxy_with};
use crate::navigation::{Encounter, EncounterKind};
use crate::police::PoliceContact;
use crate::travel::{Location, PlayerId, TravelState};

const PLAYER: PlayerId = PlayerId(1);

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

fn job(task: Task) -> Job {
    Job {
        id: 1,
        kind: JobKind::Raid,
        issuer: MarketId {
            system: crate::SystemId(0),
            body: crate::BodyId(0),
        },
        faction: None,
        task,
        reward: Credits(50_000),
        standing: 0,
        penalty: 0,
        deadline: 10,
        title: "Fence order".into(),
        brief: String::new(),
        quest: None,
    }
}

#[test]
fn fence_work_is_posted_only_at_ports_no_faction_holds() {
    let (systems, mut state, economy) = world();
    let lawless = |market: &MarketId| economy.factions.owner(market.system).is_none();
    assert!(
        economy.markets.iter().any(|market| lawless(&market.id)),
        "the test galaxy has lawless ports"
    );
    let mut fence = 0;
    for market in &economy.markets {
        state.ships[0].location = at(market);
        state.jobs.boards.clear();
        state.jobs.offers.clear();
        refresh_offers(&mut state, &economy, &systems);
        for offer in &state.jobs.offers {
            if matches!(offer.kind, JobKind::Raid | JobKind::Smuggle) {
                fence += 1;
                assert!(lawless(&offer.issuer), "{}", offer.title);
                assert_eq!(offer.faction, None);
                assert!(!offer.brief.contains('{'), "{}", offer.brief);
            }
        }
    }
    assert!(fence > 0, "some lawless port posts fence work");
}

#[test]
fn a_raid_counts_only_units_seized_after_it_was_taken() {
    let (_, mut state, mut economy) = world();
    state.navigation.pirate_cargo = 30;
    state.jobs.next_id = 2;
    state
        .jobs
        .active
        .push(job(Task::Raid { units: 8, taken: 0 }));
    state.jobs.record_seizure(5);
    assert!(settle(&mut state, &mut economy, PLAYER).is_empty());
    let credits = economy.credits;
    state.jobs.record_seizure(4);
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(outcomes[0].success, "{}", outcomes[0].reason);
    assert_eq!(economy.credits.0, credits.0 + 50_000);
}

#[test]
fn a_smuggling_run_fails_when_that_factions_police_stop_you() {
    let (_, mut state, mut economy) = world();
    let target = economy
        .markets
        .iter()
        .find(|market| economy.factions.owner(market.id.system).is_some())
        .unwrap()
        .clone();
    let owner = economy.factions.owner(target.id.system).unwrap();
    let smuggle = Task::Smuggle {
        commodity: crate::economy::Commodity::Medicine,
        quantity: 4,
        market: target.id,
    };
    state.jobs.next_id = 2;
    state.jobs.active.push(job(smuggle.clone()));
    let fleet = state.fleets[0].id;
    state.navigation.encounters.push(Encounter {
        id: 1,
        fleet,
        kind: EncounterKind::Police,
        run: 0,
        hostiles: Vec::new(),
        band: None,
        haven: false,
        trader: None,
        police: Some(PoliceContact {
            patrol: 1,
            faction: owner,
        }),
        rescue: None,
    });
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(!outcomes[0].success);
    // Unseen, the goods are delivered and consumed like a delivery.
    state.navigation.encounters.clear();
    state.jobs.active.push(job(smuggle));
    state.ships[0].location = at(&target);
    state.ships[0].cargo[crate::economy::Commodity::Medicine.index()] = 4;
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(outcomes[0].success, "{}", outcomes[0].reason);
    assert_eq!(state.ships[0].cargo_used(), 0);
}

#[test]
fn the_free_captains_open_with_seizures_and_their_last_step_wins_the_pirate_road() {
    let (systems, mut state, mut economy) = world();
    let pirate_offer = |state: &TravelState| {
        state
            .jobs
            .quest_offers
            .iter()
            .find(|job| {
                job.quest
                    .as_ref()
                    .is_some_and(|quest| quest.faction.is_none())
            })
            .cloned()
    };
    quests::offer(&mut state, &economy, &systems);
    assert!(pirate_offer(&state).is_none(), "no seizure yet");
    state.navigation.piracies = 1;
    state.navigation.pirate_seizures = 1;
    quests::offer(&mut state, &economy, &systems);
    let first = pirate_offer(&state).expect("the first step opens with a seizure");
    assert_eq!(first.faction, None);
    assert_eq!(economy.factions.owner(first.issuer.system), None);
    assert!(!first.brief.contains('{'), "{}", first.brief);
    // Jump to the last step, which needs a police escape too.
    state.jobs.quest_offers.clear();
    let steps = economy.rules.quest_catalog.steps(None).len() as u32;
    state.jobs.pirate_steps = steps - 1;
    state.navigation.piracies = 3;
    state.navigation.pirate_seizures = 3;
    quests::offer(&mut state, &economy, &systems);
    let last = pirate_offer(&state).expect("the last step needs no escape");
    let Task::Raid { units, .. } = last.task else {
        panic!("the last step is a raid");
    };
    state.jobs.accept(last.id, None, 0, 0).unwrap();
    state.jobs.record_seizure(units);
    let outcomes = settle(&mut state, &mut economy, PLAYER);
    assert!(outcomes[0].success);
    assert_eq!(state.jobs.pirate_steps, steps);
    assert!(matches!(
        campaign::check_end(&state, &economy, PLAYER),
        Some(CampaignEnd::Won {
            road: Road::Pirate,
            ..
        })
    ));
}

#[test]
fn a_pirate_step_that_needs_an_escape_waits_for_one_where_factions_hold_space() {
    let (systems, mut state, economy) = world();
    let index = economy
        .rules
        .quest_catalog
        .steps(None)
        .iter()
        .position(|step| step.escapes > 0)
        .expect("a step needs a police escape") as u32;
    state.jobs.pirate_steps = index;
    state.navigation.piracies = 5;
    state.navigation.pirate_seizures = 5;
    assert!(quests::pirate_needs(&state, &economy).is_some());
    assert!(quests::offer(&mut state, &economy, &systems).is_empty());
    state.navigation.pirate_escapes = 1;
    assert_eq!(quests::pirate_needs(&state, &economy), None);
    assert_eq!(quests::offer(&mut state, &economy, &systems).len(), 1);
}
