//! The NPC population and the factions its traders fly for
//! (`game_core::npc::population`, route choice, `standing`).
use super::{
    GameSetup, Session,
    combat_tests::{choose, reload},
    faction_tests::claimed,
    piracy_tests::{hailed, meet_trader},
    test_support::execute_quiet_run,
};
use game_core::{
    economy::EconomyRules,
    factions::{Faction, Relation},
    generation::GalaxyConfig,
    movement::MoveTarget,
    navigation::EncounterChoice,
    npc::{MAX_TRADERS, trader_count},
};

#[test]
fn new_galaxies_field_traders_flying_for_their_home_ports_holder() {
    let setup = GameSetup {
        galaxy: GalaxyConfig {
            systems: 30,
            ..GalaxyConfig::default()
        },
        ..GameSetup::default()
    };
    let session =
        Session::new_game(5, EconomyRules::default(), Default::default(), &setup).unwrap();
    let economy = &session.economy;
    let ports = economy.markets.iter().filter(|m| !m.is_station).count();
    let systems: std::collections::HashSet<_> =
        economy.markets.iter().map(|m| m.id.system.0).collect();
    assert_eq!(economy.traders.len(), trader_count(systems.len(), ports));
    assert!((4..=MAX_TRADERS).contains(&economy.traders.len()));
    let names: std::collections::HashSet<_> = economy.traders.iter().map(|t| &t.name).collect();
    assert_eq!(names.len(), economy.traders.len());
    for trader in &economy.traders {
        let home = trader.home.expect("new traders record their home port");
        assert_eq!(trader.affiliation, economy.factions.owner(home.system));
        assert_eq!(trader.state.fleets[0].captain.species, trader.species);
    }
    let species: std::collections::HashSet<_> = economy
        .traders
        .iter()
        .map(|trader| trader.species)
        .collect();
    assert_eq!(species.len(), game_core::species::Species::ALL.len());
    assert!(economy.traders.iter().any(|t| t.affiliation.is_some()));
    assert_eq!(reload(&session).economy.traders, economy.traders);
}

#[test]
fn saves_reject_traders_flying_for_factions_not_in_play() {
    let mut session = Session::default();
    session.economy.traders[0].affiliation = Some(Faction::Empire);
    assert!(session.export_json().is_err());
}

#[test]
fn faction_traders_never_sail_for_a_rival_port() {
    // Classic relations: the Empire is the Federation's rival.
    let mut session = claimed([
        Some(Faction::Federation),
        Some(Faction::Empire),
        Some(Faction::Empire),
    ]);
    for trader in &mut session.economy.traders {
        trader.affiliation = Some(Faction::Federation);
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
                assert_ne!(
                    factions.relation(Faction::Federation, holder),
                    Relation::Rival,
                    "{} sailed for {holder:?} space",
                    trader.name
                );
            }
        }
    }
    assert!(sailed > 0, "traders should still find friendly hauls");
}

#[test]
fn robbing_a_faction_trader_costs_its_own_faction_too() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    let system = session.state.ships[0].location.system().unwrap();
    let mut holders = [None; 3];
    holders[system.0 as usize - 1] = Some(Faction::Federation);
    session.economy.factions = claimed(holders).economy.factions;
    // The Alliance is neutral to the Federation, allied to the Republic and
    // rival to the Empire.
    session.economy.traders[0].affiliation = Some(Faction::Alliance);
    let contact = hailed(&mut session, fleet, trader);
    choose(&mut session, contact, EncounterChoice::AutoResolve);
    let factions = &session.economy.factions;
    assert_eq!(factions.standing(Faction::Federation), -25);
    assert_eq!(factions.standing(Faction::Alliance), -8);
    // Half of each spreads: −12 and −4 to allies, +12 and +4 to rivals.
    assert_eq!(factions.standing(Faction::Republic), -16);
    assert_eq!(factions.standing(Faction::Empire), 16);
    assert!(factions.wanted_runs(Faction::Federation) > 0);
    assert!(factions.wanted_runs(Faction::Alliance) > 0);
    reload(&session);
}
