//! Fleet orders: one bulk trade split among the holds of a fleet at a port.
use super::{LOCAL_PLAYER, Session, combat_tests::reload};
use game_core::{
    economy::{Commodity, EconomyError, TradeSide},
    fleet::ShipKind,
    fleet_trade::LoadOrder,
    travel::ShipId,
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

const GOOD: Commodity = Commodity::Food;

/// The starting ship and a freighter in its fleet, both at the start port.
fn two_ships() -> Session {
    let mut session = Session::default();
    let fleet = session.state.fleets[0].id;
    let spec = session
        .economy
        .rules
        .ship_catalog
        .spec(ShipKind::Freighter)
        .unwrap()
        .clone();
    session.state.add_ship(LOCAL_PLAYER, fleet, &spec).unwrap();
    for ship in &mut session.state.ships {
        ship.cargo = [0; 6];
    }
    session
}

fn order(session: &mut Session, quantity: u32, side: TradeSide, order: LoadOrder) -> ServerMessage {
    session.handle(ClientCommand::FleetTrade {
        ship: ShipId(1),
        commodity: GOOD,
        quantity,
        side,
        order,
        expected_revision: session.state.revision,
    })
}

fn money(session: &Session) -> u64 {
    session.economy.credits.0
        + session
            .economy
            .markets
            .iter()
            .map(|market| market.credits.0)
            .sum::<u64>()
}

fn held(session: &Session) -> Vec<u32> {
    session
        .state
        .ships
        .iter()
        .map(|ship| ship.cargo[GOOD.index()])
        .collect()
}

#[test]
fn an_even_fleet_order_fills_both_holds_and_conserves_goods_and_credits() {
    let mut session = two_ships();
    let before = money(&session);
    let ServerMessage::SessionSnapshot { events, .. } =
        order(&mut session, 20, TradeSide::Buy, LoadOrder::Even)
    else {
        panic!("the order is accepted");
    };
    let [SessionEvent::FleetTraded { quote, split }] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert_eq!(quote.quantity, 20);
    assert_eq!(split.len(), 2, "both ships take a share");
    assert_eq!(held(&session).iter().sum::<u32>(), 20);
    assert!(held(&session).iter().all(|units| *units > 0));
    assert_eq!(money(&session), before);
    reload(&session);
    // Selling everything empties both holds.
    assert!(matches!(
        order(&mut session, 20, TradeSide::Sell, LoadOrder::Even),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert_eq!(held(&session), vec![0, 0]);
    assert_eq!(money(&session), before);
}

#[test]
fn the_selected_ship_order_uses_one_hold_and_an_oversized_order_changes_nothing() {
    let mut session = two_ships();
    let one = session.state.ships[0].cargo_capacity;
    let both = one + session.state.ships[1].cargo_capacity;
    let refused = order(&mut session, one + 1, TradeSide::Buy, LoadOrder::Selected);
    assert_eq!(
        refused,
        ServerMessage::CommandRejected(CommandError::Economy(EconomyError::CargoFull))
    );
    let snapshot = session.clone();
    let stock = session
        .economy
        .markets
        .iter()
        .map(|m| m.goods[GOOD.index()].stock)
        .max();
    if stock.is_some_and(|stock| stock > both) {
        assert!(matches!(
            order(&mut session, both + 1, TradeSide::Buy, LoadOrder::Even),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.state, snapshot.state);
    }
    assert!(matches!(
        order(&mut session, 5, TradeSide::Buy, LoadOrder::Selected),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert_eq!(held(&session), vec![5, 0]);
}
