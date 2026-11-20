use super::{BattleView, camera, claim_keyboard, keyboard_claimed, scene::SceneInput};
use crate::map::art::Artwork;
use game_core::combat::tactical::Battle;
use game_core::combat::{CombatCatalog, Side, engagement::CombatantSetup};
use game_core::fleet::{ShipCatalog, ShipKind};
use glam::Vec3;

#[test]
fn the_shown_layer_follows_each_new_turn_and_stays_in_the_volume() {
    let setup = |side| CombatantSetup {
        name: format!("{side:?}"),
        side,
        ship: None,
        kind: ShipKind::Scout,
        profile: ShipCatalog::default()
            .spec(ShipKind::Scout)
            .unwrap()
            .combat
            .clone(),
        hull_damage: 0,
    };
    let mut battle = Battle::new(
        &[setup(Side::Player), setup(Side::Hostile)],
        &CombatCatalog::default(),
        2,
    )
    .unwrap();
    let mut view = BattleView::default();
    view.sync(&battle);
    let me = battle.active().unwrap();
    assert_eq!(view.layer, battle.units[me].cell.z);
    let top = battle.size.height - 1;
    view.shift_layer(30);
    assert_eq!(view.layer, top);
    view.sync(&battle);
    assert_eq!(view.layer, top, "same turn: the player's choice stays");
    let next = battle.order[(battle.turn + 1) % battle.order.len()];
    battle.units[usize::from(next)].cell.z = 0;
    battle.end_turn().unwrap();
    view.sync(&battle);
    assert_eq!(view.layer, 0, "a new turn shows the acting ship's layer");
    view.shift_layer(-3);
    assert_eq!(view.layer, 0);
}

fn duel() -> Battle {
    let setup = |kind, side| CombatantSetup {
        name: format!("{side:?}"),
        side,
        ship: None,
        kind,
        profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
        hull_damage: 0,
    };
    Battle::new(
        &[
            setup(ShipKind::Freighter, Side::Player),
            setup(ShipKind::Scout, Side::Hostile),
        ],
        &CombatCatalog::default(),
        2,
    )
    .unwrap()
}

/// One frame of the view in a full-window panel, fed `events`.
fn frame(ctx: &egui::Context, view: &mut BattleView, battle: &Battle, events: Vec<egui::Event>) {
    let weapons = CombatCatalog::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        let artwork = Artwork::preload(ctx);
        let scene = SceneInput {
            battle,
            weapons: &weapons,
            reachable: &[],
            targets: &[],
            layer: 0,
            artwork: &artwork,
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            view.show(ui, egui::vec2(700.0, 500.0), &scene);
        });
    });
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn keys_centre_on_the_acting_ship_and_reset_the_view() {
    let ctx = egui::Context::default();
    let battle = duel();
    let mut view = BattleView::default();
    view.sync(&battle);
    frame(&ctx, &mut view, &battle, vec![key(egui::Key::F)]);
    let me = battle.active().unwrap();
    let ship = camera::ScenePoint::of_cell(battle.units[me].cell);
    assert!((view.camera.focus(battle.size) - ship.0).length() < 1e-4);
    frame(&ctx, &mut view, &battle, vec![key(egui::Key::R)]);
    assert_eq!(view.camera, camera::OrbitCamera::default());
}

#[test]
fn right_dragging_pans_and_scrolling_zooms() {
    let ctx = egui::Context::default();
    let battle = duel();
    let mut view = BattleView::default();
    let start = egui::pos2(300.0, 300.0);
    let button = |pressed, pos| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &ctx,
        &mut view,
        &battle,
        vec![egui::Event::PointerMoved(start)],
    );
    frame(&ctx, &mut view, &battle, vec![button(true, start)]);
    for step in 1..=4 {
        let pos = start + egui::vec2(20.0 * step as f32, 0.0);
        frame(
            &ctx,
            &mut view,
            &battle,
            vec![egui::Event::PointerMoved(pos)],
        );
    }
    frame(
        &ctx,
        &mut view,
        &battle,
        vec![button(false, start + egui::vec2(80.0, 0.0))],
    );
    assert_ne!(view.camera.pan, Vec3::ZERO, "the view panned");
    assert_eq!(
        view.camera.yaw,
        camera::OrbitCamera::default().yaw,
        "no orbit"
    );
    let zoom = view.camera.zoom;
    frame(
        &ctx,
        &mut view,
        &battle,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 120.0),
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for _ in 0..30 {
        frame(&ctx, &mut view, &battle, Vec::new());
    }
    assert!(view.camera.zoom > zoom, "scrolling up zooms in");
}

#[test]
fn an_open_battle_claims_the_keyboard_from_the_map() {
    let ctx = egui::Context::default();
    assert!(!keyboard_claimed(&ctx));
    claim_keyboard(&ctx, true);
    assert!(keyboard_claimed(&ctx));
    claim_keyboard(&ctx, false);
    assert!(!keyboard_claimed(&ctx));
}
