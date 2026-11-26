//! Hailing, robbing and attacking NPC traders (`game_core::piracy`).
use super::{
    LOCAL_PLAYER, Session,
    combat_tests::{choose, reload},
    faction_tests::claimed,
    test_support::execute_quiet_run,
};
use game_core::{
    campaign::{CampaignEnd, Road},
    economy::{Commodity, Credits},
    factions::Faction,
    fleet::{FleetId, ShipKind},
    jobs::{Job, JobKind, Task, quests::QuestRef},
    navigation::{EncounterChoice, EncounterKind, NavigationError},
    npc::{REFIT_RUNS, market_location},
    piracy,
    travel::Location,
};
use game_protocol::{ClientCommand, CommandError, ServerMessage};

/// After one quiet run, puts trader `index` (with 10 ore and 6 medicine) and
/// the player's flagship in orbit at the same planet market.
pub(super) fn meet_trader(session: &mut Session, index: usize) -> (FleetId, u64) {
    execute_quiet_run(session);
    let market = session
        .economy
        .markets
        .iter()
        .find(|market| !market.is_station)
        .unwrap()
        .id;
    let at = market_location(market, &session.systems);
    assert!(matches!(at, Location::Orbiting { .. }));
    let trader = &mut session.economy.traders[index];
    trader.state.navigation.orders.clear();
    trader.state.ships[0].location = at.clone();
    trader.state.ships[0].cargo = [0; 6];
    trader.state.ships[0].cargo[Commodity::Ore.index()] = 10;
    trader.state.ships[0].cargo[Commodity::Medicine.index()] = 6;
    let trader = trader.id;
    session.state.navigation.orders.clear();
    session.state.ships[0].location = at;
    (session.state.fleets[0].id, trader)
}

fn hail(session: &mut Session, fleet: FleetId, trader: u64) -> ServerMessage {
    session.handle(ClientCommand::HailTrader {
        fleet,
        trader,
        expected_revision: session.state.revision,
    })
}

fn rejected(message: ServerMessage, expected: NavigationError) {
    match message {
        ServerMessage::CommandRejected(CommandError::Navigation(error)) => {
            assert_eq!(error, expected)
        }
        other => panic!("expected {expected:?}, got {other:?}"),
    }
}

pub(super) fn hailed(session: &mut Session, fleet: FleetId, trader: u64) -> u64 {
    let response = hail(session, fleet, trader);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    let contact = session.state.navigation.encounters.last().unwrap();
    assert_eq!(contact.kind, EncounterKind::Trader);
    contact.id
}

#[test]
fn the_flagship_hails_only_traders_in_reach_and_out_of_dock() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    let ours = session.state.ships[0].location.clone();
    // Far away: the trader back at another market.
    let far = session
        .economy
        .markets
        .iter()
        .filter(|market| !market.is_station)
        .map(|market| market_location(market.id, &session.systems))
        .find(|place| *place != ours)
        .unwrap();
    session.economy.traders[0].state.ships[0].location = far;
    rejected(
        hail(&mut session, fleet, trader),
        NavigationError::TraderOutOfReach,
    );
    // Refitting traders are laid up.
    session.economy.traders[0].state.ships[0].location = ours.clone();
    session.economy.traders[0].refit_runs = 2;
    rejected(
        hail(&mut session, fleet, trader),
        NavigationError::TraderProtected,
    );
    session.economy.traders[0].refit_runs = 0;
    rejected(
        hail(&mut session, fleet, 99),
        NavigationError::UnknownTrader,
    );
    let systems = session.systems.clone();
    assert!(
        piracy::in_reach(&session.state, &session.economy, LOCAL_PLAYER, &systems)
            .contains(&trader)
    );
    let contact = hailed(&mut session, fleet, trader);
    // A pending hail holds the run like any contact, and survives a save.
    let run = session.handle(ClientCommand::ExecuteNavigationRun {
        expected_revision: session.state.revision,
    });
    assert!(matches!(run, ServerMessage::CommandRejected(_)));
    let mut session = reload(&session);
    let before = session.economy.traders[0].clone();
    choose(&mut session, contact, EncounterChoice::Bypass);
    assert!(session.state.navigation.encounters.is_empty());
    assert_eq!(
        session.economy.traders[0], before,
        "letting it go costs nothing"
    );
    assert_eq!(session.state.navigation.piracies, 0);
}

