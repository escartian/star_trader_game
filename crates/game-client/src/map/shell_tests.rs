//! The phone-first shell: strip, action bar, sheet pages and full-screen panels.
use super::*;
use crate::session::SheetTab;
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
fn a_phone_shows_the_strip_with_its_actions_instead_of_floating_cards() {
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
    // No bottom bar: on a phone the actions take the strip's second row.
    let advance = shows(&ctx, &mut map, PHONE, "ADVANCE").unwrap();
    let title = shows(&ctx, &mut map, PHONE, "GALAXY").unwrap();
    assert!(title.y < 40.0 && advance.y > title.y && advance.y < 90.0);
    // A wide window keeps title and actions on one row.
    let advance = shows(&ctx, &mut map, DESKTOP, "ADVANCE").unwrap();
    assert!((advance.y - shows(&ctx, &mut map, DESKTOP, "GALAXY").unwrap().y).abs() < 8.0);
}

#[test]
fn the_card_closes_fully_and_reopens_on_a_tab() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    tap(&ctx, &mut map, PHONE, "Fleets");
    assert_eq!(map.sheet, SheetTab::Fleets);
    let with_card = map.card.map_area.height();
    tap(&ctx, &mut map, PHONE, "Close");
    assert_eq!(map.sheet, SheetTab::Selection);
    assert!(!map.card.open, "the whole card is gone");
    settle(&ctx, &mut map, PHONE);
    assert!(
        map.card.map_area.height() > with_card + 200.0,
        "the map takes its room"
    );
    assert!(shows(&ctx, &mut map, PHONE, "FLEETS").is_none());
    // A tab opens it again on that page.
    tap(&ctx, &mut map, PHONE, "Fleets");
    assert!(map.card.open && map.sheet == SheetTab::Fleets);
    tap(&ctx, &mut map, PHONE, "Close");
    tap(&ctx, &mut map, PHONE, "Menu");
    tap(&ctx, &mut map, PHONE, "Songbook · Music");
    assert!(map.songbook.0, "the in-game songbook opens");
    // On a phone it fills the screen; Back returns to the Menu page.
    tap(&ctx, &mut map, PHONE, "Back");
    assert!(!map.songbook.0);
    tap(&ctx, &mut map, PHONE, "Close");
    assert_eq!(map.sheet, SheetTab::Selection);
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

#[test]
fn the_search_button_opens_a_name_box_beside_the_map_buttons() {
    let ctx = egui::Context::default();
    let mut map = multi_fleet();
    assert!(shows(&ctx, &mut map, DESKTOP, "Star or planet name").is_none());
    super::search::toggle(&ctx);
    let at = shows(&ctx, &mut map, DESKTOP, "Star or planet name").expect("box open");
    assert!(at.y < 120.0, "at the top of the map: {at:?}");
    super::search::toggle(&ctx);
    assert!(shows(&ctx, &mut map, DESKTOP, "Star or planet name").is_none());
}
