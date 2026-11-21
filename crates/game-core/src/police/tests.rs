use super::*;
use crate::{
    economy::{Economy, EconomyRules},
    factions::{FactionCatalog, FactionSetup},
    generation::generate_galaxy,
    travel::PlayerId,
};

fn world() -> (Vec<StarSystem>, TravelState, Economy) {
    let systems = generate_galaxy(0);
    let mut economy = Economy::with_rules(&systems, EconomyRules::default()).unwrap();
    economy.factions = FactionSetup::default()
        .generate(0, &systems, &FactionCatalog::default())
        .unwrap();
    let mut state = TravelState::default();
    state.spawn_police(&economy.factions, &systems);
    (systems, state, economy)
}

#[test]
fn every_claimed_system_gets_one_valid_patrol() {
    let (systems, state, economy) = world();
    let patrols = &state.navigation.police.patrols;
    assert_eq!(patrols.len(), economy.factions.territory.len());
    assert!(patrols.iter().all(|patrol| patrol.ships == PATROL_SHIPS));
    state.validate_police(&systems).unwrap();
    state.validate_police_factions(&economy.factions).unwrap();
}

#[test]
fn patrols_visit_each_body_in_turn() {
    let (systems, state, _) = world();
    let world = &systems[1];
    let count = world.bodies.len() as u64;
    let rules = &state.movement_rules;
    assert_eq!(post(world, 0, rules), post(world, count, rules));
    assert_ne!(post(world, 0, rules), post(world, 1, rules));
}

#[test]
fn a_patrol_wiped_out_is_replaced_after_a_while() {
    let (systems, mut state, mut economy) = world();
    let patrol = state.navigation.police.patrols[0].clone();
    state.police_after_contact(patrol.id, &[ShipKind::Scout]);
    let survivor = state.navigation.police.patrol(patrol.id).unwrap();
    assert_eq!(survivor.ships, [ShipKind::Patrol]);
    assert_eq!(survivor.cooldown, COOLDOWN_RUNS);
    state.police_after_contact(patrol.id, &[ShipKind::Patrol]);
    assert!(state.navigation.police.patrol(patrol.id).is_none());
    assert_eq!(
        state.navigation.police.respawns,
        [(patrol.system, RESPAWN_RUNS)]
    );
    state.validate_police(&systems).unwrap();
    state.navigation.run = RESPAWN_RUNS;
    patrol_run(&mut state, &mut economy, PlayerId(1), &systems).unwrap();
    let replaced = state
        .navigation
        .police
        .patrols
        .iter()
        .find(|known| known.system == patrol.system)
        .unwrap();
    assert!(replaced.id > patrol.id);
    assert_eq!(replaced.faction, patrol.faction);
    assert!(state.navigation.police.respawns.is_empty());
}
