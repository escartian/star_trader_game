use super::*;
use egui::{Event, PointerButton, Pos2, RawInput, Rect, Vec2, epaint::Shape};

fn frame(ctx: &egui::Context, session: &mut LocalSession, events: Vec<Event>) -> egui::FullOutput {
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0))),
            events,
            ..Default::default()
        },
        |ctx| session.show_navigation(ctx, &game_core::demo_galaxy()),
    )
}

#[test]
fn preview_does_not_move_and_confirm_button_resolves_the_authoritative_action() {
    let ctx = egui::Context::default();
    let mut session = LocalSession::new(game_server::Session::default());
    session.request_movement(MoveTarget::Body {
        system: SystemId(1),
        body: game_core::BodyId(1),
    });
    frame(&ctx, &mut session, vec![]);
    let output = frame(&ctx, &mut session, vec![]);
    assert_eq!(session.state.tick, 0);
    let confirm = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let Shape::Text(text) = &shape.shape
                && text.galley.job.text == "Plot course"
            {
                Some(text.pos + text.galley.size() / 2.0)
            } else {
                None
            }
        })
        .expect("movement preview must expose a confirm action");
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut session,
            vec![
                Event::PointerMoved(confirm),
                Event::PointerButton {
                    pos: confirm,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.selected_ship().fuel, 40);
    session.request_execute_run();
    assert_eq!(session.state.tick, 4);
    assert_eq!(session.selected_ship().fuel, 39);
    assert_eq!(
        session.selected_ship().location,
        Location::Orbiting {
            system: SystemId(1),
            body: game_core::BodyId(1)
        }
    );
    assert!(session.navigation.target.is_none());
    let moved = session.state.clone();
    frame(&ctx, &mut session, vec![]);
    assert_eq!(session.state, moved);
}

#[test]
fn impossible_preview_has_no_confirm_button_and_preserves_the_world() {
    let ctx = egui::Context::default();
    let mut session = LocalSession::new(game_server::Session::default());
    session.request_movement(MoveTarget::EnterSystem(SystemId(2)));
    let initial = session.state.clone();
    frame(&ctx, &mut session, vec![]);
    let output = frame(&ctx, &mut session, vec![]);
    assert!(!output.shapes.iter().any(
        |shape| matches!(&shape.shape, Shape::Text(text) if text.galley.job.text == "Plot course")
    ));
    assert_eq!(session.state, initial);
}
