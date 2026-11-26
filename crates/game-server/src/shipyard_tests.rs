use super::{LOCAL_PLAYER, Session, combat_tests::reload};
use game_core::{
    SystemId,
    combat::gear::Gear,
    economy::{Commodity, Credits, ShipUpgrade},
    fleet::{FleetError, FleetId, ShipKind},
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

fn buy_scout(session: &mut Session) {
    let response = session.handle_offline(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}

#[test]
fn a_sale_moves_every_good_to_other_ships_at_the_same_berth() {
    let mut session = Session::default();
    buy_scout(&mut session);
    buy_scout(&mut session);
    let split = session.handle_offline(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: session.state.revision,
    });
    assert!(matches!(split, ServerMessage::SessionSnapshot { .. }));
    session.state.ships[0].cargo[Commodity::Ore.index()] = 15;
    session.state.ships[0].cargo[Commodity::Medicine.index()] = 4;
    let response = session.handle_offline(ClientCommand::SellShip {
        ship: ShipId(1),
        discard_overflow: false,
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { events, .. }
        if events.iter().any(|event| matches!(event, SessionEvent::ShipSold { discarded: 0, .. })))
    );
    assert_eq!(
        session.state.ship(ShipId(2)).unwrap().cargo[Commodity::Ore.index()],
        10
    );
    assert_eq!(
        session.state.ship(ShipId(3)).unwrap().cargo[Commodity::Ore.index()],
        5
    );
    assert_eq!(
        session.state.ship(ShipId(3)).unwrap().cargo[Commodity::Medicine.index()],
        4
    );
    reload(&session);
}

#[test]
fn overflow_requires_explicit_discard_and_loses_only_the_excess() {
    let mut session = Session::default();
    buy_scout(&mut session);
    session.state.ships[0].cargo[Commodity::Ore.index()] = 15;
    let before = session.clone();
    let rejected = session.handle_offline(ClientCommand::SellShip {
        ship: ShipId(1),
        discard_overflow: false,
        expected_revision: session.state.revision,
    });
    assert!(matches!(
        rejected,
        ServerMessage::CommandRejected(CommandError::Fleet(FleetError::SaleCargoOverflow))
    ));
    assert_eq!(session.state, before.state);
    assert_eq!(session.economy, before.economy);
    let response = session.handle_offline(ClientCommand::SellShip {
        ship: ShipId(1),
        discard_overflow: true,
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { events, .. }
        if events.iter().any(|event| matches!(event, SessionEvent::ShipSold { discarded: 5, .. })))
    );
    assert_eq!(
        session.state.ship(ShipId(2)).unwrap().cargo[Commodity::Ore.index()],
        10
    );
    reload(&session);
}

#[test]
fn a_ship_at_another_port_cannot_receive_sale_cargo() {
    let mut session = Session::default();
    buy_scout(&mut session);
    let split = session.handle_offline(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: session.state.revision,
    });
    assert!(matches!(split, ServerMessage::SessionSnapshot { .. }));
    session.state.ships[1].location = Location::Docked(SystemId(2));
    session.state.ships[0].cargo[Commodity::Ore.index()] = 3;
    assert_eq!(
        session.state.sale_cargo_room(LOCAL_PLAYER, ShipId(1)),
        Ok((3, 0))
    );
    let rejected = session.handle_offline(ClientCommand::SellShip {
        ship: ShipId(1),
        discard_overflow: false,
        expected_revision: session.state.revision,
    });
    assert!(matches!(
        rejected,
        ServerMessage::CommandRejected(CommandError::Fleet(FleetError::SaleCargoOverflow))
    ));
}

#[test]
fn refit_refunds_and_swaps_capacity_without_losing_cargo() {
    let mut session = Session::default();
    session.economy.credits = Credits(1_000_000);
    let fitted = session.handle_offline(ClientCommand::FitGear {
        ship: ShipId(1),
        gear: Gear::FreighterRacks,
        expected_revision: session.state.revision,
    });
    assert!(matches!(fitted, ServerMessage::SessionSnapshot { .. }));
    session.state.ships[0].cargo[Commodity::Ore.index()] = 30;
    let before = session.clone();
    let remove = ShipUpgrade::Gear(Gear::FreighterRacks);
    let rejected = session.handle_offline(ClientCommand::RefitShip {
        ship: ShipId(1),
        remove,
        install: None,
        expected_revision: session.state.revision,
    });
    assert!(matches!(rejected, ServerMessage::CommandRejected(_)));
    assert_eq!(session.state, before.state);
    assert_eq!(session.economy, before.economy);
    let wallet = session.economy.credits.0;
    let response = session.handle_offline(ClientCommand::RefitShip {
        ship: ShipId(1),
        remove,
        install: Some(ShipUpgrade::Hold),
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
    let ship = &session.state.ships[0];
    assert!(ship.upgraded);
    assert!(!ship.gear.contains(&Gear::FreighterRacks));
    assert_eq!(ship.cargo_capacity, 40);
    assert_eq!(ship.cargo[Commodity::Ore.index()], 30);
    assert_eq!(session.economy.credits.0, wallet + 54_000 - 150_000);
    reload(&session);
}

#[test]
fn removing_gear_and_the_original_hold_upgrade_refunds_their_resale_values() {
    let mut session = Session::default();
    session.economy.credits = Credits(1_000_000);
    let base = session.state.ships[0].cargo_capacity;
    let fit = session.handle_offline(ClientCommand::FitGear {
        ship: ShipId(1),
        gear: Gear::Afterburners,
        expected_revision: session.state.revision,
    });
    assert!(matches!(fit, ServerMessage::SessionSnapshot { .. }));
    let hold = session.handle_offline(ClientCommand::UpgradeHold {
        ship: ShipId(1),
        expected_revision: session.state.revision,
    });
    assert!(matches!(hold, ServerMessage::SessionSnapshot { .. }));
    let wallet = session.economy.credits.0;
    let remove_gear = session.handle_offline(ClientCommand::RefitShip {
        ship: ShipId(1),
        remove: ShipUpgrade::Gear(Gear::Afterburners),
        install: None,
        expected_revision: session.state.revision,
    });
    assert!(matches!(remove_gear, ServerMessage::SessionSnapshot { .. }));
    let remove_hold = session.handle_offline(ClientCommand::RefitShip {
        ship: ShipId(1),
        remove: ShipUpgrade::Hold,
        install: None,
        expected_revision: session.state.revision,
    });
    assert!(matches!(remove_hold, ServerMessage::SessionSnapshot { .. }));
    assert_eq!(session.economy.credits.0, wallet + 30_000 + 90_000);
    assert!(session.state.ships[0].gear.is_empty());
    assert!(!session.state.ships[0].upgraded);
    assert_eq!(session.state.ships[0].cargo_capacity, base);
    reload(&session);
}
