use super::camera::{BoardCamera, ScenePoint};
use super::scene::{Pick, SceneInput, pick};
use crate::map::art::Artwork;
use egui::{Pos2, Rect};
use game_core::combat::{CombatCatalog, engagement::CombatantSetup};
use game_core::combat::{
    Side,
    tactical::{Battle, Hex},
};
use game_core::fleet::{ShipCatalog, ShipKind};

fn battle() -> Battle {
    let setup = |kind, side| CombatantSetup {
        name: format!("{kind:?}"),
        side,
        ship: None,
        kind,
        profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
        hull_damage: 0,
    };
    let setups = [
        setup(ShipKind::Freighter, Side::Player),
        setup(ShipKind::Scout, Side::Hostile),
    ];
    Battle::new(&setups, &CombatCatalog::default(), 1).unwrap()
}

#[test]
fn pointing_at_a_ship_or_a_lit_hex_picks_it_and_unlit_hexes_are_ignored() {
    let battle = battle();
    let camera = BoardCamera::default();
    let artwork = Artwork::preload(&egui::Context::default());
    let here = battle.units[0].at;
    let step = Hex::new(here.q + 1, here.r);
    let unlit = Hex::new(here.q + 2, here.r - 1);
    let reachable = [step];
    let input = SceneInput {
        battle: &battle,
        reachable: &reachable,
        weapons: &CombatCatalog::default(),
        targets: &[],
        aimed: None,
        artwork: &artwork,
    };
    for rect in [
        Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 560.0)),
        Rect::from_min_size(Pos2::ZERO, egui::vec2(374.0, 420.0)),
    ] {
        let screen = |point| camera.project(rect, battle.board, point).unwrap().pos;
        assert_eq!(
            pick(rect, &camera, &input, Some(screen(ScenePoint::of(here)))),
            Some(Pick::Unit(0))
        );
        let raider = ScenePoint::of(battle.units[1].at);
        assert_eq!(
            pick(rect, &camera, &input, Some(screen(raider))),
            Some(Pick::Unit(1))
        );
        assert_eq!(
            pick(
                rect,
                &camera,
                &input,
                Some(screen(ScenePoint::of_hex(step)))
            ),
            Some(Pick::Hex(step))
        );
        assert_eq!(
            pick(
                rect,
                &camera,
                &input,
                Some(screen(ScenePoint::of_hex(unlit)))
            ),
            None,
            "only lit hexes take taps"
        );
        assert_eq!(
            pick(rect, &camera, &input, Some(Pos2::new(-5.0, -5.0))),
            None
        );
    }
}
