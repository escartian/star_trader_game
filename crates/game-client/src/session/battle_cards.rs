//! Read-only parts of the battle window: side strength, turn order, the acting
//! ship's card, its weapons and the combat log. No commands are sent from here.
use crate::battle::scene::{HOSTILE, hull_color, hull_left, side_color};
use crate::map::art::{ArtKind, Artwork};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, Stroke, Ui, Vec2};
use game_core::combat::{
    CombatCatalog, Side,
    tactical::{Battle, Unit, morale},
};

const CARD: Color32 = Color32::from_rgb(14, 22, 26);

/// A labelled bar: `fraction` filled in `color`, with the value on the right.
pub fn meter(ui: &mut Ui, label: &str, fraction: f32, color: Color32, value: &str, width: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([54.0, 16.0], egui::Label::new(RichText::new(label).small()));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 8.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_black_alpha(160));
        let mut fill = rect;
        fill.set_width(rect.width() * fraction.clamp(0.0, 1.0));
        ui.painter().rect_filled(fill, 2.0, color);
        ui.label(RichText::new(value).small().color(color));
    });
}

/// One side's ships still fighting, their combined hull and their morale.
pub fn side_strength(ui: &mut Ui, battle: &Battle, side: Side, label: &str) {
    let ships: Vec<&Unit> = battle
        .units
        .iter()
        .filter(|unit| unit.side == side)
        .collect();
    let total: u32 = ships.iter().map(|unit| unit.profile.hull).sum();
    let left: u32 = ships
        .iter()
        .filter(|unit| unit.active())
        .map(|unit| unit.profile.hull - unit.damage)
        .sum();
    let fighting = ships.iter().filter(|unit| unit.active()).count();
    let fraction = left as f32 / total.max(1) as f32;
    ui.vertical(|ui| {
        ui.label(
            RichText::new(format!("{label} · {fighting} of {} fighting", ships.len()))
                .small()
                .color(side_color(side)),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(170.0, 6.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_black_alpha(160));
        let mut fill = rect;
        fill.set_width(rect.width() * fraction);
        ui.painter().rect_filled(fill, 2.0, side_color(side));
        let morale = morale::morale(battle, side);
        let (color, mood) = match morale {
            value if value < morale::BREAK_MORALE => (HOSTILE, "BROKEN"),
            value if value < 70 => (AMBER, "SHAKEN"),
            _ => (GREEN, "STEADY"),
        };
        ui.label(
            RichText::new(format!("Morale {morale} · {mood}"))
                .small()
                .color(color),
        );
    })
    .response
    .on_hover_text(format!(
        "{left} of {total} hull points left. Morale falls with damage, losses and \
         retreats and rises with kills; below {} a side breaks and the AI pulls its \
         ships out.",
        morale::BREAK_MORALE
    ));
}

/// Every ship in initiative order: portrait and hull (and the name, when there
/// is room); the acting ship is lit and kept in view.
pub fn initiative_bar(ui: &mut Ui, battle: &Battle, artwork: &Artwork) {
    let active = battle.active();
    // Past six ships, chips drop their names (shown on hover) so all fit.
    let compact = battle.units.len() > 6;
    egui::ScrollArea::horizontal()
        .id_salt("battle-initiative")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("TURN ORDER").small().color(Color32::GRAY));
                for &index in &battle.order {
                    let index = usize::from(index);
                    let acting = Some(index) == active;
                    let chip = chip(ui, &battle.units[index], acting, compact, artwork);
                    if acting {
                        chip.scroll_to_me(Some(egui::Align::Center));
                    }
                }
            });
        });
}

