//! The tactical battle view: a fixed-tilt camera over the hex board, the
//! aimed target and pointer picking. View state only; battle rules and every
//! mutation stay in game-core and the server.
mod backdrop;
pub mod camera;
mod canvas;
pub mod effects;
pub mod model;
mod particles;
pub mod scene;
#[cfg(test)]
mod scene_tests;
mod terrain;
#[cfg(test)]
mod view_tests;

use egui::{Key, Rect, Sense, Vec2};
use game_core::combat::tactical::Battle;
use scene::{Pick, SceneInput};

#[derive(Default)]
pub struct BattleView {
    pub camera: camera::BoardCamera,
    /// The enemy picked for the next volley; firing needs a second tap on
    /// Fire, so a stray tap never fires. Cleared each new turn and after firing.
    pub aimed: Option<usize>,
    /// (round, turn position, fired) the aim last belonged to.
    synced: Option<(u32, usize, bool)>,
    effects: effects::Effects,
}

impl BattleView {
    pub fn take_sounds(&mut self, now: f64) -> Vec<effects::SoundCue> {
        self.effects.take_sounds(now)
    }
    /// Drops the aim when a new turn starts or the volley has been fired.
    pub fn sync(&mut self, battle: &Battle) {
        let key = (battle.round, battle.turn, battle.fired);
        if self.synced != Some(key) {
            self.synced = Some(key);
            self.aimed = None;
        }
    }

    /// Paints the scene and handles the camera:
    /// - drag pans; scroll or pinch zooms (never smaller than fitting the board);
    /// - arrow keys slide the view; F centres on the acting ship, R resets it.
    ///
    /// Moves and shots since the last frame animate unless the UI has
    /// animation turned off. Returns the hovered pick and any clicked one.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        size: Vec2,
        input: &SceneInput,
    ) -> (Option<Pick>, Option<Pick>) {
        let board = input.battle.board;
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if response.dragged() {
            self.camera.drag_pan(response.drag_delta(), rect, board);
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
        // After the scene, so the buttons draw (and take clicks) on top of it,
        // in a child Ui so placing them leaves the parent layout's cursor alone.
        let mut overlay = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        if self.view_controls(&mut overlay, rect, input) {
            ui.ctx().request_repaint();
        }
        let clicked = response
            .clicked()
            .then(|| scene::pick(rect, &self.camera, input, response.interact_pointer_pos()))
            .flatten();
        (hovered, clicked)
    }

    /// Keyboard camera controls. Held arrow keys slide the view at
    /// [`SLIDE_HEXES_PER_SECOND`].
    fn keys(&mut self, ui: &egui::Ui, input: &SceneInput) {
        let board = input.battle.board;
        let (dt, down, pressed) = ui.input(|i| {
            let down = |key| f32::from(u8::from(i.key_down(key)));
            (
                i.stable_dt.min(0.1),
                [
                    down(Key::ArrowUp) - down(Key::ArrowDown),
                    down(Key::ArrowRight) - down(Key::ArrowLeft),
                ],
                [Key::F, Key::R].map(|key| i.key_pressed(key)),
            )
        });
        if down.iter().any(|value| *value != 0.0) {
            let step = SLIDE_HEXES_PER_SECOND * dt / self.camera.zoom.sqrt();
            self.camera.nudge(down[0] * step, down[1] * step, board);
            ui.ctx().request_repaint();
        }
        if pressed[0] {
            self.focus_active(input.battle);
        }
        if pressed[1] {
            self.camera = camera::BoardCamera::default();
        }
    }

    /// Centres the camera on the acting ship.
    pub fn focus_active(&mut self, battle: &Battle) {
        if let Some(me) = battle.active() {
            let at = camera::ScenePoint::of(battle.units[me].at);
            self.camera.look_at(at, battle.board);
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
            self.camera = camera::BoardCamera::default();
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
const SLIDE_HEXES_PER_SECOND: f32 = 6.0;
