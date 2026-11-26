use super::*;
use egui::Vec2;

#[test]
fn overlapping_targets_are_all_pickable_and_distance_is_view_independent() {
    let systems = fixtures::systems(Scene::Overlap);
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
    let camera = Camera::default();
    assert_eq!(pick(&systems, &camera, rect, rect.center()).len(), 5);
    let distance = position(&systems[0]).distance(position(&systems[4]));
    assert_eq!(distance, 0.0);
    assert_eq!(
        camera.project(position(&systems[0]), rect),
        camera.project(position(&systems[4]), rect)
    );
    assert_eq!(
        position(&systems[0]).distance(position(&systems[4])),
        distance
    );
    assert!(pick(&systems, &camera, rect, Pos2::new(-100.0, -100.0)).is_empty());
}

#[test]
fn test_scenes_do_not_replace_session_selection() {
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.selected = Some(SystemId(2));
    map.change_scene(Scene::Dense);
    map.selected = Some(SystemId(1010));
    map.change_scene(Scene::Session);
    assert_eq!(map.selected, Some(SystemId(2)));
    assert_eq!(map.systems, game_core::demo_galaxy());
}

#[test]
fn returning_from_local_view_preserves_galaxy_context() {
    let ctx = egui::Context::default();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.selected = Some(SystemId(2));
    map.camera = Camera {
        yaw: 1.2,
        pitch: -0.7,
        zoom: 2.5,
        pan: Vec2::new(45.0, -30.0),
        focus: DVec3::new(8.0, 4.0, 12.0),
    };
    map.candidates = vec![SystemId(1), SystemId(2)];
    let expected_camera = Camera { ..map.camera };
    map.open_selected_system();
    assert!(map.local.is_some());
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| map.ui(ctx),
    );
    assert!(map.local.is_none());
    assert_eq!(map.selected, Some(SystemId(2)));
    assert_eq!(map.camera, expected_camera);
    assert_eq!(map.candidates, vec![SystemId(1), SystemId(2)]);
}

#[test]
fn escape_closes_an_open_window_before_leaving_the_system() {
    let ctx = egui::Context::default();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.selected = Some(SystemId(2));
    map.open_selected_system();
    map.session.open_market();
    let escape = || egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(escape(), |ctx| map.ui(ctx));
    assert!(
        map.local.is_some(),
        "the first Escape only closes the Market"
    );
    assert!(!map.session.close_popups());
    let _ = ctx.run(escape(), |ctx| map.ui(ctx));
    assert!(map.local.is_none());
}

#[test]
fn dragging_the_flat_galaxy_pans_without_rotating() {
    let ctx = egui::Context::default();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    let screen = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0)));
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: screen,
            ..Default::default()
        },
        |ctx| map.ui(ctx),
    );
    let start = map.last_map_rect.center();
    let initial_orientation = (map.camera.yaw, map.camera.pitch);
    for events in [
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        vec![egui::Event::PointerMoved(start + Vec2::new(55.0, 35.0))],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                events,
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        );
    }
    assert!(map.camera.pan.length() > 0.0);
    assert_eq!((map.camera.yaw, map.camera.pitch), initial_orientation);
}

#[test]
fn phone_and_desktop_layouts_tessellate_all_scenes() {
    let ctx = egui::Context::default();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    for size in [Vec2::new(320.0, 480.0), Vec2::new(1000.0, 720.0)] {
        for scene in [Scene::Session, Scene::Overlap, Scene::Dense] {
            map.change_scene(scene);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| map.ui(ctx),
            );
            assert!(
                !ctx.tessellate(output.shapes, output.pixels_per_point)
                    .is_empty()
            );
            map.open_selected_system();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| map.ui(ctx),
            );
            assert!(
                !ctx.tessellate(output.shapes, output.pixels_per_point)
                    .is_empty()
            );
            map.local = None;
        }
    }
}

