//! The battle window: side strength, turn order, the 3D grid, the acting ship's
//! card and the player's actions. Every action is a `BattleAction` command; the
//! server plays raider turns and answers with the next player turn or the
//! finished battle. Read-only cards live in `battle_cards`.
use super::LocalSession;
use super::battle_cards::{initiative_bar, ship_card, shot_log, side_strength, weapon_list};
use crate::battle::{
    BattleView,
    scene::{Pick, SceneInput},
};
use crate::map::art::Artwork;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::combat::{
    Side,
    tactical::{Battle, HEIGHT, home_edge},
};
use game_core::navigation::{ActiveBattle, BattleAction};
use game_protocol::ClientCommand;

#[derive(Default)]
pub(super) struct BattlePanel {
    view: BattleView,
    /// Encounter whose battle the player hid to look at the map.
    hidden: Option<u64>,
    hovered: Option<Pick>,
}

const SIDE_WIDTH: f32 = 290.0;

impl LocalSession {
    pub fn battle_open(&self) -> bool {
        self.state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| self.battle.hidden != Some(open.encounter))
    }

    /// Brings a hidden battle back.
    pub fn resume_battle(&mut self) {
        self.battle.hidden = None;
    }

    pub fn show_battle(&mut self, ctx: &egui::Context) {
        let Some(open) = self.state.navigation.battle.clone() else {
            return;
        };
        if self.battle.hidden == Some(open.encounter) {
            return;
        }
        self.battle.view.sync(&open.battle);
        let artwork = Artwork::preload(ctx);
        let screen = ctx.screen_rect().size();
        let width = (screen.x - 40.0).clamp(600.0, 1220.0);
        let scene = Vec2::new(
            width - SIDE_WIDTH - 36.0,
            (screen.y - 330.0).clamp(280.0, 640.0),
        );
        egui::Window::new("Battle")
            .title_bar(false)
            .frame(egui::Frame::NONE)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                metal::frame(ui, 255, |ui| {
                    ui.set_width(width);
                    self.battle_header(ui, &open);
                    initiative_bar(ui, &open.battle, &artwork);
                    ui.add_space(6.0);
                    ui.horizontal_top(|ui| {
                        self.battle_scene(ui, &open.battle, scene, &artwork);
                        ui.vertical(|ui| {
                            ui.set_width(SIDE_WIDTH);
                            self.battle_side(ui, &open.battle, &artwork);
                        });
                    });
                    ui.add_space(4.0);
                    shot_log(ui, &open.battle, &self.economy.rules.combat_catalog);
                });
            });
    }

    fn battle_header(&mut self, ui: &mut egui::Ui, open: &ActiveBattle) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("BATTLE · ROUND {}", open.battle.round))
                    .font(crate::theme::fonts::display(ui.ctx(), 16.0))
                    .color(GREEN),
            );
            if open.evasion_failed {
                ui.label(RichText::new("ESCAPE FAILED").small().color(AMBER));
            }
            ui.add_space(16.0);
            side_strength(ui, &open.battle, Side::Player, "YOUR FLEET");
            ui.add_space(8.0);
            side_strength(ui, &open.battle, Side::Hostile, "RAIDERS");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, Icon::Cancel, "Hide the battle to look at the map")
                    .clicked()
                {
                    self.battle.hidden = Some(open.encounter);
                }
            });
        });
        ui.separator();
    }

    fn battle_scene(&mut self, ui: &mut egui::Ui, battle: &Battle, size: Vec2, artwork: &Artwork) {
        let weapons = &self.economy.rules.combat_catalog;
        let me = battle.active();
        let reachable = battle.reachable();
        let targets = me
            .filter(|_| !battle.fired)
            .map(|me| battle.targets(me, weapons))
            .unwrap_or_default();
        let input = SceneInput {
            battle,
            weapons,
            reachable: &reachable,
            targets: &targets,
            layer: self.battle.view.layer,
            artwork,
        };
        let (hovered, clicked) = self.battle.view.show(ui, size, &input);
        self.battle.hovered = hovered;
        let action = match clicked {
            Some(Pick::Cell(cell)) => Some(BattleAction::Move(cell)),
            Some(Pick::Unit(target)) if targets.contains(&target) => Some(BattleAction::Fire {
                target: target as u8,
            }),
            _ => None,
        };
        if let Some(action) = action {
            self.battle_command(action);
        }
    }

    fn battle_side(&mut self, ui: &mut egui::Ui, battle: &Battle, artwork: &Artwork) {
        let Some(me) = battle.active() else {
            return;
        };
        ship_card(ui, battle, me, artwork);
        ui.add_space(6.0);
        let target = match self.battle.hovered {
            Some(Pick::Unit(index)) if battle.units[index].side == Side::Hostile => Some(index),
            _ => None,
        };
        weapon_list(ui, battle, me, target, &self.economy.rules.combat_catalog);
        ui.add_space(6.0);
        ui.label(RichText::new(turn_hint(battle)).small());
        ui.add_space(6.0);
        self.layer_picker(ui);
        ui.add_space(10.0);
        let mut action = None;
        let end_turn = ui
            .add_sized(
                [SIDE_WIDTH, 34.0],
                egui::Button::new(
                    RichText::new("END TURN")
                        .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                        .color(Color32::BLACK),
                )
                .fill(GREEN.gamma_multiply(0.85)),
            )
            .on_hover_text("Finish this ship's turn (Space)");
        let space =
            ui.input(|i| i.key_pressed(egui::Key::Space)) && !ui.ctx().wants_keyboard_input();
        if end_turn.clicked() || space {
            action = Some(BattleAction::EndTurn);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let at_edge = battle.units[me].cell.x == home_edge(Side::Player);
            let blocked =
                (!at_edge).then(|| "Move to your edge (the cyan column) to withdraw.".into());
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

    /// One button per layer; the shown layer's move cells are clickable.
    fn layer_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("LAYER").small().color(Color32::GRAY))
                .on_hover_text("Which height's move cells are shown (Q / E)");
            for layer in 0..HEIGHT {
                let selected = self.battle.view.layer == layer;
                let text = RichText::new((layer + 1).to_string()).color(if selected {
                    Color32::BLACK
                } else {
                    GREEN
                });
                let mut button = egui::Button::new(text).min_size(Vec2::new(30.0, 24.0));
                if selected {
                    button = button.fill(GREEN.gamma_multiply(0.85));
                }
                if ui.add(button).clicked() {
                    self.battle.view.layer = layer;
                }
            }
        });
    }

    fn battle_command(&mut self, action: BattleAction) {
        self.command(ClientCommand::BattleAction {
            action,
            expected_revision: self.state.revision,
        });
    }
}

fn turn_hint(battle: &Battle) -> &'static str {
    match (battle.moved, battle.fired) {
        (false, false) => {
            "Click a green dot to move, or a ringed raider to fire. Drag to orbit, scroll to zoom."
        }
        (true, false) => "Moved. Fire at a ringed raider, or end the turn.",
        (false, true) => "Fired. You may still move, then end the turn.",
        (true, true) => "Moved and fired. End the turn.",
    }
}
