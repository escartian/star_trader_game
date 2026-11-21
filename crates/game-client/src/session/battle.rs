//! The battle screen: side strength, turn order, the hex board, the acting
//! ship's controls and the combat log. Wide windows put the controls in a
//! column beside the board; phone-width ones stack them under it. Every action
//! is a `BattleAction` command (see `battle_controls`); the server plays raider
//! turns and answers with the next player turn or the finished battle.
//! Read-only cards live in `battle_cards`.
use super::LocalSession;
use super::battle_cards::{initiative_bar, shot_log, side_strength};
use crate::battle::{
    BattleView,
    scene::{HOSTILE, Pick},
};
use crate::map::art::Artwork;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::combat::Side;
use game_core::navigation::{ActiveBattle, BattleAction};
use game_protocol::ClientCommand;

#[derive(Default)]
pub(super) struct BattlePanel {
    pub(super) view: BattleView,
    /// The × (or Escape) was pressed: ask before surrendering.
    confirm_surrender: bool,
    pub(super) hovered: Option<Pick>,
}

pub(super) const SIDE_WIDTH: f32 = 290.0;
/// Space around the battle screen's content, in points.
const MARGIN: f32 = 14.0;
/// Height of everything but the board on wide windows: header, turn order,
/// combat log, margins.
const CHROME_HEIGHT: f32 = 290.0;
/// Share of a phone-width screen's height given to the board.
const NARROW_BOARD_SHARE: f32 = 0.5;
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
        let narrow = screen.width() < metal::FULL_SCREEN_BELOW;
        egui::Area::new(egui::Id::new("battle-screen"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.painter().extend(metal::plate(screen, 255));
                ui.set_min_size(screen.size());
                let margin = if narrow { MARGIN / 2.0 } else { MARGIN };
                egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                    let width = screen.width() - 2.0 * margin;
                    ui.set_width(width);
                    self.battle_header(ui, &open, narrow);
                    initiative_bar(ui, &open.battle, &artwork);
                    ui.add_space(6.0);
                    if narrow {
                        let board = Vec2::new(width, screen.height() * NARROW_BOARD_SHARE - prompt);
                        self.battle_scene(ui, &open.battle, board, &artwork);
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt("battle-controls")
                            .show(ui, |ui| {
                                self.battle_controls(ui, &open.battle, &artwork, width, true);
                            });
                        return;
                    }
                    let board = Vec2::new(
                        (width - SIDE_WIDTH - MARGIN).max(320.0),
                        (screen.height() - CHROME_HEIGHT - prompt).max(240.0),
                    );
                    ui.horizontal_top(|ui| {
                        self.battle_scene(ui, &open.battle, board, &artwork);
                        ui.add_space(MARGIN / 2.0);
                        ui.vertical(|ui| {
                            ui.set_width(SIDE_WIDTH);
                            self.battle_controls(ui, &open.battle, &artwork, SIDE_WIDTH, false);
                        });
                    });
                    ui.add_space(4.0);
                    shot_log(ui, &open.battle, &self.economy.rules.combat_catalog);
                });
            });
    }

    fn battle_header(&mut self, ui: &mut egui::Ui, open: &ActiveBattle, narrow: bool) {
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            // Escape opens the surrender question, or backs out of it.
            self.battle.confirm_surrender = !self.battle.confirm_surrender;
        }
        // An attack on a hailed trader names it, and giving up only breaks off.
        let trader = self.attacked_trader();
        let foe = trader
            .as_ref()
            .map_or_else(|| "RAIDERS".to_owned(), |name| name.to_uppercase());
        let give_up = if trader.is_some() {
            "Break off (Esc)"
        } else {
            "Surrender (Esc)"
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("BATTLE · ROUND {}", open.battle.round))
                    .font(crate::theme::fonts::display(ui.ctx(), 16.0))
                    .color(GREEN),
            );
            if open.evasion_failed {
                ui.label(RichText::new("ESCAPE FAILED").small().color(AMBER));
            }
            if !narrow {
                ui.add_space(16.0);
                side_strength(ui, &open.battle, Side::Player, "YOUR FLEET");
                ui.add_space(8.0);
                side_strength(ui, &open.battle, Side::Hostile, &foe);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::toggle(ui, Icon::Cancel, give_up, self.battle.confirm_surrender).clicked()
                {
                    self.battle.confirm_surrender = !self.battle.confirm_surrender;
                }
            });
        });
        if narrow {
            ui.horizontal(|ui| {
                side_strength(ui, &open.battle, Side::Player, "YOU");
                side_strength(ui, &open.battle, Side::Hostile, &foe);
            });
        }
        if self.battle.confirm_surrender {
            self.surrender_prompt(ui);
        }
        ui.separator();
    }

    /// A warning row under the header: the cost of surrendering and the choice.
    fn surrender_prompt(&mut self, ui: &mut egui::Ui) {
        let (question, act) = match self.attacked_trader() {
            Some(name) => (
                format!("Break off the attack? {name} gets away; you lose nothing more."),
                "BREAK OFF",
            ),
            None => (
                "Surrender? The raiders seize all cargo in this fleet and let it go.".to_owned(),
                "SURRENDER",
            ),
        };
        egui::Frame::new()
            .fill(HOSTILE.gamma_multiply(0.12))
            .stroke(egui::Stroke::new(1.0_f32, HOSTILE.gamma_multiply(0.6)))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(question).color(AMBER));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(
                                RichText::new("Keep fighting").color(GREEN),
                            ))
                            .clicked()
                        {
                            self.battle.confirm_surrender = false;
                        }
                        let surrender = egui::Button::new(RichText::new(act).color(Color32::BLACK))
                            .fill(HOSTILE);
                        if ui.add(surrender).clicked() {
                            self.battle.confirm_surrender = false;
                            self.battle_command(BattleAction::Surrender);
                        }
                    });
                });
            });
    }

    pub(super) fn battle_command(&mut self, action: BattleAction) {
        self.command(ClientCommand::BattleAction {
            action,
            expected_revision: self.state.revision,
        });
    }
}
