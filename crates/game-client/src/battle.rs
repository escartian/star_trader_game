//! The tactical battle view: an orbit camera over the 3D grid, the shown layer
//! and pointer picking. View state only; battle rules and every mutation stay
//! in game-core and the server.
pub mod camera;
pub mod model;
pub mod scene;

use egui::{Sense, Vec2};
use game_core::combat::tactical::{Battle, HEIGHT};
use scene::{Pick, SceneInput};

#[derive(Default)]
pub struct BattleView {
    pub camera: camera::OrbitCamera,
    /// Layer whose move cells are shown; follows the acting ship each new turn.
    pub layer: i8,
    /// (round, turn position) the layer last followed.
    synced: Option<(u32, usize)>,
}

impl BattleView {
    /// At the start of each turn, show the acting ship's own layer.
    pub fn sync(&mut self, battle: &Battle) {
        let key = (battle.round, battle.turn);
        if self.synced != Some(key) {
            self.synced = Some(key);
            if let Some(me) = battle.active() {
                self.layer = battle.units[me].cell.z;
            }
        }
    }

    pub fn shift_layer(&mut self, by: i8) {
        self.layer = (self.layer + by).clamp(0, HEIGHT - 1);
    }

    /// Paints the scene; drag orbits, scroll or pinch zooms, Q/E change layer.
    /// Returns the hovered pick and, on a click, the clicked one.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        size: Vec2,
        input: &SceneInput,
    ) -> (Option<Pick>, Option<Pick>) {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if response.dragged() {
            self.camera.drag(response.drag_delta());
        }
        if response.hovered() {
            let (scroll, pinch, lower, higher) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.key_pressed(egui::Key::Q),
                    i.key_pressed(egui::Key::E),
                )
            });
            self.camera.zoom(pinch * (scroll * 0.002).exp());
            self.shift_layer(i8::from(higher) - i8::from(lower));
        }
        let hovered = scene::pick(rect, &self.camera, input, response.hover_pos());
        scene::paint(ui.painter(), rect, &self.camera, input, hovered);
        let clicked = response
            .clicked()
            .then(|| scene::pick(rect, &self.camera, input, response.interact_pointer_pos()))
            .flatten();
        (hovered, clicked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat::{CombatCatalog, Side, engagement::CombatantSetup};
    use game_core::fleet::{ShipCatalog, ShipKind};

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
        view.shift_layer(10);
        assert_eq!(view.layer, HEIGHT - 1);
        view.sync(&battle);
        assert_eq!(
            view.layer,
            HEIGHT - 1,
            "same turn: the player's choice stays"
        );
        let next = battle.order[(battle.turn + 1) % battle.order.len()];
        battle.units[usize::from(next)].cell.z = 0;
        battle.end_turn().unwrap();
        view.sync(&battle);
        assert_eq!(view.layer, 0, "a new turn shows the acting ship's layer");
        view.shift_layer(-3);
        assert_eq!(view.layer, 0);
    }
}
