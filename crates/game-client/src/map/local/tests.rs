use super::*;
use egui::Vec2;
use game_core::BodyKind;

#[test]
fn local_distances_are_kilometers_and_do_not_depend_on_view_scale() {
    let system = game_core::demo_galaxy().remove(0);
    let body = &system.bodies[0];
    assert!(kilometers(body.position).length() > 1_000_000.0);
    assert_eq!(
        view_position(body) * KM_PER_VIEW_UNIT,
        kilometers(body.position)
    );
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
    let mut local = LocalView::new(system);
    local.camera.yaw = 1.0;
    local.camera.zoom = 4.0;
    local.focus();
    let selected = &local.system.bodies[0];
    assert_eq!(
        local.camera.project(view_position(selected), rect),
        rect.center()
    );
    assert!(pick(&local.system, &local.camera, rect, rect.center()).contains(&selected.id));
}

#[test]
fn local_overlaps_are_not_resolved_arbitrarily() {
    let mut system = game_core::demo_galaxy().remove(0);
    system.bodies[1].position = system.bodies[0].position;
    let camera = Camera::default();
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
    let point = camera.project(view_position(&system.bodies[0]), rect);
    let candidates = pick(&system, &camera, rect, point);
    assert!(candidates.contains(&system.bodies[0].id));
    assert!(candidates.contains(&system.bodies[1].id));
}

#[test]
fn selected_station_inspector_tessellates_at_phone_and_desktop_sizes() {
    let ctx = egui::Context::default();
    let mut local = LocalView::new(game_core::demo_galaxy().remove(0));
    local.selected = Some(BodyId(3));
    for size in [Vec2::new(320.0, 480.0), Vec2::new(1000.0, 720.0)] {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            },
            |ctx| {
                assert!(!local.ui(ctx, &[], &[]));
            },
        );
        assert!(
            !ctx.tessellate(output.shapes, output.pixels_per_point)
                .is_empty()
        );
    }
}

#[test]
fn schematic_moon_is_separate_pickable_and_keeps_saved_coordinates() {
    let system = game_core::demo_galaxy().remove(0);
    let moon = system
        .bodies
        .iter()
        .find(|body| body.kind == BodyKind::Moon)
        .unwrap();
    let parent = moon_parent(&system, moon).unwrap();
    let original = system.clone();
    let mut local = LocalView::new(system.clone());
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 400.0));
    let moon_point = local.camera.project(display_position(&system, moon), rect);
    let planet_point = local
        .camera
        .project(display_position(&system, parent), rect);
    // Clear of each other even zoomed in: a rocky planet's and a moon's
    // markers (11 and 5 points, scaled by up to 1.5) touch at 24 points.
    assert!(moon_point.distance(planet_point) > 26.0);
    assert_eq!(
        pick(&system, &local.camera, rect, moon_point),
        vec![moon.id]
    );
    local.selected = Some(moon.id);
    local.focus();
    assert_eq!(
        local.camera.project(display_position(&system, moon), rect),
        rect.center()
    );
    assert_eq!(system, original);
    assert_eq!(local.system, original);
}

#[test]
fn every_generated_moon_is_drawn_beside_its_planet_never_across_the_star() {
    let mut systems = game_core::generation::generate_galaxy_with(
        3,
        game_core::generation::GalaxyConfig::default(),
    )
    .unwrap();
    game_core::orbits::place(&mut systems, 0);
    let mut moons = 0;
    for system in &systems {
        for moon in system
            .bodies
            .iter()
            .filter(|body| body.kind == BodyKind::Moon)
        {
            let parent = moon_parent(system, moon).unwrap();
            let (shown, planet) = (
                display_position(system, moon),
                display_position(system, parent),
            );
            let from_planet = shown.distance(planet);
            assert!(from_planet > 0.5, "{}: drawn on its planet", moon.name);
            assert!(
                from_planet < shown.length(),
                "{} in {}: nearer the star than its planet",
                moon.name,
                system.name
            );
            moons += 1;
        }
    }
    assert!(moons > 20, "{moons} moons checked");
}

#[test]
fn a_moon_keeps_one_drawn_distance_from_its_planet_all_the_way_round() {
    let mut systems = game_core::generation::generate_galaxy_with(
        3,
        game_core::generation::GalaxyConfig::default(),
    )
    .unwrap();
    // The drawn distance of each moon at every placement through its orbit.
    let mut distances: std::collections::HashMap<String, Vec<f64>> = Default::default();
    for tick in (0..2_000).step_by(25) {
        game_core::orbits::place(&mut systems, tick);
        for system in &systems {
            for moon in system
                .bodies
                .iter()
                .filter(|body| body.kind == BodyKind::Moon)
            {
                let parent = moon_parent(system, moon).unwrap();
                let shown = display_position(system, moon);
                distances
                    .entry(format!("{} in {}", moon.name, system.name))
                    .or_default()
                    .push(shown.distance(display_position(system, parent)));
            }
        }
    }
    for (moon, seen) in &distances {
        let (low, high) = seen.iter().fold((f64::MAX, f64::MIN), |(low, high), d| {
            (low.min(*d), high.max(*d))
        });
        // Planets drift a little as they orbit the star, so allow a small
        // change; the old inward cap jumped by far more.
        assert!(
            high - low < 0.25 * high,
            "{moon} jumps between {low:.2} and {high:.2}"
        );
    }
    assert!(distances.len() > 20);
}

