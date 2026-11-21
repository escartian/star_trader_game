use super::{FLEETS_PROFIT, Feature, Progress, ROUTE_DELIVERIES, newly_unlocked};
use crate::{
    BodyId, SystemId,
    combat::{CombatOutcome, CombatReport},
    economy::{Credits, Economy},
    fleet::{Fleet, FleetId},
    navigation::{Encounter, EncounterKind},
    travel::{Location, PlayerId, TravelState},
};

const PLAYER: PlayerId = PlayerId(1);

fn captain() -> (Vec<crate::StarSystem>, TravelState, Economy) {
    let systems = crate::demo_galaxy();
    let economy = Economy::new(&systems);
    let mut state = TravelState::default();
    let start = crate::campaign::net_worth(&state, &economy, PLAYER);
    state.progress = Progress::captain(start);
    (systems, state, economy)
}

fn unlocks(state: &TravelState, economy: &Economy, systems: &[crate::StarSystem]) -> Vec<Feature> {
    newly_unlocked(state, economy, systems, PLAYER)
}

#[test]
fn a_new_captain_has_nothing_extra_and_old_saves_have_everything() {
    let (systems, state, economy) = captain();
    assert!(unlocks(&state, &economy, &systems).is_empty());
    assert!(
        Feature::ALL
            .iter()
            .all(|feature| Progress::default().has(*feature))
    );
    // An older save without progress loads fully unlocked.
    let mut json = serde_json::to_value(&state).unwrap();
    json.as_object_mut().unwrap().remove("progress");
    let old: TravelState = serde_json::from_value(json).unwrap();
    assert!(old.progress.has(Feature::Victory));
}

#[test]
fn profit_opens_the_shipyard_and_deliveries_or_fleets_open_routes() {
    let (systems, mut state, mut economy) = captain();
    economy.credits = Credits(economy.credits.0 + FLEETS_PROFIT.0 - 100);
    assert!(unlocks(&state, &economy, &systems).is_empty());
    economy.credits = Credits(economy.credits.0 + 100);
    assert_eq!(unlocks(&state, &economy, &systems), [Feature::Fleets]);
    let (systems, mut fresh, economy) = captain();
    fresh.progress.deliveries = ROUTE_DELIVERIES;
    assert_eq!(unlocks(&fresh, &economy, &systems), [Feature::TradeRoutes]);
    state.progress.unlocked.push(Feature::Fleets);
    state.fleets.push(Fleet {
        id: FleetId(2),
        name: "Second".into(),
        owner: PLAYER,
        ships: Vec::new(),
    });
    assert!(unlocks(&state, &economy, &systems).contains(&Feature::TradeRoutes));
}

#[test]
fn raiders_moons_and_bounties_open_battles_landing_and_victory() {
    let (systems, mut state, economy) = captain();
    state.navigation.encounters.push(Encounter {
        id: 1,
        fleet: FleetId(1),
        kind: EncounterKind::Raiders,
        run: 1,
        hostiles: Vec::new(),
        band: None,
        haven: false,
        trader: None,
    });
    assert_eq!(unlocks(&state, &economy, &systems), [Feature::Battles]);
    let (systems, mut state, economy) = captain();
    state.ships[0].location = Location::Orbiting {
        system: SystemId(1),
        body: BodyId(4),
    };
    assert_eq!(
        unlocks(&state, &economy, &systems),
        [Feature::Landing],
        "Luna"
    );
    state.ships[0].location = Location::Orbiting {
        system: SystemId(1),
        body: BodyId(1),
    };
    assert!(
        unlocks(&state, &economy, &systems).is_empty(),
        "a planet is not enough"
    );
    let (systems, mut state, economy) = captain();
    state.navigation.last_combat = Some(Box::new(CombatReport {
        encounter: 1,
        fleet: FleetId(1),
        run: 1,
        seed: 0,
        outcome: CombatOutcome::Victory,
        combatants: Vec::new(),
        rounds: Vec::new(),
        salvage_ore: 0,
        cargo_lost: 0,
        crippled: None,
        bounty: Credits(15_000),
        trader: None,
        plunder: 0,
    }));
    let found = unlocks(&state, &economy, &systems);
    assert!(found.contains(&Feature::Victory) && found.contains(&Feature::Battles));
}
