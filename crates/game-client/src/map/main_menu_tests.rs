use super::*;
use crate::audio::Track;

fn frame(map: &mut Map, ctx: &egui::Context, time: f64) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(1000.0, 720.0),
        )),
        time: Some(time),
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| map.ui(ctx));
}

#[test]
fn the_theme_plays_on_the_menu_and_continues_into_gameplay_by_default() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("slot-1.json");
    game_server::Session::default().save(&save).unwrap();
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.open_save_chooser(save.clone(), None);
    let ctx = egui::Context::default();
    frame(&mut map, &ctx, 0.0);
    assert_eq!(map.music.playing(), Some(Track::MainTheme));
    frame(&mut map, &ctx, 3.0);

    let mut menu = map.save_recovery.take().unwrap();
    assert!(map.carry_out(&ctx, &mut menu, Some(Choice::Load(save.clone()))));
    frame(&mut map, &ctx, 3.5);
    assert_eq!(
        map.music.playing(),
        Some(Track::MainTheme),
        "gameplay music stays enabled by default"
    );
    frame(&mut map, &ctx, 8.0);
    assert_eq!(map.music.playing(), Some(Track::MainTheme));

    // "Main menu" without saving reopens the menu on the same save.
    map.session.leave_to_main_menu(false);
    frame(&mut map, &ctx, 9.0);
    let menu = map.save_recovery.as_ref().expect("menu reopened");
    assert_eq!(menu.selected, save);
    assert_eq!(menu.page, MenuPage::Title);
}

#[test]
fn starting_over_needs_an_empty_slot_or_a_confirmed_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let empty = directory.path().join("slot-2.json");
    let mut map = Map::new(game_core::demo_galaxy(), game_server::Session::default());
    map.open_save_chooser(directory.path().join("slot-1.json"), None);
    let ctx = egui::Context::default();
    let mut menu = map.save_recovery.take().unwrap();
    assert!(map.carry_out(&ctx, &mut menu, Some(Choice::StartNew(empty.clone()))));
    assert_eq!(map.session.save_path(), empty);
    assert!(!empty.exists(), "nothing is written until the game saves");
}
