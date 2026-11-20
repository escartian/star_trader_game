//! The battle window: turn order, the 3D grid, the acting ship's weapons and the
//! player's actions. Every action is a `BattleAction` command; the server plays
//! raider turns and answers with the next player turn or the finished battle.
use super::LocalSession;
use crate::battle::{
    BattleView,
    scene::{HOSTILE, Pick, SceneInput, side_color},
};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::combat::{
    CombatCatalog, Side,
    engagement::hit_chance,
    tactical::{Battle, HEIGHT, MAX_ROUNDS, home_edge, move_range},
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

const SIDE_WIDTH: f32 = 270.0;

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
        let screen = ctx.screen_rect().size();
        let width = (screen.x - 40.0).clamp(560.0, 1180.0);
        let scene = Vec2::new(
            width - SIDE_WIDTH - 36.0,
            (screen.y * 0.6).clamp(300.0, 600.0),
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
                    initiative_bar(ui, &open.battle);
                    ui.add_space(4.0);
                    ui.horizontal_top(|ui| {
                        self.battle_scene(ui, &open.battle, scene);
                        ui.vertical(|ui| {
                            ui.set_width(SIDE_WIDTH);
                            self.battle_side(ui, &open.battle);
                        });
                    });
                    shot_log(ui, &open.battle);
                });
            });
    }

    fn battle_header(&mut self, ui: &mut egui::Ui, open: &ActiveBattle) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "BATTLE · ROUND {} OF {MAX_ROUNDS}",
                    open.battle.round.min(MAX_ROUNDS)
                ))
                .font(crate::theme::fonts::display(ui.ctx(), 15.0))
                .color(GREEN),
            );
            if open.evasion_failed {
                ui.colored_label(AMBER, "Escape failed");
            }
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

    fn battle_scene(&mut self, ui: &mut egui::Ui, battle: &Battle, size: Vec2) {
        let weapons = &self.economy.rules.combat_catalog;
        let artwork = crate::map::art::Artwork::preload(ui.ctx());
        let me = battle.active();
        let reachable = battle.reachable();
        let targets = me
            .filter(|_| !battle.fired)
            .map(|me| battle.targets(me, weapons))
            .unwrap_or_default();
        let input = SceneInput {
            battle,
            reachable: &reachable,
            targets: &targets,
            layer: self.battle.view.layer,
            artwork: &artwork,
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

    fn battle_side(&mut self, ui: &mut egui::Ui, battle: &Battle) {
        let Some(me) = battle.active() else {
            return;
        };
        let unit = &battle.units[me];
        ui.label(
            RichText::new(&unit.name)
                .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                .color(CYAN),
        );
        ui.small(format!("{} · your turn", unit.kind.label()));
        ui.horizontal_wrapped(|ui| {
            let hull = format!("{}/{}", unit.profile.hull - unit.damage, unit.profile.hull);
            icons::stat(ui, Icon::Hull, GREEN, &hull, "Hull points left");
            let shields = format!("{}/{}", unit.shields, unit.profile.shields);
            icons::stat(ui, Icon::Shield, CYAN, &shields, "Shield points left");
            let reach = move_range(unit.profile.engine).to_string();
            icons::stat(
                ui,
                Icon::Engine,
                AMBER,
                &reach,
                "Cells this ship can move per turn",
            );
        });
        ui.add_space(6.0);
        self.weapon_list(ui, battle, me);
        ui.add_space(6.0);
        ui.label(turn_hint(battle));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(format!("Layer {} of {HEIGHT}", self.battle.view.layer + 1));
            if ui
                .add_enabled(self.battle.view.layer > 0, egui::Button::new("Lower"))
                .on_hover_text("Show move cells one layer down (Q)")
                .clicked()
            {
                self.battle.view.shift_layer(-1);
            }
            if ui
                .add_enabled(
                    self.battle.view.layer < HEIGHT - 1,
                    egui::Button::new("Higher"),
                )
                .on_hover_text("Show move cells one layer up (E)")
                .clicked()
            {
                self.battle.view.shift_layer(1);
            }
        });
        ui.add_space(8.0);
        let mut action = None;
        if icons::button(ui, Icon::Confirm, "End turn").clicked() {
            action = Some(BattleAction::EndTurn);
        }
        let at_edge = unit.cell.x == home_edge(Side::Player);
        let blocked = (!at_edge).then(|| "Move to your edge (left column) to withdraw.".into());
        if icons::labeled_action(ui, Icon::Back, "Withdraw", blocked).clicked() {
            action = Some(BattleAction::Withdraw);
        }
        if icons::button(ui, Icon::Execute, "Auto-resolve")
            .on_hover_text("Let the AI fight every remaining turn on both sides.")
            .clicked()
        {
            action = Some(BattleAction::AutoResolve);
        }
        if let Some(action) = action {
            self.battle_command(action);
        }
    }

    /// Fitted weapons with range, and hit chance against the hovered enemy.
    fn weapon_list(&self, ui: &mut egui::Ui, battle: &Battle, me: usize) {
        let catalog: &CombatCatalog = &self.economy.rules.combat_catalog;
        let target = match self.battle.hovered {
            Some(Pick::Unit(index)) if battle.units[index].side == Side::Hostile => Some(index),
            _ => None,
        };
        let distance = target.map(|index| battle.units[me].cell.distance(battle.units[index].cell));
        if let (Some(index), Some(distance)) = (target, distance) {
            ui.colored_label(
                HOSTILE,
                format!("{} · {distance} cells", battle.units[index].name),
            );
        }
        for id in &battle.units[me].profile.loadout {
            let Some(weapon) = catalog.weapon(id) else {
                continue;
            };
            ui.horizontal(|ui| {
                icons::glyph(ui, Icon::Weapon, AMBER, "Weapon");
                ui.label(&weapon.name);
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| match (target, distance) {
                        (Some(index), Some(distance)) if distance <= weapon.range => {
                            let engine = battle.units[index].profile.engine;
                            let chance = hit_chance(weapon, engine, distance);
                            ui.colored_label(GREEN, format!("{chance}% ×{}", weapon.shots));
                        }
                        (Some(_), Some(_)) => {
                            ui.colored_label(Color32::GRAY, "out of range");
                        }
                        _ => {
                            ui.small(format!("range {}", weapon.range));
                        }
                    },
                );
            });
        }
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