#[test]
fn local_fleet_and_body_clicks_emit_distinct_selection_and_destination_intents() {
    use game_core::{
        SystemId,
        fleet::{FleetId, ShipKind},
        travel::ShipId,
    };
    use game_protocol::{ClientCommand, ServerMessage};
    let mut authority = game_server::Session::default();
    authority.handle(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    });
    authority.handle(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: 1,
    });
    let ServerMessage::SessionSnapshot { state, .. } = authority.handle(ClientCommand::GetSession)
    else {
        panic!("snapshot missing");
    };
    let mut local = LocalView::new(game_core::demo_galaxy()[0].clone());
    let ctx = egui::Context::default();
    let screen = Some(Rect::from_min_size(
        Pos2::ZERO,
        egui::Vec2::new(1200.0, 900.0),
    ));
    for _ in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                ..Default::default()
            },
            |ctx| {
                local.ui(ctx, &state.ships, &state.fleets);
            },
        );
    }
    let fleet_point = local
        .fleet_hits
        .iter()
        .find(|hit| hit.fleet == FleetId(2))
        .unwrap()
        .bounds
        .center();
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                events: vec![
                    egui::Event::PointerMoved(fleet_point),
                    egui::Event::PointerButton {
                        pos: fleet_point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                local.ui(ctx, &state.ships, &state.fleets);
            },
        );
    }
    assert_eq!(local.take_fleet_selection(), Some(ShipId(2)));
    assert!(local.take_movement_request().is_none());
    // Body tiles (in the Selection sheet) use the same destination intent as
    // canvas picks; neither moves authority by itself.
    let frame = |ctx: &egui::Context, local: &mut LocalView| {
        egui::SidePanel::right("sheet").show(ctx, |ui| local.selection_sheet(ui));
        local.ui(ctx, &state.ships, &state.fleets);
    };
    let output = ctx.run(
        egui::RawInput {
            screen_rect: screen,
            ..Default::default()
        },
        |ctx| frame(ctx, &mut local),
    );
    // The sheet draws before the map, so its tile is the first "Jove".
    let tile = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape
                && text.galley.job.text == "Jove"
            {
                Some(text.pos + text.galley.size() / 2.0)
            } else {
                None
            }
        })
        .unwrap();
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                events: vec![
                    egui::Event::PointerMoved(tile),
                    egui::Event::PointerButton {
                        pos: tile,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| frame(ctx, &mut local),
        );
    }
    assert_eq!(
        local.take_movement_request(),
        Some(game_core::movement::MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(2),
        })
    );
}

#[test]
fn double_clicking_the_body_a_fleet_trades_at_opens_its_market() {
    let system = game_core::demo_galaxy().remove(0);
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
    let mut local = LocalView::new(system);
    local.camera.zoom = 4.0;
    local.focus();
    let (here, elsewhere) = (local.system.bodies[0].id, local.system.bodies[1].id);
    let point = local.camera.project(
        display_position(&local.system, &local.system.bodies[0]),
        rect,
    );
    let option = |body, selected| crate::session::TradeOption {
        ship: game_core::travel::ShipId(if selected { 1 } else { 2 }),
        fleet: "Fleet".into(),
        body,
        selected,
    };
    local.set_trade_options(vec![option(elsewhere, true)]);
    local.open_market_at(point, rect);
    assert_eq!(local.take_market_request(), None, "no fleet trades there");
    local.set_trade_options(vec![option(here, false), option(here, true)]);
    local.open_market_at(point, rect);
    assert_eq!(
        local.take_market_request(),
        Some(game_core::travel::ShipId(1)),
        "the selected fleet's market"
    );
}

#[test]
fn hidden_traffic_is_neither_drawn_nor_tappable() {
    let ctx = egui::Context::default();
    let system = game_core::demo_galaxy().remove(0);
    let host = crate::session::LocalSession::new(game_server::Session::default());
    let mut band = host.raider_bands()[0].clone();
    band.system = system.id;
    band.transit = None;
    band.position = game_core::LocalPosition {
        x_km: 60_000_000,
        y_km: 0,
        z_km: 0,
    };
    let mut local = LocalView::new(system);
    local.set_bands(vec![band]);
    let frame = |local: &mut LocalView| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0))),
                ..Default::default()
            },
            |ctx| {
                local.ui(ctx, &[], &[]);
            },
        );
    };
    frame(&mut local);
    assert!(!local.npc_hits.is_empty(), "the band is drawn and tappable");
    local.set_show_traffic(false);
    frame(&mut local);
    assert!(local.npc_hits.is_empty());
}