#[test]
fn a_150_system_galaxy_tessellates_at_phone_size() {
    let authority = game_server::Session::with_galaxy(
        42,
        game_core::economy::EconomyRules::default(),
        game_core::generation::GalaxyConfig {
            systems: 150,
            radius_ly: 95,
            max_objects: 15,
        },
    )
    .unwrap();
    let mut map = Map::new(authority.systems().to_vec(), authority);
    assert!(map.camera.zoom < 0.3);
    let ctx = egui::Context::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(390.0, 844.0))),
            ..Default::default()
        },
        |ctx| map.ui(ctx),
    );
    assert!(
        !ctx.tessellate(output.shapes, output.pixels_per_point)
            .is_empty()
    );
}

#[test]
fn double_click_enters_only_an_unambiguous_system() {
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.select_targets(vec![SystemId(2)], false);
    assert_eq!(map.selected, Some(SystemId(2)));
    assert!(map.local.is_none());
    map.select_targets(vec![SystemId(1), SystemId(2)], true);
    assert!(map.local.is_none());
    assert_eq!(map.selected, Some(SystemId(2)));
    map.select_targets(vec![], true);
    assert!(map.local.is_none());
    map.select_targets(vec![SystemId(2)], true);
    assert!(map.local.is_some());
}

#[test]
fn map_fleet_markers_select_a_fleet_without_plotting_or_advancing_time() {
    use game_core::{
        fleet::{FleetId, ShipKind},
        travel::ShipId,
    };
    use game_protocol::ClientCommand;
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
    let mut map = Map::new(game_core::demo_galaxy(), authority);
    let ctx = egui::Context::default();
    let screen = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0)));
    for _ in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        );
    }
    let point = map
        .galaxy_fleet_hits
        .iter()
        .find(|hit| hit.fleet == FleetId(2))
        .expect("fleet must be pickable")
        .bounds
        .center();
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
            |ctx| map.ui(ctx),
        );
    }
    assert_eq!(map.session.selected_ship().id, ShipId(2));
    assert_eq!(map.session.run_number(), 0);
}

#[test]
fn leaving_and_entering_system_space_switch_views_only_after_a_navigation_run() {
    use game_core::movement::MoveTarget;
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.open_selected_system();
    map.session
        .capture_movement(MoveTarget::LeaveSystem(SystemId(1)))
        .unwrap();
    map.follow_navigation_transitions();
    assert!(map.local.is_none());
    map.session
        .capture_movement(MoveTarget::EnterSystem(SystemId(2)))
        .unwrap();
    map.follow_navigation_transitions();
    assert_eq!(map.local.as_ref().unwrap().system().id, SystemId(2));
}

#[test]
fn hidden_traffic_leaves_only_the_selected_npc_on_both_maps() {
    use crate::preferences::MapLayer;
    let ctx = egui::Context::default();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    let screen = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0)));
    let frame = |map: &mut Map| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        );
    };
    frame(&mut map);
    assert!(map.galaxy_npc_hits.len() > 1, "traders and bands drawn");
    let chosen = map.galaxy_npc_hits[0].quarry;
    map.session.toggle_map_layer(MapLayer::Traffic);
    frame(&mut map);
    assert!(map.galaxy_npc_hits.is_empty(), "all hidden");
    map.session.select_npc(chosen);
    frame(&mut map);
    let shown: Vec<_> = map.galaxy_npc_hits.iter().map(|hit| hit.quarry).collect();
    assert_eq!(shown, vec![chosen], "the selected one stays");

    // The system map follows the same option.
    map.session.toggle_map_layer(MapLayer::Traffic);
    map.selected = map.session.current_system();
    map.open_selected_system();
    frame(&mut map);
    let all = map.local.as_ref().unwrap().npc_count();
    assert!(all > 0, "traffic in the home system");
    map.session.toggle_map_layer(MapLayer::Traffic);
    frame(&mut map);
    assert!(map.local.as_ref().unwrap().npc_count() <= 1);
}