/// Every ship in initiative order; the acting one is outlined.
fn initiative_bar(ui: &mut egui::Ui, battle: &Battle) {
    let active = battle.active();
    ui.horizontal_wrapped(|ui| {
        ui.small("Turn order");
        for &index in &battle.order {
            let index = usize::from(index);
            let unit = &battle.units[index];
            let mut color = side_color(unit.side);
            let status = if !unit.alive() {
                color = Color32::DARK_GRAY;
                " · lost"
            } else if unit.withdrawn {
                color = Color32::GRAY;
                " · left"
            } else {
                ""
            };
            let stroke = if Some(index) == active {
                egui::Stroke::new(2.0_f32, GREEN)
            } else {
                egui::Stroke::new(1.0_f32, color.gamma_multiply(0.5))
            };
            egui::Frame::new()
                .stroke(stroke)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.colored_label(color, format!("{}{status}", unit.name));
                });
        }
    });
}

/// The latest shots, newest last.
fn shot_log(ui: &mut egui::Ui, battle: &Battle) {
    let shots: Vec<_> = battle.rounds.iter().flatten().collect();
    if shots.is_empty() {
        return;
    }
    ui.separator();
    for shot in shots.iter().skip(shots.len().saturating_sub(4)) {
        let attacker = &battle.units[usize::from(shot.attacker)];
        let target = &battle.units[usize::from(shot.target)];
        let result = if !shot.hit {
            "missed".to_owned()
        } else if shot.destroyed {
            format!("hit for {} hull, destroyed it", shot.hull_damage)
        } else {
            format!(
                "hit: {} shield, {} hull",
                shot.shield_damage, shot.hull_damage
            )
        };
        ui.horizontal(|ui| {
            ui.colored_label(side_color(attacker.side), &attacker.name);
            ui.label(format!("fired on {}: {result}", target.name));
        });
    }
}
