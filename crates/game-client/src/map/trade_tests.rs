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
    assert!(
        shown.iter().any(|text| text == "Sol Inner world"),
        "{shown:?}"
    );
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
}

#[test]
fn a_trade_route_starts_from_the_destination_card_and_ends_from_the_roster() {
    use game_core::{BodyKind, SystemId, fleet::FleetId};
    let ctx = egui::Context::default();
    let mut authority = game_server::Session::default();
    authority.set_encounter_chance(0);
    let systems = match authority.handle(game_protocol::ClientCommand::GetGalaxy) {
        game_protocol::ServerMessage::GalaxySnapshot { systems, .. } => systems,
        _ => unreachable!(),
    };
    let port = systems
        .iter()
        .find(|system| system.id == SystemId(2))
        .and_then(|system| {
            system
                .bodies
                .iter()
                .find(|body| body.kind == BodyKind::Station)
        })
        .unwrap()
        .id;
    let mut map = Map::new(systems, authority);
    map.session.select_destination(SystemId(2), Some(port));
    click(&ctx, &mut map, "TRADE ROUTE");
    let route = map.session.trade_route(FleetId(1)).expect("route started");
    assert_eq!(route.stops[1].system, SystemId(2));
    click(&ctx, &mut map, "End route");
    assert!(map.session.trade_route(FleetId(1)).is_none());
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
    click(&ctx, &mut map, "Bypass");
    assert!(map.session.contact_fleets().is_empty());
}
