use super::*;
use egui::Vec2;

#[test]
fn overlapping_targets_are_all_pickable_and_distance_is_view_independent() {
    let systems = fixtures::systems(Scene::Overlap);
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
    let mut camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        ..Camera::default()
    };
    assert_eq!(pick(&systems, &camera, rect, rect.center()).len(), 5);
    let distance = position(&systems[0]).distance(position(&systems[4]));
    assert_eq!(distance, 16.0);
    camera.yaw = 1.0;
    assert_ne!(
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
