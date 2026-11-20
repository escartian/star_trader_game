use super::{
    Session,
    combat_tests::{choose, raid, reload},
    test_support::execute_quiet_run,
};
use game_core::{
    combat::{CombatOutcome, Side},
    fleet::ShipKind,
    navigation::EncounterChoice,
    npc::REFIT_RUNS,
    travel::ShipId,
};

/// Runs until some trader reports a raid, or panics after `limit` runs.
fn run_until_raided(session: &mut Session, limit: usize) -> usize {
    for _ in 0..limit {
        execute_quiet_run(session);
        if let Some(index) = session
            .economy
            .traders
            .iter()
            .position(|trader| trader.last_action.contains("raiders"))
        {
            return index;
        }
    }
    panic!("no trader met raiders in {limit} runs");
}

#[test]
fn destroyed_raiders_pay_their_bounty_once_and_fled_ones_pay_nothing() {
    for seed in 0..40 {
        let mut session = Session::default();
        session.state.navigation.rng_state = seed;
        let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Scout]);
        let credits = session.economy.credits.0;
        choose(&mut session, encounter, EncounterChoice::AutoResolve);
        let report = session.state.navigation.last_combat.clone().unwrap();
        let destroyed = report
            .side(Side::Hostile)
            .filter(|ship| ship.destroyed)
            .count() as u64;
        // Paid at today's price level.
        let scout = session
            .economy
            .inflate(session.economy.rules.raider_bounties.scout);
        let bounty = destroyed * scout.0;
        assert_eq!(report.bounty.0, bounty, "seed {seed}");
        assert_eq!(session.economy.credits.0, credits + bounty, "paid once");
        if report.outcome == CombatOutcome::Victory && destroyed == 0 {
            assert_eq!(report.bounty.0, 0, "a raider that fled pays nothing");
        }
        // Later commands never pay the same report again.
        execute_quiet_run(&mut session);
        assert!(session.economy.credits.0 <= credits + bounty);
        reload(&session);
    }
}

#[test]
fn traders_under_way_meet_raiders_and_damage_sticks_until_repaired() {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 100;
    let index = run_until_raided(&mut session, 40);
    let trader = &session.economy.traders[index];
    assert!(
        trader.last_action.contains("Fought off")
            || trader.last_action.contains("Escaped")
            || trader.last_action.contains("Destroyed"),
        "{}",
        trader.last_action
    );
    reload(&session);
}

#[test]
fn a_destroyed_trader_loses_its_cargo_and_refits_at_home_before_trading_again() {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 100;
    // Test-only raider gun: certain, long-range and lethal, so the trader has
    // no time to retreat.
    let gun = session
        .economy
        .rules
        .combat_catalog
        .weapons
        .iter_mut()
        .find(|weapon| weapon.id == "graviton_pulse")
        .unwrap();
    (gun.range, gun.damage, gun.accuracy_percent) = (12, 10_000, 100);
    let mut destroyed = None;
    for _ in 0..60 {
        execute_quiet_run(&mut session);
        destroyed = session
            .economy
            .traders
            .iter()
            .position(|trader| trader.last_action.starts_with("Destroyed by raiders"));
        if destroyed.is_some() {
            break;
        }
    }
    let index = destroyed.expect("a trader was destroyed");
    let trader = session.economy.traders[index].clone();
    assert_eq!(trader.refit_runs, REFIT_RUNS);
    assert_eq!(trader.state.ships[0].cargo_used(), 0);
    assert_eq!(trader.state.ships[0].hull_damage, 0);
    assert!(trader.state.navigation.orders.is_empty());
    reload(&session);
    session.economy.rules.npc_raid_chance_percent = 0;
    for left in (0..REFIT_RUNS).rev() {
        execute_quiet_run(&mut session);
        let trader = &session.economy.traders[index];
        assert_eq!(trader.refit_runs, left);
        assert!(
            trader.state.navigation.orders.is_empty(),
            "no trading while refitting"
        );
    }
    assert!(
        session.economy.traders[index]
            .last_action
            .contains("back in service")
    );
}