fn chip(
    ui: &mut Ui,
    unit: &Unit,
    acting: bool,
    compact: bool,
    artwork: &Artwork,
) -> egui::Response {
    let width = if compact { 52.0 } else { 148.0 };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
    let painter = ui.painter();
    let color = if unit.active() {
        side_color(unit.side)
    } else {
        Color32::DARK_GRAY
    };
    let fill = if acting {
        GREEN.gamma_multiply(0.14)
    } else {
        CARD
    };
    let stroke = if acting {
        Stroke::new(2.0_f32, GREEN)
    } else {
        Stroke::new(1.0_f32, color.gamma_multiply(0.45))
    };
    painter.rect(
        rect,
        CornerRadius::same(4),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    let portrait = if compact {
        rect.center_top() + Vec2::new(0.0, 15.0)
    } else {
        rect.left_center() + Vec2::new(19.0, 0.0)
    };
    if unit.active() {
        let size = if compact { 26.0 } else { 32.0 };
        artwork.paint_facing(
            ArtKind::from(unit.kind),
            painter,
            portrait,
            size,
            unit.side == Side::Hostile,
        );
    }
    let status = if !unit.alive() {
        Some("LOST")
    } else if unit.withdrawn {
        Some("LEFT")
    } else {
        None
    };
    let text = if compact {
        rect.shrink2(Vec2::new(5.0, 4.0))
    } else {
        Rect::from_min_max(
            rect.min + Vec2::new(38.0, 4.0),
            rect.max - Vec2::new(6.0, 4.0),
        )
    };
    if !compact {
        painter.text(
            text.left_top(),
            egui::Align2::LEFT_TOP,
            &unit.name,
            egui::FontId::proportional(13.0),
            color,
        );
    }
    match status {
        Some(status) => {
            painter.text(
                text.left_bottom(),
                egui::Align2::LEFT_BOTTOM,
                status,
                egui::FontId::proportional(11.0),
                Color32::GRAY,
            );
        }
        None => {
            let bar = Rect::from_min_size(
                text.left_bottom() - Vec2::new(0.0, 4.0),
                Vec2::new(text.width(), 4.0),
            );
            painter.rect_filled(bar, 1.0, Color32::from_black_alpha(170));
            let hull = hull_left(unit);
            let mut fill = bar;
            fill.set_width(bar.width() * hull);
            painter.rect_filled(fill, 1.0, hull_color(hull));
        }
    }
    response.on_hover_text(format!(
        "{} · {} · engine {} · hull {}/{}",
        unit.name,
        unit.kind.label(),
        unit.profile.engine,
        unit.profile.hull - unit.damage,
        unit.profile.hull
    ))
}

/// Portrait, condition and what the acting ship has left to do this turn.
pub fn ship_card(ui: &mut Ui, battle: &Battle, me: usize, artwork: &Artwork) {
    let unit = &battle.units[me];
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CYAN.gamma_multiply(0.4)))
        .corner_radius(4)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(58.0), Sense::hover());
                artwork.paint(ArtKind::from(unit.kind), ui.painter(), rect.center(), 58.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&unit.name)
                            .font(crate::theme::fonts::display(ui.ctx(), 14.0))
                            .color(CYAN),
                    );
                    ui.label(RichText::new(format!("{} · your turn", unit.kind.label())).small());
                    ui.horizontal(|ui| {
                        step(ui, "MOVE", battle.moves_left > 0);
                        step(ui, "FIRE", !battle.fired);
                    });
                });
            });
            let hull = hull_left(unit);
            let hull_text = format!("{}/{}", unit.profile.hull - unit.damage, unit.profile.hull);
            meter(ui, "Hull", hull, hull_color(hull), &hull_text, 120.0);
            let shields = unit.shields as f32 / unit.profile.shields.max(1) as f32;
            let shield_text = format!("{}/{}", unit.shields, unit.profile.shields);
            meter(ui, "Shields", shields, CYAN, &shield_text, 120.0);
            let points = unit.move_points();
            ui.horizontal(|ui| {
                icons::glyph(ui, Icon::Engine, AMBER, "Engine");
                ui.label(
                    RichText::new(if unit.jammed {
                        format!(
                            "{} of {points} moves left · ENGINES JAMMED",
                            battle.moves_left
                        )
                    } else {
                        format!("{} of {points} moves left", battle.moves_left)
                    })
                    .small(),
                )
                .on_hover_text("Each hex step costs one move; asteroids must be flown around.");
            });
        });
}

/// A pill showing whether an action is still available this turn.
fn step(ui: &mut Ui, label: &str, available: bool) {
    let color = if available { GREEN } else { Color32::DARK_GRAY };
    egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, color))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().color(color));
        });
}

