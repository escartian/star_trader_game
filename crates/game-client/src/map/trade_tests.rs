//! Trade buttons open the market where the chosen fleet actually is.
use super::*;

fn frame(ctx: &egui::Context, map: &mut Map, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| map.ui(ctx),
    )
}

fn texts(output: &egui::FullOutput) -> Vec<(String, Pos2)> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some((
                text.galley.job.text.clone(),
                text.pos + text.galley.size() / 2.0,
            )),
            _ => None,
        })
        .collect()
}

fn click(ctx: &egui::Context, map: &mut Map, label: &str) {
    let mut output = frame(ctx, map, Vec::new());
    for _ in 0..3 {
        output = frame(ctx, map, Vec::new());
    }
    let (_, point) = texts(&output)
        .into_iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("no {label:?} in {:?}", texts(&output)));
    for pressed in [true, false] {
        frame(
            ctx,
            map,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn switching_fleet_trade_opens_the_planet_market_it_orbits() {
    let ctx = egui::Context::default();
    let authority = game_server::Session::multi_fleet_scenario(1).unwrap();
    let systems = match authority
        .clone()
        .handle(game_protocol::ClientCommand::GetGalaxy)
    {
        game_protocol::ServerMessage::GalaxySnapshot { systems, .. } => systems,
        _ => unreachable!(),
    };
    let mut map = Map::new(systems, authority);
    map.open_selected_system();
    click(&ctx, &mut map, "Scout Wing · TRADE");
    let mut output = frame(&ctx, &mut map, Vec::new());
    for _ in 0..3 {
        output = frame(&ctx, &mut map, Vec::new());
    }
    let shown: Vec<_> = texts(&output).into_iter().map(|(text, _)| text).collect();
    assert!(shown.iter().any(|text| text == "MARKET"), "{shown:?}");
    assert!(shown.iter().any(|text| text == "Terra"), "{shown:?}");
    assert_eq!(
        map.session.selected_ship().kind,
        game_core::fleet::ShipKind::Scout
    );
}

#[test]
fn max_fills_the_hold_all_sells_it_back_and_full_refuels() {
    use game_protocol::{ClientCommand, ServerMessage};
    let ctx = egui::Context::default();
    let mut authority = game_server::Session::default();
    authority.set_encounter_chance(0);
    let systems = match authority.handle(ClientCommand::GetGalaxy) {
        ServerMessage::GalaxySnapshot { systems, .. } => systems,
        _ => unreachable!(),
    };
    // One run to the inner world's planet market leaves the tank short of full.
    for command in [
        ClientCommand::Move {
            ship: game_core::travel::ShipId(1),
            target: game_core::movement::MoveTarget::Body {
                system: game_core::SystemId(1),
                body: game_core::BodyId(1),
            },
            expected_revision: 0,
        },
        ClientCommand::ExecuteNavigationRun {
            expected_revision: 1,
        },
    ] {
        assert!(matches!(
            authority.handle(command),
            ServerMessage::SessionSnapshot { .. }
        ));
    }
    let mut map = Map::new(systems, authority);
    map.session.open_market();
    let food = game_core::economy::Commodity::Food.index();
    click(&ctx, &mut map, "Max");
    let ship = map.session.selected_ship();
    assert_eq!(ship.cargo[food], ship.cargo_capacity, "Max fills the hold");
    click(&ctx, &mut map, "All");
    assert_eq!(map.session.selected_ship().cargo_used(), 0, "All sells it");
    assert!(map.session.selected_ship().fuel < map.session.selected_ship().fuel_capacity);
    click(&ctx, &mut map, "Full");
    let ship = map.session.selected_ship();
    assert_eq!(ship.fuel, ship.fuel_capacity, "Full refuels");
    // With full tanks, Refuel is disabled instead of reporting an over-capacity error.
    let notice = map.session.notice().to_owned();
    click(&ctx, &mut map, "Refuel");
    assert_eq!(map.session.notice(), notice, "no error");
    let ship = map.session.selected_ship();
    assert_eq!(ship.fuel, ship.fuel_capacity);
}

/// The labels a few settled frames draw.
fn labels(ctx: &egui::Context, map: &mut Map) -> Vec<String> {
    let mut output = frame(ctx, map, Vec::new());
    for _ in 0..3 {
        output = frame(ctx, map, Vec::new());
    }
    texts(&output).into_iter().map(|(text, _)| text).collect()
}

/// System 2's trade port, the other end of the routes below.
fn meridian_port(systems: &[game_core::StarSystem]) -> game_core::BodyId {
    systems
        .iter()
        .find(|system| system.id == game_core::SystemId(2))
        .and_then(|system| {
            system
                .bodies
                .iter()
                .find(|body| body.kind == game_core::BodyKind::Station)
        })
        .unwrap()
        .id
}

#[test]
fn the_flagship_has_no_trade_route_button() {
    use game_core::SystemId;
    let ctx = egui::Context::default();
    let mut authority = game_server::Session::default();
    authority.set_encounter_chance(0);
    let systems = authority.systems().to_vec();
    let port = meridian_port(&systems);
    let mut map = Map::new(systems, authority);
    map.session.select_destination(SystemId(2), Some(port));
    assert!(
        !labels(&ctx, &mut map)
            .iter()
            .any(|text| text == "TRADE ROUTE")
    );
}

#[test]
fn a_secondary_fleet_starts_a_route_from_the_destination_card_and_ends_it() {
    use game_core::{SystemId, fleet::FleetId, travel::ShipId};
    let ctx = egui::Context::default();
    // Scout Wing (ship 2) waits in orbit of a planet with a market.
    let mut authority = game_server::Session::multi_fleet_scenario(1).unwrap();
    authority.set_encounter_chance(0);
    let systems = authority.systems().to_vec();
    let port = meridian_port(&systems);
    let mut map = Map::new(systems, authority);
    map.session.select_map_fleet(ShipId(2));
    map.session.select_destination(SystemId(2), Some(port));
    click(&ctx, &mut map, "TRADE ROUTE");
    let fleet = (1..=4)
        .map(FleetId)
        .find(|fleet| map.session.trade_route(*fleet).is_some())
        .expect("route started");
    assert_ne!(fleet, FleetId(1), "not the flagship");
    let route = map.session.trade_route(fleet).unwrap();
    assert_eq!(route.stops[1].system, SystemId(2));
    click(&ctx, &mut map, "Fleets");
    click(&ctx, &mut map, "End route");
    assert!(map.session.trade_route(fleet).is_none());
}

#[test]
fn a_contact_can_be_resolved_from_the_run_card_without_opening_navigation() {
    let ctx = egui::Context::default();
    let mut authority = game_server::Session::with_seed(2);
    let systems = match authority.handle(game_protocol::ClientCommand::GetGalaxy) {
        game_protocol::ServerMessage::GalaxySnapshot { systems, .. } => systems,
        _ => unreachable!(),
    };
    let mut map = Map::new(systems, authority);
    map.session.capture_encounter().unwrap();
    map.session.close_navigation();
    assert!(!map.session.contact_fleets().is_empty());
    // The pass button reads by contact ("Leave it", "Decline", "Bypass", …).
    let kind = map.session.pending_contact_kind().unwrap();
    let aboard = crate::session::contact_aboard_nothing();
    click(
        &ctx,
        &mut map,
        crate::session::contact_pass_label(kind, &aboard),
    );
    assert!(map.session.contact_fleets().is_empty());
}
