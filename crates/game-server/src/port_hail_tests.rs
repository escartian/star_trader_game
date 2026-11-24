//! Hailing a trader docked at your port (trade only), and pursuing from a dock.
use super::{
    Session,
    combat_tests::{choose, reload},
    test_support::execute_quiet_run,
};
use game_core::{
    LocalPosition,
    economy::Commodity,
    fleet::FleetId,
    navigation::{EncounterChoice, EncounterKind},
    pursuit::Quarry,
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage};

/// The flagship docked at its system's port, trader 0 docked there too with
/// 10 ore, one quiet run on record.
fn at_port() -> (Session, u64, game_core::SystemId) {
    let mut session = Session::default();
    execute_quiet_run(&mut session);
    let system = session.state.ships[0].location.system().unwrap();
    session.state.ships[0].location = Location::Docked(system);
    session.state.navigation.orders.clear();
    let npc = &mut session.economy.traders[0];
    npc.state.navigation.orders.clear();
    npc.state.ships[0].location = Location::Docked(system);
    npc.state.ships[0].cargo = [0; 6];
    npc.state.ships[0].cargo[Commodity::Ore.index()] = 10;
    let id = npc.id;
    (session, id, system)
}

fn hail(session: &mut Session, trader: u64) -> ServerMessage {
    session.handle(ClientCommand::HailTrader {
        fleet: FleetId(1),
        trader,
        expected_revision: session.state.revision,
    })
}

#[test]
fn a_trader_docked_at_your_port_can_be_hailed_only_to_trade() {
    let (mut session, trader, _) = at_port();
    assert!(matches!(
        hail(&mut session, trader),
        ServerMessage::SessionSnapshot { .. }
    ));
    let contact = session.state.navigation.encounters[0].clone();
    assert_eq!(contact.kind, EncounterKind::TraderOffer);
    reload(&session);
    // Under the station's guns: no demands, no attacks.
    for choice in [EncounterChoice::Demand, EncounterChoice::Fight] {
        let refused = choose(&mut session, contact.id, choice);
        assert!(
            matches!(refused, ServerMessage::CommandRejected(_)),
            "{choice:?}"
        );
    }
    let held = |session: &Session| session.state.ships[0].cargo[Commodity::Ore.index()];
    let before = held(&session);
    choose(&mut session, contact.id, EncounterChoice::Trade);
    assert!(held(&session) > before, "bought its ore");
}

#[test]
fn a_docked_trader_is_out_of_reach_from_anywhere_but_its_port() {
    let (mut session, trader, system) = at_port();
    // The flagship in open space right by the port is still not "at" it.
    session.state.ships[0].location = Location::InSystem {
        system,
        position: LocalPosition {
            x_km: 1,
            y_km: 0,
            z_km: 0,
        },
    };
    assert!(matches!(
        hail(&mut session, trader),
        ServerMessage::CommandRejected(_)
    ));
    // An empty hold has nothing to offer at port.
    let (mut session, trader, _) = at_port();
    session.economy.traders[0].state.ships[0].cargo = [0; 6];
    assert!(matches!(
        hail(&mut session, trader),
        ServerMessage::CommandRejected(_)
    ));
}

#[test]
fn a_docked_flagship_can_set_off_in_pursuit() {
    let (mut session, trader, system) = at_port();
    session.economy.traders[0].state.ships[0].location = Location::InSystem {
        system,
        position: LocalPosition {
            x_km: 120_000_000,
            y_km: 0,
            z_km: 0,
        },
    };
    let response = session.handle(ClientCommand::Pursue {
        fleet: FleetId(1),
        quarry: Quarry::Trader(trader),
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    assert!(session.state.navigation.pursuit.is_some());
    assert!(
        session.state.navigation.order(FleetId(1)).is_some(),
        "course plotted from the dock"
    );
    execute_quiet_run(&mut session);
    assert!(
        !matches!(session.state.ships[0].location, Location::Docked(_)),
        "it left the port"
    );
}
