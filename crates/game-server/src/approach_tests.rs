//! NPC traders that approach your flagship (`game_core::approach`).
use super::{
    LOCAL_PLAYER, Session,
    combat_tests::{choose, reload},
    faction_tests::claimed,
    piracy_tests::meet_trader,
    test_support::execute_quiet_run,
};
use game_core::{
    approach,
    economy::Commodity,
    factions::Faction,
    navigation::{EncounterChoice, EncounterKind, NavigationEvent},
    npc::Personality,
};
use game_protocol::ServerMessage;

/// Every personality approaches whenever it can.
fn eager(session: &mut Session) {
    for definition in &mut session.economy.rules.personality_catalog.personalities {
        definition.approach_percent = 100;
    }
}

/// Lets the traders near the flagship try their approach; returns the contact.
fn approached(session: &mut Session) -> Option<(u64, EncounterKind)> {
    let events = approach::approach_run(
        &mut session.state,
        &mut session.economy,
        LOCAL_PLAYER,
        &session.systems,
    )
    .unwrap();
    events.into_iter().find_map(|event| match event {
        NavigationEvent::EncounterFound(found) => Some((found.id, found.kind)),
        _ => None,
    })
}

fn wallets(session: &Session) -> u64 {
    session.economy.credits.0
        + session
            .economy
            .traders
            .iter()
            .map(|trader| trader.credits.0)
            .sum::<u64>()
}

#[test]
fn a_trader_alongside_sells_its_hold_below_list_and_keeps_the_money() {
    let mut session = Session::default();
    eager(&mut session);
    let (_, trader) = meet_trader(&mut session, 0);
    let (contact, kind) = approached(&mut session).expect("trader 0 is in reach");
    assert_eq!(kind, EncounterKind::TraderOffer);
    let npc = |session: &Session| {
        session
            .economy
            .traders
            .iter()
            .find(|npc| npc.id == trader)
            .unwrap()
            .clone()
    };
    assert_eq!(npc(&session).last_approach, session.state.navigation.run);
    reload(&session);
    let total = wallets(&session);
    let before = (session.economy.credits.0, npc(&session).credits.0);
    // Medicine is worth more than ore, so it is bought first.
    let medicine = approach::offer_price(&session.economy, Commodity::Medicine).0;
    let response = choose(&mut session, contact, EncounterChoice::Trade);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    let ours = session.state.ships[0].cargo;
    assert_eq!(ours[Commodity::Medicine.index()], 6);
    let paid = before.0 - session.economy.credits.0;
    assert!(paid >= 6 * medicine);
    assert_eq!(npc(&session).credits.0, before.1 + paid);
    assert_eq!(
        wallets(&session),
        total,
        "money changes hands, never appears"
    );
    assert!(session.state.navigation.encounters.is_empty());
    // The same trader rests before approaching again (others nearby may not).
    approached(&mut session);
    assert!(
        session
            .state
            .navigation
            .encounters
            .iter()
            .all(|contact| contact.trader.as_ref().is_none_or(|t| t.id != trader))
    );
    reload(&session);
}

#[test]
fn a_corsair_that_outguns_you_shakes_you_down_and_complying_costs_half_your_cargo() {
    let mut session = claimed([Some(Faction::Federation); 3]);
    eager(&mut session);
    let (_, trader) = meet_trader(&mut session, 0);
    session.economy.traders[0].personality = Personality::Corsair;
    session.economy.traders[0].state.ships[0].cargo = [0; 6];
    // A battered freighter is no match for the corsair.
    session.state.ships[0].hull_damage = 50;
    session.state.ships[0].cargo = [0; 6];
    session.state.ships[0].cargo[Commodity::Ore.index()] = 5;
    let (contact, kind) = approached(&mut session).unwrap();
    assert_eq!(kind, EncounterKind::Shakedown);
    reload(&session);
    // Hand-overs are not trade: a demand only works on hailed traders.
    let refused = choose(&mut session, contact, EncounterChoice::Trade);
    assert!(matches!(refused, ServerMessage::CommandRejected(_)));
    let standing = session.economy.factions.standing;
    choose(&mut session, contact, EncounterChoice::Comply);
    assert_eq!(session.state.ships[0].cargo[Commodity::Ore.index()], 2);
    let corsair = session
        .economy
        .traders
        .iter()
        .find(|npc| npc.id == trader)
        .unwrap();
    assert_eq!(corsair.state.ships[0].cargo[Commodity::Ore.index()], 3);
    assert_eq!(session.economy.factions.standing, standing);
    assert_eq!(session.state.navigation.piracies, 0);
    reload(&session);
}

#[test]
fn resisting_a_shakedown_is_self_defence_not_piracy() {
    let mut session = claimed([Some(Faction::Federation); 3]);
    eager(&mut session);
    meet_trader(&mut session, 0);
    session.economy.traders[0].personality = Personality::Corsair;
    session.state.ships[0].hull_damage = 50;
    session.state.ships[0].cargo[Commodity::Food.index()] = 4;
    let (contact, kind) = approached(&mut session).unwrap();
    assert_eq!(kind, EncounterKind::Shakedown);
    let standing = session.economy.factions.standing;
    choose(&mut session, contact, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert!(report.shakedown);
    assert_eq!(report.bounty.0, 0);
    assert_eq!(session.state.navigation.piracies, 0);
    assert_eq!(session.economy.factions.standing, standing);
    assert!(
        session
            .economy
            .factions
            .wanted
            .iter()
            .all(|runs| *runs == 0)
    );
    reload(&session);
}

#[test]
fn a_corsair_that_does_not_outgun_you_only_offers_to_trade() {
    let mut session = Session::default();
    eager(&mut session);
    meet_trader(&mut session, 0);
    session.economy.traders[0].personality = Personality::Corsair;
    let (_, kind) = approached(&mut session).unwrap();
    assert_eq!(kind, EncounterKind::TraderOffer);
}

#[test]
fn traders_come_to_an_exposed_flagship_during_ordinary_runs() {
    let mut session = Session::default();
    eager(&mut session);
    session.economy.rules.npc_raid_chance_percent = 0;
    meet_trader(&mut session, 0);
    for _ in 0..30 {
        execute_quiet_run(&mut session);
        if let Some(contact) = session.state.navigation.encounters.first() {
            assert!(contact.kind.with_trader(), "{:?}", contact.kind);
            reload(&session);
            return;
        }
    }
    panic!("no trader approached an exposed flagship in 30 runs");
}

#[test]
fn a_corsair_leaves_an_empty_fleet_alone_or_trades_with_it() {
    let mut session = Session::default();
    eager(&mut session);
    meet_trader(&mut session, 0);
    session.economy.traders[0].personality = Personality::Corsair;
    session.state.ships[0].hull_damage = 50;
    session.state.ships[0].cargo = [0; 6];
    let (_, kind) = approached(&mut session).unwrap();
    assert_eq!(kind, EncounterKind::TraderOffer);
}
