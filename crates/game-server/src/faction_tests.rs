//! Factions at setup and in play (`game_core::factions`, `standing`).
use super::{
    GameSetup, Session,
    band_tests::{near_band, run},
    combat_tests::{choose, reload},
    navigation_tests::succeed,
    piracy_tests::{hailed, meet_trader},
};
use game_core::{
    SystemId,
    economy::{Commodity, EconomyError, EconomyRules, TradeSide},
    factions::{Faction, FactionSetup, Tier},
    navigation::{EncounterChoice, NavigationEvent},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

/// The default fixture with all four factions in play (classic relations),
/// `holders` claiming systems 1, 2 and 3 in turn, and neutral standing.
pub(super) fn claimed(holders: [Option<Faction>; 3]) -> Session {
    let mut session = Session::default();
    let catalog = &session.economy.rules.faction_catalog;
    let mut factions = FactionSetup::default()
        .generate(0, &session.systems, catalog)
        .unwrap();
    factions.territory = holders
        .iter()
        .enumerate()
        .filter_map(|(index, holder)| Some((SystemId(index as u64 + 1), (*holder)?)))
        .collect();
    session.economy.factions = factions;
    session
}

pub(super) fn standing_events(message: &ServerMessage) -> Vec<(Faction, i32)> {
    let ServerMessage::SessionSnapshot { events, .. } = message else {
        return Vec::new();
    };
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::StandingChanged {
                faction, change, ..
            } => Some((*faction, *change)),
            _ => None,
        })
        .collect()
}

#[test]
fn new_games_set_up_factions_and_saves_keep_them() {
    let setup = GameSetup {
        factions: FactionSetup {
            allegiance: Some(Faction::Republic),
            ..FactionSetup::default()
        },
        ..GameSetup::default()
    };
    let session =
        Session::new_game(3, EconomyRules::default(), Default::default(), &setup).unwrap();
    let factions = &session.economy.factions;
    assert_eq!(factions.in_play, Faction::ALL);
    assert_eq!(
        factions.owner(session.systems[0].id),
        Some(Faction::Republic)
    );
    assert_eq!(factions.standing(Faction::Republic), 30);
    let restored = reload(&session);
    assert_eq!(restored.economy.factions, session.economy.factions);

    let off = GameSetup {
        factions: FactionSetup {
            in_play: [false; 4],
            ..FactionSetup::default()
        },
        ..GameSetup::default()
    };
    let session = Session::new_game(3, EconomyRules::default(), Default::default(), &off).unwrap();
    assert!(!session.economy.factions.is_active());
    let disloyal = GameSetup {
        factions: FactionSetup {
            in_play: [true, false, false, false],
            allegiance: Some(Faction::Empire),
            ..FactionSetup::default()
        },
        ..GameSetup::default()
    };
    assert!(Session::new_game(3, EconomyRules::default(), Default::default(), &disloyal).is_err());
}

#[test]
fn standing_sets_prices_and_hostile_ports_refuse_trade() {
    let mut session = claimed([Some(Faction::Federation), None, None]);
    let ship = session.state.ships[0].clone();
    let quote = |session: &Session| {
        session
            .economy
            .quote(&ship, Commodity::Ore, 10, TradeSide::Buy, 0)
            .map(|quote| quote.total.0)
    };
    let list = quote(&session).unwrap();
    session.economy.factions.standing[Faction::Federation.index()] = 60;
    assert_eq!(
        session.economy.port_terms(&ship).holder,
        Some((Faction::Federation, Tier::Honored))
    );
    assert_eq!(quote(&session).unwrap(), (list * 90).div_ceil(100));
    session.economy.factions.standing[Faction::Federation.index()] = -60;
    assert_eq!(quote(&session), Err(EconomyError::PortClosed));
    // Fuel is still sold, at half again the price.
    session.state.ships[0].fuel -= 2;
    let before = session.economy.credits.0;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::Refuel {
            ship: ship.id,
            units: 2,
            expected_revision: revision,
        },
    );
    let paid = before - session.economy.credits.0;
    assert_eq!(
        paid,
        (2 * session.economy.fuel_price().0 * 150).div_ceil(100)
    );
    let response = session.handle(ClientCommand::UpgradeHold {
        ship: ship.id,
        expected_revision: session.state.revision,
    });
    assert!(matches!(
        response,
        ServerMessage::CommandRejected(CommandError::Economy(EconomyError::PortClosed))
    ));
}

#[test]
fn selling_in_a_factions_port_wins_its_favour() {
    let mut session = claimed([Some(Faction::Republic), None, None]);
    session.economy.rules.faction_catalog.factions[Faction::Republic.index()]
        .sale_credits_per_point = 100;
    session.state.ships[0].cargo[Commodity::Ore.index()] = 3;
    let response = session.handle(ClientCommand::Trade {
        ship: session.state.ships[0].id,
        commodity: Commodity::Ore,
        quantity: 3,
        side: TradeSide::Sell,
        expected_revision: session.state.revision,
    });
    let changes = standing_events(&response);
    let gained = session.economy.factions.standing(Faction::Republic);
    assert!(gained > 0);
    assert!(
        changes.contains(&(Faction::Republic, gained)),
        "{changes:?}"
    );
}

#[test]
fn piracy_costs_standing_with_the_systems_holder_and_its_friends() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    let system = session.state.ships[0].location.system().unwrap();
    let mut holders = [None; 3];
    holders[system.0 as usize - 1] = Some(Faction::Federation);
    let economy = claimed(holders).economy.factions;
    session.economy.factions = economy;
    let contact = hailed(&mut session, fleet, trader);
    let response = choose(&mut session, contact, EncounterChoice::AutoResolve);
    let factions = &session.economy.factions;
    // Classic: the Republic is the Federation's ally, the Empire its rival.
    assert_eq!(factions.standing(Faction::Federation), -25);
    assert_eq!(factions.standing(Faction::Republic), -12);
    assert_eq!(factions.standing(Faction::Empire), 12);
    assert!(standing_events(&response).contains(&(Faction::Federation, -25)));
    reload(&session);
}

#[test]
fn destroying_raiders_in_a_factions_space_wins_its_favour() {
    let mut session = near_band(1_000_000);
    session.economy.factions = claimed([None, Some(Faction::Empire), None])
        .economy
        .factions;
    // A lethal player: every raider in reach dies in the first volley.
    session.state.ships[0].kind = game_core::fleet::ShipKind::Patrol;
    let mut contact = None;
    for _ in 0..4 {
        let events = run(&mut session);
        contact = events.into_iter().find_map(|event| match event {
            NavigationEvent::EncounterFound(found) if found.band.is_some() => Some(found),
            _ => None,
        });
        if contact.is_some() {
            break;
        }
    }
    let contact = contact.expect("the band struck");
    choose(&mut session, contact.id, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    let destroyed = report
        .side(game_core::combat::Side::Hostile)
        .filter(|ship| ship.destroyed)
        .count() as i32;
    assert!(destroyed > 0);
    let factions = &session.economy.factions;
    assert_eq!(factions.standing(Faction::Empire), 4 * destroyed);
    // Classic: the Alliance is the Empire's rival.
    assert_eq!(factions.standing(Faction::Alliance), -(4 * destroyed / 2));
    reload(&session);
}
