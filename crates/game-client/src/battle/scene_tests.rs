use super::camera::{OrbitCamera, ScenePoint};
use super::scene::{Pick, SceneInput, pick};
use crate::map::art::Artwork;
use egui::{Pos2, Rect};
use game_core::combat::{CombatCatalog, engagement::CombatantSetup};
use game_core::combat::{
    Side,
    tactical::{Battle, Cell},
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
fn pointing_at_a_ship_or_a_move_dot_picks_it_and_other_layers_are_ignored() {
    let battle = battle();
    let camera = OrbitCamera::default();
    let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 560.0));
    let artwork = Artwork::preload(&egui::Context::default());
    let here = battle.units[0].cell;
    let step = Cell {
        x: here.x + 1,
        ..here
    };
    let above = Cell {
        z: here.z + 1,
        x: here.x + 1,
        ..here
    };
    let reachable = [step, above];
    let input = SceneInput {
        battle: &battle,
        reachable: &reachable,
        weapons: &CombatCatalog::default(),
        targets: &[],
        layer: here.z,
        artwork: &artwork,
    };
    let screen = |cell| camera.project(rect, ScenePoint::of_cell(cell)).unwrap().pos;
    assert_eq!(
        pick(rect, &camera, &input, Some(screen(here))),
        Some(Pick::Unit(0))
    );
    let raider = battle.units[1].cell;
    assert_eq!(
        pick(rect, &camera, &input, Some(screen(raider))),
        Some(Pick::Unit(1))
    );
    assert_eq!(
        pick(rect, &camera, &input, Some(screen(step))),
        Some(Pick::Cell(step))
    );
    // `above` is on another layer: not shown, so not clickable.
    assert_ne!(
        pick(rect, &camera, &input, Some(screen(above))),
        Some(Pick::Cell(above))
    );
    assert_eq!(
        pick(rect, &camera, &input, Some(Pos2::new(-5.0, -5.0))),
        None
    );
}