#[test]
fn an_outgunned_trader_hands_over_half_its_cargo_most_valuable_first() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    let contact = hailed(&mut session, fleet, trader);
    // Freighter against freighter: a cautious trader wants 150%.
    rejected(
        choose(&mut session, contact, EncounterChoice::Demand),
        NavigationError::WillNotYield,
    );
    // Badly damaged, it is outgunned and gives way.
    choose(&mut session, contact, EncounterChoice::Bypass);
    session.economy.traders[0].state.ships[0].hull_damage = 40;
    let contact = hailed(&mut session, fleet, trader);
    let response = choose(&mut session, contact, EncounterChoice::Demand);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    // Half of 16 units; medicine is worth more than ore.
    let ours = &session.state.ships[0].cargo;
    assert_eq!(
        (
            ours[Commodity::Medicine.index()],
            ours[Commodity::Ore.index()]
        ),
        (6, 2)
    );
    let theirs = &session.economy.traders[0].state.ships[0].cargo;
    assert_eq!(
        (
            theirs[Commodity::Medicine.index()],
            theirs[Commodity::Ore.index()]
        ),
        (0, 8)
    );
    assert_eq!(session.state.navigation.piracies, 1);
    assert_eq!(session.state.navigation.pirate_seizures, 1);
    assert_eq!(session.state.navigation.pirate_cargo, 8);
    assert!(session.economy.traders[0].last_action.contains("pirates"));
    reload(&session);
}

#[test]
fn a_seizure_that_completes_the_free_captains_last_step_wins_the_pirate_road() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    session.economy.factions = claimed([None, Some(Faction::Federation), None])
        .economy
        .factions;
    let steps = session.economy.rules.quest_catalog.steps(None).len() as u32;
    let jobs = &mut session.state.jobs;
    jobs.pirate_steps = steps - 1;
    let id = jobs.next_id;
    jobs.next_id += 1;
    jobs.active.push(Job {
        id,
        kind: JobKind::Raid,
        issuer: session.economy.markets[0].id,
        faction: None,
        task: Task::Raid { units: 4, taken: 0 },
        reward: Credits(200_000),
        standing: 0,
        penalty: 0,
        deadline: 50,
        title: "The Free Port Charter".into(),
        brief: String::new(),
        quest: Some(QuestRef {
            faction: None,
            step: steps - 1,
            runs: 20,
            intro: String::new(),
            treaty: false,
        }),
    });
    session.economy.traders[0].state.ships[0].hull_damage = 40;
    let contact = hailed(&mut session, fleet, trader);
    choose(&mut session, contact, EncounterChoice::Demand);
    assert_eq!(session.state.jobs.pirate_steps, steps);
    assert_eq!(
        session.state.campaign.ended,
        Some(CampaignEnd::Won {
            road: Road::Pirate,
            run: session.state.navigation.run,
        })
    );
    let restored = reload(&session);
    assert_eq!(restored.state.campaign.ended, session.state.campaign.ended);
}

#[test]
fn an_opportunist_needs_a_far_stronger_fleet_to_yield() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 1);
    assert_eq!(
        session.economy.traders[1].personality,
        game_core::npc::Personality::Opportunist
    );
    // 70 against 30 is enough for a cautious trader, not for an opportunist.
    session.economy.traders[1].state.ships[0].hull_damage = 40;
    let contact = hailed(&mut session, fleet, trader);
    rejected(
        choose(&mut session, contact, EncounterChoice::Demand),
        NavigationError::WillNotYield,
    );
    // A patrol ship (360) outguns it more than threefold.
    session.state.ships[0].kind = ShipKind::Patrol;
    let response = choose(&mut session, contact, EncounterChoice::Demand);
    assert!(matches!(response, ServerMessage::SessionSnapshot { .. }));
}

