use super::{Session, test_support::execute_quiet_run};
use game_core::{economy::Commodity, travel::ShipId};
use game_protocol::{ClientCommand, ServerMessage};

fn money(session: &Session) -> u128 {
    u128::from(session.economy.credits.0)
        + session
            .economy
            .markets
            .iter()
            .map(|market| u128::from(market.credits.0))
            .sum::<u128>()
        + session
            .economy
            .traders
            .iter()
            .map(|trader| u128::from(trader.credits.0))
            .sum::<u128>()
}
fn goods(economy: &game_core::economy::Economy, good: Commodity) -> u64 {
    economy
        .markets
        .iter()
        .map(|market| u64::from(market.goods[good.index()].stock))
        .sum::<u64>()
        + economy
            .traders
            .iter()
            .map(|trader| u64::from(trader.state.ships[0].cargo[good.index()]))
            .sum::<u64>()
}

#[test]
fn traders_act_only_on_runs_and_conserve_cash_and_goods_through_real_trades() {
    let mut session = Session::default();
    // Raids destroy cargo, and living markets move money in and out of
    // treasuries; this test is about trades alone.
    session.economy.rules.npc_raid_chance_percent = 0;
    session.economy.rules.dynamics = game_core::economy::MarketDynamics::frozen();
    let initial_cash = money(&session);
    let initial = session.snapshot(Vec::new());
    for _ in 0..5 {
        session.handle(ClientCommand::GetSession);
    }
    assert_eq!(session.snapshot(Vec::new()), initial);
    let mut cargo_seen = false;
    for _ in 0..16 {
        let mut expected = session.economy.clone();
        expected.advance(session.state.movement_rules.ticks_per_navigation_run);
        execute_quiet_run(&mut session);
        assert_eq!(money(&session), initial_cash);
        for good in Commodity::ALL {
            assert_eq!(goods(&session.economy, good), goods(&expected, good));
        }
        cargo_seen |= session
            .economy
            .traders
            .iter()
            .any(|trader| trader.state.ships[0].cargo_used() > 0);
        session.export_json().unwrap();
    }
    assert!(cargo_seen, "NPCs must actually trade and haul cargo");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("npc.json");
    session.save(&path).unwrap();
    let (mut restored, _) = Session::load(&path).unwrap();
    for _ in 0..4 {
        execute_quiet_run(&mut session);
        execute_quiet_run(&mut restored);
    }
    assert_eq!(restored.economy, session.economy);
    assert_eq!(restored.state, session.state);
}

#[test]
fn npc_hulls_are_not_player_commands_and_bad_npc_balances_reject_saving() {
    let mut session = Session::default();
    let id = session.economy.traders[0].state.ships[0].id;
    assert_ne!(id, ShipId(1));
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::Refuel {
            ship: id,
            units: 1,
            expected_revision: 0
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    session.economy.traders[0].credits.0 = u64::MAX;
    assert!(session.export_json().is_err());
}
