//! The tactical battle view: an orbit camera over the 3D grid, the shown layer
//! and pointer picking. View state only; battle rules and every mutation stay
//! in game-core and the server.
mod backdrop;
pub mod camera;
mod canvas;
pub mod effects;
pub mod model;
pub mod scene;
#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod view_tests;

use egui::{Key, PointerButton, Rect, Sense, Vec2};
use game_core::combat::tactical::Battle;
use scene::{Pick, SceneInput};

#[derive(Default)]
pub struct BattleView {
    pub camera: camera::OrbitCamera,
    /// Layer whose move cells are shown; follows the acting ship each new turn.
    pub layer: i8,
    /// Layers in the battle's grid.
    pub layers: i8,
    /// (round, turn position) the layer last followed.
    synced: Option<(u32, usize)>,
    effects: effects::Effects,
}

impl BattleView {
    /// At the start of each turn, show the acting ship's own layer.
    pub fn sync(&mut self, battle: &Battle) {
        self.layers = battle.size.height;
        let key = (battle.round, battle.turn);
        if self.synced != Some(key) {
            self.synced = Some(key);
            if let Some(me) = battle.active() {
                self.layer = battle.units[me].cell.z;
            }
        }
    }

    pub fn shift_layer(&mut self, by: i8) {
        self.layer = (self.layer + by).clamp(0, (self.layers - 1).max(0));
    }

    /// Paints the scene and handles the camera:
    /// - drag orbits; right-drag, middle-drag or Shift+drag pans;
    /// - scroll or pinch zooms; arrow keys slide the view, Page Up/Down raise
    ///   and lower it; F centres on the acting ship, R resets the view;
    /// - Q/E change the shown layer.
    ///
    /// Moves and shots since the last frame animate unless the UI has
    /// animation turned off. Returns the hovered pick and any clicked one.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        size: Vec2,
        input: &SceneInput,
    ) -> (Option<Pick>, Option<Pick>) {
        let grid = input.battle.size;
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let shift = ui.input(|i| i.modifiers.shift);
        let panning = response.dragged_by(PointerButton::Secondary)
            || response.dragged_by(PointerButton::Middle)
            || (response.dragged_by(PointerButton::Primary) && shift);
        if panning {
            self.camera.drag_pan(response.drag_delta(), rect, grid);
        } else if response.dragged_by(PointerButton::Primary) {
            self.camera.drag(response.drag_delta());
        }
        if response.hovered() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            self.camera.zoom(pinch * (scroll * 0.002).exp());
        }
        if !ui.ctx().wants_keyboard_input() {
            self.keys(ui, input);
        }
        let hovered = scene::pick(rect, &self.camera, input, response.hover_pos());
        let now = ui.input(|i| i.time);
        let animate = ui.ctx().style().animation_time > 0.0;
        self.effects.observe(input.battle, now, animate);
        if self.effects.busy(now) {
            ui.ctx().request_repaint();
        }
        let frame = scene::Frame {
            rect,
            camera: &self.camera,
            hovered,
            effects: &self.effects,
            now,
        };
        scene::paint(ui.painter(), &frame, input);
        // After the scene, so the buttons draw (and take clicks) on top of it.
        if self.view_controls(ui, rect, input) {
            ui.ctx().request_repaint();
        }
        let clicked = response
            .clicked()
            .then(|| scene::pick(rect, &self.camera, input, response.interact_pointer_pos()))
            .flatten();
        (hovered, clicked)
    }

    /// Keyboard camera and layer controls. Held arrow keys slide the view at
    /// [`SLIDE_CELLS_PER_SECOND`].
    fn keys(&mut self, ui: &egui::Ui, input: &SceneInput) {
        let grid = input.battle.size;
        let (dt, down, pressed) = ui.input(|i| {
            let down = |key| f32::from(u8::from(i.key_down(key)));
            (
                i.stable_dt.min(0.1),
                [
                    down(Key::ArrowUp) - down(Key::ArrowDown),
                    down(Key::ArrowRight) - down(Key::ArrowLeft),
                    down(Key::PageUp) - down(Key::PageDown),
                ],
                [Key::Q, Key::E, Key::F, Key::R].map(|key| i.key_pressed(key)),
            )
        });
        if down.iter().any(|value| *value != 0.0) {
            let step = SLIDE_CELLS_PER_SECOND * dt / self.camera.zoom.sqrt();
            self.camera
                .nudge(down[0] * step, down[1] * step, down[2] * step, grid);
            ui.ctx().request_repaint();
        }
        self.shift_layer(i8::from(pressed[1]) - i8::from(pressed[0]));
        if pressed[2] {
            self.focus_active(input.battle);
        }
        if pressed[3] {
            self.camera = camera::OrbitCamera::default();
        }
    }

    /// Centres the camera on the acting ship.
    pub fn focus_active(&mut self, battle: &Battle) {
        if let Some(me) = battle.active() {
            let at = camera::ScenePoint::of_cell(battle.units[me].cell);
            self.camera.look_at(at, battle.size);
        }
    }

    /// Focus and Reset buttons over the scene's top-right corner; true when one
    /// was used.
    fn view_controls(&mut self, ui: &mut egui::Ui, rect: Rect, input: &SceneInput) -> bool {
        use crate::theme::icons::{self, Icon};
        let corner = rect.right_top() + Vec2::new(-74.0, 8.0);
        let focus = ui.put(
            Rect::from_min_size(corner, Vec2::splat(30.0)),
            |ui: &mut egui::Ui| {
                icons::icon_button(ui, Icon::Focus, "Centre on the acting ship (F)")
            },
        );
        let reset = ui.put(
            Rect::from_min_size(corner + Vec2::new(36.0, 0.0), Vec2::splat(30.0)),
            |ui: &mut egui::Ui| icons::icon_button(ui, Icon::Reset, "Reset the view (R)"),
        );
        if focus.clicked() {
            self.focus_active(input.battle);
        }
        if reset.clicked() {
            self.camera = camera::OrbitCamera::default();
        }
        focus.clicked() || reset.clicked()
    }
}

/// Records whether an open battle owns the keyboard this frame, so map views
/// drawn underneath leave its keys (F, Escape and the rest) alone.
pub fn claim_keyboard(ctx: &egui::Context, open: bool) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("battle-keyboard"), open));
}

/// Whether an open battle owns the keyboard (see [`claim_keyboard`]).
pub fn keyboard_claimed(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp(egui::Id::new("battle-keyboard")))
        .unwrap_or(false)
}

/// How fast held arrow keys slide the view, at zoom 1.
const SLIDE_CELLS_PER_SECOND: f32 = 8.0;
