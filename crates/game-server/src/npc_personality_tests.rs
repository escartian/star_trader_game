//! NPC trader personalities (`game_core::npc::personality`, `config/npc.json`).
use super::{GameSetup, Session, faction_tests::claimed, test_support::execute_quiet_run};
use game_core::{
    economy::EconomyRules,
    factions::{Faction, Relation},
    generation::GalaxyConfig,
    movement::MoveTarget,
    npc::Personality,
};

#[test]
fn new_traders_are_dealt_the_spawn_cycle_and_independents_are_never_loyalists() {
    let setup = GameSetup {
        galaxy: GalaxyConfig {
            systems: 40,
            ..GalaxyConfig::default()
        },
        ..GameSetup::default()
    };
    let session =
        Session::new_game(5, EconomyRules::default(), Default::default(), &setup).unwrap();
    let catalog = &session.economy.rules.personality_catalog;
    for (index, trader) in session.economy.traders.iter().enumerate() {
        let dealt = catalog.dealt(index);
        match trader.affiliation {
            Some(_) => assert_eq!(trader.personality, dealt, "{}", trader.name),
            None => assert_ne!(trader.personality, Personality::Loyalist, "{}", trader.name),
        }
    }
    let kinds: std::collections::HashSet<_> = session
        .economy
        .traders
        .iter()
        .map(|trader| trader.personality)
        .collect();
    assert!(
        kinds.len() >= 4,
        "a 40-system galaxy mixes personalities: {kinds:?}"
    );
}

#[test]
fn bulk_haulers_load_more_than_the_others() {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 0;
    for trader in &mut session.economy.traders {
        trader.personality = Personality::BulkHauler;
    }
    let units = session
        .economy
        .rules
        .personality_catalog
        .get(Personality::BulkHauler)
        .haul_units;
    let mut largest = 0;
    for _ in 0..20 {
        execute_quiet_run(&mut session);
        for trader in &session.economy.traders {
            largest = largest.max(trader.state.ships[0].cargo_used());
        }
    }
    assert_eq!(largest, u64::from(units));
}

#[test]
fn loyalists_trade_only_in_their_own_and_allied_space() {
    // Classic relations: the Alliance is neutral to the Federation, so an
    // ordinary Federation trader may sail there; a loyalist stays home.
    let mut session = claimed([
        Some(Faction::Federation),
        Some(Faction::Alliance),
        Some(Faction::Alliance),
    ]);
    for trader in &mut session.economy.traders {
        trader.affiliation = Some(Faction::Federation);
        trader.personality = Personality::Loyalist;
    }
    let factions = session.economy.factions.clone();
    let mut sailed = 0;
    for _ in 0..30 {
        execute_quiet_run(&mut session);
        for trader in &session.economy.traders {
            for order in &trader.state.navigation.orders {
                let MoveTarget::Body { system, .. } = order.target else {
                    continue;
                };
                sailed += 1;
                let holder = factions.owner(system).unwrap();
                assert!(
                    holder == Faction::Federation
                        || factions.relation(Faction::Federation, holder) == Relation::Allied,
                    "{} sailed for {holder:?} space",
                    trader.name
                );
            }
        }
    }
    assert!(sailed > 0, "loyalists should still find friendly hauls");
}

#[test]
fn smugglers_slip_past_raiders_their_escape_chance_allows() {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 100;
    let catalog = &mut session.economy.rules.personality_catalog;
    catalog.personalities[Personality::Smuggler.index()].raid_escape_percent = 100;
    for trader in &mut session.economy.traders {
        trader.personality = Personality::Smuggler;
    }
    let mut slipped = 0;
    for _ in 0..20 {
        execute_quiet_run(&mut session);
        for trader in &session.economy.traders {
            assert!(
                !trader.last_action.contains("Destroyed"),
                "{}",
                trader.last_action
            );
            assert!(
                !trader.last_action.contains("Fought"),
                "{}",
                trader.last_action
            );
            slipped += usize::from(trader.last_action == "Slipped past raiders");
        }
    }
    assert!(slipped > 0, "under-way smugglers meet raiders every run");
}
