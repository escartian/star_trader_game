use super::*;
use egui::Vec2;

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
    assert!(moon_point.distance(planet_point) > 35.0);
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
    // Body buttons use the same destination intent as canvas picks; neither moves authority.
    let output = ctx.run(
        egui::RawInput {
            screen_rect: screen,
            ..Default::default()
        },
        |ctx| {
            local.ui(ctx, &state.ships, &state.fleets);
        },
    );
    let point = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape
                && text.galley.job.text == "Select as destination"
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
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
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
    assert_eq!(
        local.take_movement_request(),
        Some(game_core::movement::MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1)
        })
    );
}
