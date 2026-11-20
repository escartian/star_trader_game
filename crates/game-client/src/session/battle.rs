//! The battle window: side strength, turn order, the 3D grid, the acting ship's
//! card and the player's actions. Every action is a `BattleAction` command; the
//! server plays raider turns and answers with the next player turn or the
//! finished battle. Read-only cards live in `battle_cards`.
use super::LocalSession;
use super::battle_cards::{initiative_bar, ship_card, shot_log, side_strength, weapon_list};
use crate::battle::{
    BattleView,
    scene::{HOSTILE, Pick, SceneInput},
};
use crate::map::art::Artwork;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::combat::{Side, tactical::Battle};
use game_core::navigation::{ActiveBattle, BattleAction};
use game_protocol::ClientCommand;

#[derive(Default)]
pub(super) struct BattlePanel {
    view: BattleView,
    /// The × (or Escape) was pressed: ask before surrendering.
    confirm_surrender: bool,
    hovered: Option<Pick>,
}

const SIDE_WIDTH: f32 = 290.0;
/// Space around the battle screen's content, in points.
const MARGIN: f32 = 14.0;
/// Height of everything but the grid: header, turn order, combat log, margins.
const CHROME_HEIGHT: f32 = 290.0;
/// Height of the surrender question's row, in points.
const PROMPT_HEIGHT: f32 = 44.0;

impl LocalSession {
    #[cfg(test)]
    pub fn surrender_prompt_open(&self) -> bool {
        self.battle.confirm_surrender
    }

    /// A battle is open; it covers the whole window until it ends.
    pub fn battle_open(&self) -> bool {
        self.state.navigation.battle.is_some()
    }

    /// Capture helper: centres on the acting ship and zooms in `factor` times.
    #[cfg(any(test, feature = "dev-tools"))]
    pub(super) fn battle_close_up(&mut self, factor: f32) {
        if let Some(open) = self.state.navigation.battle.clone() {
            self.battle.view.sync(&open.battle);
            self.battle.view.focus_active(&open.battle);
            self.battle.view.camera.zoom(factor);
        }
    }

    /// The battle fills the whole window while it is open; leaving it means
    /// fighting it out, withdrawing, auto-resolving or surrendering.
    pub fn show_battle(&mut self, ctx: &egui::Context) {
        let Some(open) = self.state.navigation.battle.clone() else {
            self.battle.confirm_surrender = false;
            return;
        };
        self.battle.view.sync(&open.battle);
        let artwork = Artwork::preload(ctx);
        let screen = ctx.screen_rect();
        // The surrender question takes a row of its own while it is open.
        let prompt = if self.battle.confirm_surrender {
            PROMPT_HEIGHT
        } else {
            0.0
        };
        let scene = Vec2::new(
            (screen.width() - SIDE_WIDTH - 3.0 * MARGIN).max(320.0),
            (screen.height() - CHROME_HEIGHT - prompt).max(240.0),
        );
        egui::Area::new(egui::Id::new("battle-screen"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.painter().extend(metal::plate(screen, 255));
                ui.set_min_size(screen.size());
                egui::Frame::NONE.inner_margin(MARGIN).show(ui, |ui| {
                    ui.set_width(screen.width() - 2.0 * MARGIN);
                    self.battle_header(ui, &open);
                    initiative_bar(ui, &open.battle, &artwork);
                    ui.add_space(6.0);
                    ui.horizontal_top(|ui| {
                        self.battle_scene(ui, &open.battle, scene, &artwork);
                        ui.add_space(MARGIN / 2.0);
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
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            // Escape opens the surrender question, or backs out of it.
            self.battle.confirm_surrender = !self.battle.confirm_surrender;
        }
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
                if icons::toggle(
                    ui,
                    Icon::Cancel,
                    "Surrender (Esc)",
                    self.battle.confirm_surrender,
                )
                .clicked()
                {
                    self.battle.confirm_surrender = !self.battle.confirm_surrender;
                }
            });
        });
        if self.battle.confirm_surrender {
            self.surrender_prompt(ui);
        }
        ui.separator();
    }

    /// A warning row under the header: the cost of surrendering and the choice.
    fn surrender_prompt(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(HOSTILE.gamma_multiply(0.12))
            .stroke(egui::Stroke::new(1.0_f32, HOSTILE.gamma_multiply(0.6)))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            "Surrender? The raiders seize all cargo in this fleet and let it go.",
                        )
                        .color(AMBER),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(
                                RichText::new("Keep fighting").color(GREEN),
                            ))
                            .clicked()
                        {
                            self.battle.confirm_surrender = false;
                        }
                        let surrender =
                            egui::Button::new(RichText::new("SURRENDER").color(Color32::BLACK))
                                .fill(HOSTILE);
                        if ui.add(surrender).clicked() {
                            self.battle.confirm_surrender = false;
                            self.battle_command(BattleAction::Surrender);
                        }
                    });
                });
            });
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
            let at_edge = battle.units[me].cell.x == battle.size.home_edge(Side::Player);
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

    /// A slider over the grid's layers (shown 1-based); the shown layer's move
    /// cells are clickable. Grids are as tall as they are wide, so a row of
    /// buttons would not fit the bigger battles.
    fn layer_picker(&mut self, ui: &mut egui::Ui) {
        let top = (self.battle.view.layers - 1).max(0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("LAYER").small().color(Color32::GRAY))
                .on_hover_text("Which height's move cells are shown (Q / E)");
            ui.spacing_mut().slider_width = SIDE_WIDTH - 120.0;
            ui.add(
                egui::Slider::new(&mut self.battle.view.layer, 0..=top)
                    .custom_formatter(|value, _| format!("{}", value as i64 + 1))
                    .custom_parser(|text| text.trim().parse::<f64>().ok().map(|value| value - 1.0)),
            );
            ui.label(RichText::new(format!("of {}", top + 1)).small());
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
            "Click a green dot to move, or a ringed raider to fire. Drag to orbit, right-drag or Shift+drag to pan, scroll to zoom, arrows to slide, F to centre."
        }
        (true, false) => "Moved. Fire at a ringed raider, or end the turn.",
        (false, true) => "Fired. You may still move, then end the turn.",
        (true, true) => "Moved and fired. End the turn.",
    }
}
