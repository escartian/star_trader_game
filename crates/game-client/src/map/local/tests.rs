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
                assert!(!local.ui(ctx));
            },
        );
        assert!(
            !ctx.tessellate(output.shapes, output.pixels_per_point)
                .is_empty()
        );
    }
}
