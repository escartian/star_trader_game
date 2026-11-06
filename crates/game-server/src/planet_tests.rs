use super::{Session, test_support::move_fleet};
use game_core::{
    BodyId, BodyKind, SystemId,
    economy::{Commodity, Credits, EconomyError, MarketId, TradeSide},
    fleet::{FleetId, ShipKind},
    movement::MoveTarget,
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

fn succeed(session: &mut Session, command: ClientCommand) {
    let response = session.handle_offline(command);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}
fn money(session: &Session) -> u128 {
    u128::from(session.economy.credits.0)
        + session
            .economy
            .markets
            .iter()
            .map(|market| u128::from(market.credits.0))
            .sum::<u128>()
}
fn visit_planet(session: &mut Session) {
    move_fleet(
        session,
        ShipId(1),
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
    );
}
fn trade(session: &mut Session, side: TradeSide) -> ClientCommand {
    ClientCommand::Trade {
        ship: ShipId(1),
        commodity: Commodity::Food,
        quantity: 1,
        side,
        expected_revision: session.state.revision,
    }
}

#[test]
fn planets_have_distinct_markets_and_specialization_changes_production() {
    let mut session = Session::default();
    assert_eq!(session.economy.markets.len(), 9);
    for (index, system) in session.systems.iter().enumerate() {
        for body in &system.bodies {
            let market = session.economy.market(MarketId {
                system: system.id,
                body: body.id,
            });
            if matches!(body.kind, BodyKind::RockyPlanet | BodyKind::GasGiant) {
                let profile = body.planet.as_ref().unwrap();
                let market = market.unwrap();
                let template = session
                    .economy
                    .rules
                    .planet_catalog
                    .template(index, body.kind)
                    .unwrap();
                let base = &session.economy.rules.markets[template.market_template];
                assert_eq!(
                    market.goods[profile.specialization.index()].production,
                    (base.production[profile.specialization.index()] + 2).min(1000)
                );
                assert_eq!(market.credits, template.starting_credits);
            } else if body.kind != BodyKind::Station {
                assert!(market.is_err());
            }
        }
    }
    visit_planet(&mut session);
    let other_id = MarketId {
        system: SystemId(2),
        body: BodyId(1),
    };
    let other = session.economy.market(other_id).unwrap().clone();
    let initial = money(&session);
    let tick = session.state.tick;
    let command = trade(&mut session, TradeSide::Buy);
    succeed(&mut session, command);
    assert_eq!(session.economy.market(other_id).unwrap(), &other);
    assert_eq!(session.state.tick, tick);
    assert_eq!(money(&session), initial);
    assert_eq!(
        session
            .economy
            .current_market(&session.state.ships[0])
            .unwrap()
            .id,
        MarketId {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
}

#[test]
fn local_treasury_limits_sales_and_purchases_fund_the_market_without_new_money() {
    let mut session = Session::default();
    visit_planet(&mut session);
    let id = session
        .economy
        .current_market(&session.state.ships[0])
        .unwrap()
        .id;
    session.economy.market_mut(id).unwrap().credits = Credits(0);
    session.state.ships[0].cargo[0] = 1;
    let initial = money(&session);
    let before = session.snapshot(Vec::new());
    let command = trade(&mut session, TradeSide::Sell);
    assert_eq!(
        session.handle_offline(command),
        ServerMessage::CommandRejected(EconomyError::MarketInsufficientCredits.into())
    );
    assert_eq!(session.snapshot(Vec::new()), before);
    let command = trade(&mut session, TradeSide::Buy);
    succeed(&mut session, command);
    assert!(session.economy.market(id).unwrap().credits.0 > 0);
    let command = trade(&mut session, TradeSide::Sell);
    succeed(&mut session, command);
    assert_eq!(money(&session), initial);
    session.state.ships[0].fuel -= 1;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::Refuel {
            ship: ShipId(1),
            units: 1,
            expected_revision: revision,
        },
    );
    assert_eq!(money(&session), initial);
    assert_eq!(session.state.tick, 4);
}

#[test]
fn planet_shipyards_buy_sell_and_restock_without_reusing_ids_or_fleets() {
    let mut session = Session::default();
    visit_planet(&mut session);
    let id = session
        .economy
        .current_market(&session.state.ships[0])
        .unwrap()
        .id;
    session.economy.market_mut(id).unwrap().ship_stock[ShipKind::Scout.index()] = 1;
    let initial = money(&session);
    let buy = ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: session.state.revision,
    };
    succeed(&mut session, buy.clone());
    assert_eq!(
        session.state.ships[1].location,
        session.state.ships[0].location
    );
    assert_eq!(session.economy.market(id).unwrap().ship_stock[1], 0);
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle_offline(buy),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    let revision = session.state.revision;
    assert_eq!(
        session.handle_offline(ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: revision
        }),
        ServerMessage::CommandRejected(EconomyError::ShipOutOfStock.into())
    );
    assert_eq!(session.snapshot(Vec::new()), before);
    succeed(
        &mut session,
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(2)],
            expected_revision: revision,
        },
    );
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SellShip {
            ship: ShipId(2),
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.ships.len(), 1);
    assert_eq!(session.state.fleets.len(), 1);
    assert_eq!(session.economy.market(id).unwrap().ship_stock[1], 1);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.ships[1].id, ShipId(3));
    assert_eq!(session.state.fleets[0].ships, [ShipId(1), ShipId(3)]);
    assert_eq!(session.state.next_fleet_id, 3);
    assert_eq!(money(&session), initial);
    assert_eq!(session.state.tick, 4);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("planets.json");
    session.save(&path).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_none());
    assert_eq!(restored.economy, session.economy);
    assert_eq!(restored.systems, session.systems);
    assert_eq!(restored.state, session.state);
}

#[test]
fn rejected_sales_and_renames_preserve_complete_state() {
    let mut session = Session::default();
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle_offline(ClientCommand::SellShip {
            ship: ShipId(1),
            expected_revision: 0
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    succeed(
        &mut session,
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: 0,
        },
    );
    session.state.ships[1].cargo[0] = 1;
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle_offline(ClientCommand::SellShip {
            ship: ShipId(2),
            expected_revision: 1
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    for name in [
        " ".to_owned(),
        "Fleet\nOops".to_owned(),
        "x".repeat(65),
        "星".repeat(64),
    ] {
        assert!(matches!(
            session.handle_offline(ClientCommand::RenameFleet {
                fleet: FleetId(1),
                name,
                expected_revision: 1
            }),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    succeed(
        &mut session,
        ClientCommand::RenameFleet {
            fleet: FleetId(1),
            name: "  Aurora Traders  ".into(),
            expected_revision: 1,
        },
    );
    assert_eq!(session.state.fleets[0].name, "Aurora Traders");
    assert_eq!(session.state.tick, 0);
}