/// Fitted weapons: range pips, and hit chance against the aimed or hovered
/// enemy (high ground included).
pub fn weapon_list(
    ui: &mut Ui,
    battle: &Battle,
    me: usize,
    target: Option<usize>,
    catalog: &CombatCatalog,
) {
    let distance = target.map(|index| battle.units[me].at.distance(battle.units[index].at));
    ui.horizontal(|ui| {
        ui.label(RichText::new("WEAPONS").small().color(Color32::GRAY));
        if let (Some(index), Some(distance)) = (target, distance) {
            ui.label(
                RichText::new(format!(
                    "vs {} · range {distance}",
                    battle.units[index].name
                ))
                .small()
                .color(HOSTILE),
            );
        }
    });
    for id in &battle.units[me].profile.loadout {
        let Some(weapon) = catalog.weapon(id) else {
            continue;
        };
        ui.horizontal(|ui| {
            icons::glyph(ui, Icon::Weapon, AMBER, &weapon.description);
            ui.label(RichText::new(&weapon.name).small());
            let effect = weapon.effect;
            if !effect.label().is_empty() {
                ui.label(
                    RichText::new(effect.label().to_uppercase())
                        .small()
                        .color(CYAN),
                )
                .on_hover_text(effect.summary());
            }
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| match (target, distance) {
                    (Some(index), Some(distance)) if distance <= weapon.range => {
                        let (from, to) = (battle.units[me].at, battle.units[index].at);
                        let fires = battle
                            .volley_from(me, from, to, catalog)
                            .iter()
                            .any(|ready| ready.id == weapon.id);
                        if fires {
                            let chance = battle.shot_chance(me, index, weapon);
                            ui.label(
                                RichText::new(format!("{chance}% ×{}", weapon.shots)).color(GREEN),
                            );
                        } else {
                            ui.label(RichText::new("blocked").small().color(Color32::GRAY))
                                .on_hover_text("An asteroid blocks the line of fire.");
                        }
                    }
                    (Some(_), Some(_)) => {
                        ui.label(RichText::new("out of range").small().color(Color32::GRAY));
                    }
                    _ => range_pips(ui, weapon.range),
                },
            );
        });
    }
}

/// One pip per step of range, up to six.
fn range_pips(ui: &mut Ui, range: u32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(6.0 * 8.0, 10.0), Sense::hover());
    for pip in 0..6_u32 {
        let centre = rect.left_center() + Vec2::new(4.0 + pip as f32 * 8.0, 0.0);
        let color = if pip < range {
            AMBER
        } else {
            Color32::from_gray(50)
        };
        ui.painter().circle_filled(centre, 2.5, color);
    }
    response.on_hover_text(format!("Range {range} hexes"));
}

/// The latest shots, newest last, tagged with their round.
pub fn shot_log(ui: &mut Ui, battle: &Battle, catalog: &CombatCatalog) {
    let shots: Vec<_> = battle
        .rounds
        .iter()
        .enumerate()
        .flat_map(|(round, shots)| shots.iter().map(move |shot| (round + 1, shot)))
        .collect();
    ui.label(RichText::new("COMBAT LOG").small().color(Color32::GRAY));
    if shots.is_empty() {
        ui.label(RichText::new("No shots fired yet.").small());
        return;
    }
    for (round, shot) in shots.iter().skip(shots.len().saturating_sub(5)) {
        let attacker = &battle.units[usize::from(shot.attacker)];
        let target = &battle.units[usize::from(shot.target)];
        let weapon = catalog
            .weapon(&shot.weapon)
            .map_or(shot.weapon.as_str(), |weapon| weapon.name.as_str());
        let (marker, result) = if !shot.hit {
            (Color32::GRAY, "missed".to_owned())
        } else if shot.destroyed {
            (AMBER, format!("{} hull · destroyed", shot.hull_damage))
        } else if shot.hull_damage > 0 {
            (
                AMBER,
                format!("{} shield, {} hull", shot.shield_damage, shot.hull_damage),
            )
        } else {
            (CYAN, format!("{} shield", shot.shield_damage))
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("R{round}"))
                    .small()
                    .color(Color32::GRAY),
            );
            ui.label(
                RichText::new(&attacker.name)
                    .small()
                    .color(side_color(attacker.side)),
            );
            ui.label(RichText::new(format!("{weapon} on")).small());
            ui.label(
                RichText::new(&target.name)
                    .small()
                    .color(side_color(target.side)),
            );
            ui.label(RichText::new(result).small().color(marker));
        });
    }
}
