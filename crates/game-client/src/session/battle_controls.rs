//! The acting ship's controls and board interaction. Tapping a lit hex moves
//! there (around asteroids); tapping a ringed enemy aims at it and **Fire**
//! confirms, so a stray tap never fires. Asteroids block the line of fire, so
//! only enemies in sight are ringed.
//! Keys: Enter fires at the aimed enemy, Space ends the turn.
use super::LocalSession;
use super::battle::SIDE_WIDTH;
use super::battle_cards::{ship_card, weapon_list};
use crate::battle::scene::{HOSTILE, Pick, SceneInput};
use crate::map::art::Artwork;
use crate::theme::GREEN;
use crate::theme::icons::{self, Icon};
use egui::{Color32, RichText, Vec2};
use game_core::combat::{
    Side,
    tactical::{Battle, COVER_PERCENT},
};
use game_core::navigation::BattleAction;

/// Height of the primary battle buttons: a comfortable touch target.
const BUTTON_HEIGHT: f32 = 44.0;

impl LocalSession {
    pub(super) fn battle_scene(
        &mut self,
        ui: &mut egui::Ui,
        battle: &Battle,
        size: Vec2,
        artwork: &Artwork,
    ) {
        let weapons = &self.economy.rules.combat_catalog;
        let me = battle.active();
        let reachable = battle.reachable();
        let targets = me
            .filter(|_| !battle.fired)
            .map(|me| battle.targets(me, weapons))
            .unwrap_or_default();
        let view = &mut self.battle.view;
        view.sync(battle);
        if view.aimed.is_some_and(|aimed| !targets.contains(&aimed)) {
            view.aimed = None;
        }
        let input = SceneInput {
            battle,
            weapons,
            reachable: &reachable,
            targets: &targets,
            aimed: view.aimed,
            artwork,
        };
        let (hovered, clicked) = view.show(ui, size, &input);
        self.battle.hovered = hovered;
        match clicked {
            Some(Pick::Hex(hex)) if me.is_some() => {
                self.battle_command(BattleAction::Move(hex));
            }
            Some(Pick::Unit(target)) if targets.contains(&target) => {
                self.battle.view.aimed = Some(target);
            }
            Some(_) => self.battle.view.aimed = None,
            None => {}
        }
    }

    /// The acting ship's card (wide layout only), its weapons against the aimed
    /// or hovered enemy, the band, Fire, End turn, Withdraw and Auto-resolve.
    pub(super) fn battle_controls(
        &mut self,
        ui: &mut egui::Ui,
        battle: &Battle,
        artwork: &Artwork,
        width: f32,
        narrow: bool,
    ) {
        let Some(me) = battle.active() else {
            return;
        };
        if !narrow {
            ship_card(ui, battle, me, artwork);
            ui.add_space(6.0);
        }
        let aimed = self.battle.view.aimed;
        let hovered_enemy = match self.battle.hovered {
            Some(Pick::Unit(index)) if battle.units[index].side == Side::Hostile => Some(index),
            _ => None,
        };
        let catalog = &self.economy.rules.combat_catalog;
        weapon_list(ui, battle, me, aimed.or(hovered_enemy), catalog);
        ui.add_space(6.0);
        ui.label(RichText::new(turn_hint(battle, aimed, narrow)).small());
        ui.add_space(6.0);
        cover_note(ui, battle, me);
        let mut action = None;
        ui.add_space(8.0);
        let keys = !ui.ctx().wants_keyboard_input();
        let (enter, space) = ui.input(|i| {
            (
                keys && i.key_pressed(egui::Key::Enter),
                keys && i.key_pressed(egui::Key::Space),
            )
        });
        let primary = if narrow {
            (width - 8.0) / 2.0
        } else {
            width.min(SIDE_WIDTH)
        };
        let mut row = |ui: &mut egui::Ui| {
            let fire_label = match aimed {
                Some(target) => format!("FIRE AT {}", battle.units[target].name.to_uppercase()),
                None => "FIRE".into(),
            };
            let fire = ui
                .add_enabled(
                    aimed.is_some(),
                    egui::Button::new(primary_text(ui, &fire_label))
                        .fill(HOSTILE)
                        .truncate()
                        .min_size(Vec2::new(primary, BUTTON_HEIGHT)),
                )
                .on_disabled_hover_text("Tap a ringed raider to aim first.")
                .on_hover_text("Fire every weapon that reaches the aimed raider (Enter)");
            if let Some(target) = aimed
                && (fire.clicked() || enter)
            {
                action = Some(BattleAction::Fire {
                    target: target as u8,
                });
            }
            let end_turn = ui
                .add(
                    egui::Button::new(primary_text(ui, "END TURN"))
                        .fill(GREEN.gamma_multiply(0.85))
                        .min_size(Vec2::new(primary, BUTTON_HEIGHT)),
                )
                .on_hover_text("Finish this ship's turn (Space)");
            if end_turn.clicked() || space {
                action = Some(BattleAction::EndTurn);
            }
        };
        if narrow {
            ui.horizontal(&mut row);
        } else {
            ui.vertical(&mut row);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let blocked = (!battle.can_withdraw())
                .then(|| "Move to your edge (the cyan column) to withdraw.".into());
            if icons::labeled_action(ui, Icon::Back, "Withdraw", blocked).clicked() {
                action = Some(BattleAction::Withdraw);
            }
            if icons::button(ui, Icon::Execute, "Auto-resolve")
                .on_hover_text("Let the AI fight every remaining turn on both sides.")
                .clicked()
            {
                action = Some(BattleAction::AutoResolve);
            }
        });
        if let Some(action) = action {
            self.battle_command(action);
        }
    }
}

/// Whether the acting ship stands in debris, and what that buys it.
fn cover_note(ui: &mut egui::Ui, battle: &Battle, me: usize) {
    if battle.cover(battle.units[me].at) {
        ui.label(
            RichText::new(format!("IN COVER · debris: −{COVER_PERCENT}% to be hit"))
                .small()
                .color(GREEN),
        )
        .on_hover_text("Ships in a debris field are harder to hit.");
    }
}

fn primary_text(ui: &egui::Ui, label: &str) -> RichText {
    RichText::new(label)
        .font(crate::theme::fonts::display(ui.ctx(), 14.0))
        .color(Color32::BLACK)
}

fn turn_hint(battle: &Battle, aimed: Option<usize>, narrow: bool) -> &'static str {
    if aimed.is_some() && !battle.fired {
        return "Aimed. Fire to shoot, or tap another raider.";
    }
    match (battle.moved(), battle.fired) {
        (false, false) if narrow => {
            "Tap a green hex to move, or a ringed raider to aim. Drag to pan, pinch to zoom."
        }
        (false, false) => {
            "Click a green hex to move, or a ringed raider to aim. Drag to pan, scroll to zoom, arrows to slide, F to centre."
        }
        (true, false) => "Moved. Keep moving, aim at a ringed raider, or end the turn.",
        (false, true) => "Fired. You may still move, then end the turn.",
        (true, true) => "Moved and fired. End the turn.",
    }
}
