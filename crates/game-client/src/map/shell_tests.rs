//! The phone-first shell: strip, action bar, sheet pages and full-screen panels.
use super::*;
use egui::Vec2;

const PHONE: Vec2 = Vec2::new(390.0, 844.0);
const DESKTOP: Vec2 = Vec2::new(1280.0, 800.0);

fn frame(
    ctx: &egui::Context,
    map: &mut Map,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
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

fn settle(ctx: &egui::Context, map: &mut Map, size: Vec2) -> Vec<(String, Pos2)> {
    let mut output = frame(ctx, map, size, Vec::new());
    for _ in 0..3 {
        output = frame(ctx, map, size, Vec::new());
    }
    texts(&output)
}

fn shows(ctx: &egui::Context, map: &mut Map, size: Vec2, label: &str) -> Option<Pos2> {
    settle(ctx, map, size)
        .into_iter()
        .find(|(text, _)| text == label)
        .map(|(_, at)| at)
}

fn tap(ctx: &egui::Context, map: &mut Map, size: Vec2, label: &str) {
    let point = shows(ctx, map, size, label).unwrap_or_else(|| {
        panic!("no {label:?} in {:?}", settle(ctx, map, size));
    });
    for pressed in [true, false] {
        frame(
            ctx,
            map,
            size,
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

fn multi_fleet() -> Map {
    let authority = game_server::Session::multi_fleet_scenario(1).unwrap();
    let systems = authority.systems().to_vec();
    Map::new(systems, authority)
}

#[test]
fn a_phone_shows_the_strip_and_action_bar_instead_of_floating_cards() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    let labels = settle(&ctx, &mut map, PHONE);
    for label in ["GALAXY", "Fleets", "ADVANCE", "Menu"] {
        assert!(labels.iter().any(|(text, _)| text == label), "{label}");
    }
    for old in ["Nav", "Trade", "Save", "OUTER GALAXY"] {
        assert!(
            !labels.iter().any(|(text, _)| text == old),
            "old card {old}"
        );
    }
    // The action bar sits at the bottom of the screen, the strip at the top.
    let advance = shows(&ctx, &mut map, PHONE, "ADVANCE").unwrap();
    assert!(advance.y > PHONE.y - 60.0);
    assert!(shows(&ctx, &mut map, PHONE, "GALAXY").unwrap().y < 40.0);
}

#[test]
fn tabs_switch_the_sheet_and_panels_open_full_screen_on_a_phone() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    assert!(shows(&ctx, &mut map, PHONE, "FLEETS").is_none());
    tap(&ctx, &mut map, PHONE, "Fleets");
    assert!(
        shows(&ctx, &mut map, PHONE, "FLEETS").is_some(),
        "fleet list"
    );
    tap(&ctx, &mut map, PHONE, "Menu");
    tap(&ctx, &mut map, PHONE, "Trade");
    // The market fills the screen, with Back instead of a close glyph.
    let back = shows(&ctx, &mut map, PHONE, "Back").expect("full-screen Back");
    assert!(back.x < PHONE.x / 2.0 && back.y < 60.0);
    tap(&ctx, &mut map, PHONE, "Back");
    assert!(shows(&ctx, &mut map, PHONE, "MARKET").is_none(), "closed");
}

#[test]
fn phone_market_shows_trade_actions_without_horizontal_clipping() {
    let ctx = egui::Context::default();
    let authority = game_server::Session::default();
    let mut map = Map::new(authority.systems().to_vec(), authority);
    map.session.open_market();
    let size = Vec2::new(320.0, 640.0);
    let mut output = frame(&ctx, &mut map, size, Vec::new());
    for _ in 0..3 {
        output = frame(&ctx, &mut map, size, Vec::new());
    }
    let labels = texts(&output);
    for label in [
        "Food",
        "Stock 100 · Held 0",
        "Buy 1",
        "Sell 1",
        "Max",
        "All",
    ] {
        assert!(
            labels.iter().any(|(text, _)| text == label),
            "missing {label}"
        );
    }
    for shape in &output.shapes {
        if let egui::epaint::Shape::Text(text) = &shape.shape {
            let bounds = Rect::from_min_size(text.pos, text.galley.size());
            if bounds.intersects(Rect::from_min_size(Pos2::ZERO, size)) {
                assert!(
                    bounds.max.x <= size.x,
                    "clipped text: {}",
                    text.galley.job.text
                );
            }
        }
    }
}

#[test]
fn the_alert_badge_brings_you_back_to_the_flagship_when_it_needs_orders() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    // The scenario leaves every fleet waiting; a plotted then cancelled course
    // leaves the flagship (Home Guard) without orders.
    let flagship = map.session.fleets()[0].clone();
    map.session.select_ship(flagship.ships[0]);
    map.session
        .select_destination(game_core::SystemId(1), Some(game_core::BodyId(1)));
    map.session
        .test_command(game_protocol::ClientCommand::CancelCourse {
            fleet: flagship.id,
            expected_revision: map.session.revision(),
        });
    // Only the flagship can need orders; the others stand by.
    let alerts = map.session.attention_fleets();
    assert_eq!(alerts, vec![flagship.id]);
    // Viewing another fleet, the badge brings the flagship back up.
    map.session.select_ship(map.session.fleets()[1].ships[0]);
    tap(&ctx, &mut map, PHONE, "! 1");
    assert!(flagship.ships.contains(&map.session.selected_ship().id));
    assert_eq!(map.sheet, crate::session::SheetTab::Fleets);
}

#[test]
fn the_sheet_docks_right_on_a_wide_window_and_at_the_bottom_on_a_phone() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    let wide = shows(
        &ctx,
        &mut map,
        DESKTOP,
        "Trader introduction / earnings goal",
    )
    .unwrap();
    assert!(wide.x > DESKTOP.x * 0.6, "docked right: {wide:?}");
    let ctx = egui::Context::default();
    let narrow = shows(&ctx, &mut map, PHONE, "Trader introduction / earnings goal").unwrap();
    assert!(narrow.y > PHONE.y * 0.5, "bottom sheet: {narrow:?}");
}
