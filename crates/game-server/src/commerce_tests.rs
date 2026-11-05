use super::{LOCAL_PLAYER, Session};
use game_core::{
    SystemId,
    economy::{Commodity, Credits, EconomyError, TradeSide},
    travel::{PlayerId, ShipId, TravelError},
};
use game_protocol::{ClientCommand, ServerMessage};

fn succeed(session: &mut Session, command: ClientCommand) {
    let response = session.handle(command);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}

fn trade(session: &mut Session, commodity: Commodity, side: TradeSide) {
    succeed(
        session,
        ClientCommand::Trade {
            ship: ShipId(1),
            commodity,
            quantity: 20,
            side,
            expected_revision: session.state.revision,
        },
    );
}

fn travel(session: &mut Session, destination: SystemId) {
    let quote = session.quote(LOCAL_PLAYER, ShipId(1), destination).unwrap();
    succeed(
        session,
        ClientCommand::Travel {
            ship: ShipId(1),
            destination,
            expected_revision: quote.revision,
        },
    );
    session.advance(quote.duration_ticks);
}

fn refuel(session: &mut Session) {
    succeed(
        session,
        ClientCommand::Refuel {
            ship: ShipId(1),
            units: 40 - session.state.ship.fuel,
            expected_revision: session.state.revision,
        },
    );
}

#[test]
fn real_commands_complete_buy_travel_sell_refuel_and_upgrade_without_granted_credits() {
    let mut session = Session::default();
    for _ in 0..8 {
        trade(&mut session, Commodity::Food, TradeSide::Buy);
        travel(&mut session, SystemId(2));
        trade(&mut session, Commodity::Food, TradeSide::Sell);
        refuel(&mut session);
        trade(&mut session, Commodity::Ore, TradeSide::Buy);
        travel(&mut session, SystemId(1));
        trade(&mut session, Commodity::Ore, TradeSide::Sell);
        refuel(&mut session);
        if session.economy.credits >= session.economy.rules.hold_upgrade_price {
            break;
        }
    }
    assert!(session.economy.credits >= session.economy.rules.hold_upgrade_price);
    assert_eq!(session.economy.cargo_used(), 0);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::UpgradeHold {
            ship: ShipId(1),
            expected_revision: revision,
        },
    );
    assert!(session.economy.upgraded);
    assert_eq!(session.economy.cargo_capacity, 40);
}

#[test]
fn rejected_commerce_is_atomic_across_player_ship_and_market_state() {
    let mut session = Session::default();
    let original = session.snapshot(Vec::new());
    let commands = [
        ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 21,
            side: TradeSide::Buy,
            expected_revision: 0,
        },
        ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 1,
            side: TradeSide::Sell,
            expected_revision: 0,
        },
        ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Electronics,
            quantity: 20,
            side: TradeSide::Buy,
            expected_revision: 0,
        },
        ClientCommand::Refuel {
            ship: ShipId(1),
            units: 1,
            expected_revision: 0,
        },
        ClientCommand::UpgradeHold {
            ship: ShipId(1),
            expected_revision: 0,
        },
    ];
    for command in commands {
        assert!(matches!(
            session.handle(command),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), original);
    }
    let command = ClientCommand::Trade {
        ship: ShipId(1),
        commodity: Commodity::Food,
        quantity: 1,
        side: TradeSide::Buy,
        expected_revision: 0,
    };
    assert_eq!(
        session.handle_as(PlayerId(2), command.clone()),
        ServerMessage::CommandRejected(TravelError::NotOwner.into())
    );
    session.advance(1);
    let original = session.snapshot(Vec::new());
    assert_eq!(
        session.handle(command),
        ServerMessage::CommandRejected(TravelError::StaleQuote.into())
    );
    assert_eq!(session.snapshot(Vec::new()), original);
}

#[test]
fn panel_refreshes_do_not_produce_stock_or_duplicate_money() {
    let mut session = Session::default();
    let original = session.snapshot(Vec::new());
    for _ in 0..20 {
        session.handle(ClientCommand::GetSession);
        session.handle(ClientCommand::PreviewTrade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 1,
            side: TradeSide::Buy,
        });
    }
    assert_eq!(session.snapshot(Vec::new()), original);
    session.economy.credits = Credits(0);
    assert_eq!(
        session.handle(ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 1,
            side: TradeSide::Buy,
            expected_revision: 0
        }),
        ServerMessage::CommandRejected(EconomyError::InsufficientCredits.into())
    );
}