#[test]
fn a_destroyed_trader_is_plundered_and_refitted_without_a_bounty() {
    let mut session = Session::default();
    // Test-only patrol gun: certain, long-range and lethal; the patrol's
    // engine moves first, so the trader never fires back.
    session.state.ships[0].kind = ShipKind::Patrol;
    let gun = session
        .economy
        .rules
        .combat_catalog
        .weapons
        .iter_mut()
        .find(|weapon| weapon.id == "photon_singularity_beam")
        .unwrap();
    (gun.range, gun.damage, gun.accuracy_percent) = (12, 10_000, 100);
    let (fleet, trader) = meet_trader(&mut session, 0);
    let credits = session.economy.credits;
    let contact = hailed(&mut session, fleet, trader);
    choose(&mut session, contact, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.trader, Some(trader));
    assert!(
        report
            .side(game_core::combat::Side::Hostile)
            .all(|ship| ship.destroyed)
    );
    assert_eq!(report.bounty.0, 0);
    assert_eq!(session.economy.credits, credits, "no bounty for a trader");
    assert_eq!(report.plunder, 16);
    let ours = &session.state.ships[0].cargo;
    assert_eq!(ours[Commodity::Medicine.index()], 6);
    assert_eq!(ours[Commodity::Ore.index()], 10 + report.salvage_ore);
    let npc = &session.economy.traders[0];
    assert_eq!(npc.refit_runs, REFIT_RUNS);
    assert_eq!(npc.state.ships[0].cargo_used(), 0);
    assert!(npc.last_action.starts_with("Destroyed by pirates"));
    assert_eq!(session.state.navigation.piracies, 1);
    assert_eq!(session.state.navigation.pirate_seizures, 1);
    assert_eq!(session.state.navigation.pirate_cargo, 16);
    let mut session = reload(&session);
    // Later commands never settle the same battle again.
    execute_quiet_run(&mut session);
    assert_eq!(session.economy.traders[0].refit_runs, REFIT_RUNS - 1);
    assert_eq!(session.state.navigation.pirate_cargo, 16);
}

#[test]
fn a_trader_that_survives_keeps_its_cargo_and_its_damage() {
    let mut survived = 0;
    for seed in 0..40 {
        let mut session = Session::default();
        session.state.navigation.rng_state = seed;
        let (fleet, trader) = meet_trader(&mut session, 1);
        let contact = hailed(&mut session, fleet, trader);
        choose(&mut session, contact, EncounterChoice::AutoResolve);
        let report = session.state.navigation.last_combat.clone().unwrap();
        let theirs = report
            .side(game_core::combat::Side::Hostile)
            .next()
            .unwrap()
            .clone();
        let npc = &session.economy.traders[1];
        if theirs.destroyed {
            assert_eq!(npc.refit_runs, REFIT_RUNS, "seed {seed}");
            continue;
        }
        survived += 1;
        assert_eq!(report.plunder, 0, "seed {seed}");
        assert_eq!(npc.state.ships[0].hull_damage, theirs.final_damage);
        assert_eq!(npc.state.ships[0].cargo_used(), 16, "seed {seed}");
        assert!(npc.last_action.contains("Survived"), "seed {seed}");
        reload(&session);
    }
    assert!(
        survived > 0,
        "an even duel lets the trader get away sometimes"
    );
}

#[test]
fn breaking_off_a_trader_battle_costs_no_cargo() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    session.state.ships[0].cargo[Commodity::Ore.index()] = 5;
    let contact = hailed(&mut session, fleet, trader);
    choose(&mut session, contact, EncounterChoice::Fight);
    assert!(session.state.navigation.battle.is_some());
    // The open battle, its trader name and damage survive a save.
    let mut session = reload(&session);
    let response = session.handle(ClientCommand::BattleAction {
        action: game_core::navigation::BattleAction::Surrender,
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    assert!(session.state.navigation.battle.is_none());
    assert_eq!(session.state.ships[0].cargo[Commodity::Ore.index()], 5);
    reload(&session);
}
