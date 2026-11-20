//! Paints the battle volume and finds what the pointer is over. Ships are 3D
//! models (or sprites) standing on stalks over the floor grid, so their layer
//! reads at a glance; rings lie in each ship's own layer.
use super::camera::{OrbitCamera, Projector, ScenePoint};
use super::canvas::Canvas;
use super::effects::Effects;
use super::{backdrop, model};
use crate::map::art::{ArtKind, Artwork};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};
use game_core::combat::{
    CombatCatalog, Side,
    engagement::hit_chance,
    tactical::{Battle, Cell, Unit},
};

pub const HOSTILE: Color32 = Color32::from_rgb(255, 104, 84);

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Cell(Cell),
    Unit(usize),
}

pub struct SceneInput<'a> {
    pub battle: &'a Battle,
    pub weapons: &'a CombatCatalog,
    /// Cells the acting ship may still move to.
    pub reachable: &'a [Cell],
    /// Enemies the acting ship can fire on now.
    pub targets: &'a [usize],
    /// The layer whose move cells are shown and clickable.
    pub layer: i8,
    pub artwork: &'a Artwork,
}

pub fn side_color(side: Side) -> Color32 {
    match side {
        Side::Player => CYAN,
        Side::Hostile => HOSTILE,
    }
}

/// Billboard edge length in world units.
const SHIP_SIZE: f32 = 1.45;
/// Length of a 3D ship model in world units (one cell is 1.0).
const MODEL_SIZE: f32 = 1.6;
/// Radius of the ring around a ship, in world units.
const RING: f32 = 0.95;

/// What is under `pointer`: ships first (nearest wins), then move cells on the
/// shown layer.
pub fn pick(
    rect: Rect,
    camera: &OrbitCamera,
    input: &SceneInput,
    pointer: Option<Pos2>,
) -> Option<Pick> {
    let pointer = pointer.filter(|pointer| rect.contains(*pointer))?;
    let projector = camera.projector(rect, input.battle.size);
    let ship = input
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.active())
        .filter_map(|(index, unit)| {
            let at = projector.project(ScenePoint::of_cell(unit.cell))?;
            let radius = at.scale * SHIP_SIZE * 0.42;
            (at.pos.distance(pointer) <= radius).then_some((at.depth, index))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, index)) = ship {
        return Some(Pick::Unit(index));
    }
    input
        .reachable
        .iter()
        .filter(|cell| cell.z == input.layer)
        .filter_map(|cell| {
            let at = projector.project(ScenePoint::of_cell(*cell))?;
            let reach = (at.scale * 0.32).max(7.0);
            let distance = at.pos.distance(pointer);
            (distance <= reach).then_some((distance, *cell))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, cell)| Pick::Cell(cell))
}

/// Everything in the view, back to front, at UI time `now`.
pub struct Frame<'a> {
    pub rect: Rect,
    pub camera: &'a OrbitCamera,
    pub hovered: Option<Pick>,
    pub effects: &'a Effects,
    pub now: f64,
}

pub fn paint(painter: &egui::Painter, frame: &Frame, input: &SceneInput) {
    let painter = painter.with_clip_rect(frame.rect);
    let projector = frame.camera.projector(frame.rect, input.battle.size);
    let canvas = Canvas {
        painter: &painter,
        projector: &projector,
    };
    backdrop::sky(&painter, frame.rect, frame.camera.yaw);
    backdrop::floor(&canvas, input.battle.size);
    backdrop::layer(&canvas, input.battle.size, input.layer);
    let active = input.battle.active();
    if let (Some(me), Some(Pick::Cell(cell))) = (active, frame.hovered) {
        move_preview(&canvas, input, me, cell);
    }
    for cell in input.reachable.iter().filter(|cell| cell.z == input.layer) {
        if let Some(at) = projector.project(ScenePoint::of_cell(*cell)) {
            let lit = frame.hovered == Some(Pick::Cell(*cell));
            let radius = (at.scale * if lit { 0.13 } else { 0.065 }).max(2.0);
            painter.circle_filled(at.pos, radius * 2.2, GREEN.gamma_multiply(0.08));
            painter.circle_filled(
                at.pos,
                radius,
                GREEN.gamma_multiply(if lit { 1.0 } else { 0.6 }),
            );
        }
    }
    if let (Some(me), Some(Pick::Unit(target))) = (active, frame.hovered)
        && input.targets.contains(&target)
    {
        let from = ScenePoint::of_cell(input.battle.units[me].cell);
        let to = ScenePoint::of_cell(input.battle.units[target].cell);
        canvas.dashed(from, to, 0.25, Stroke::new(2.0_f32, HOSTILE));
    }
    ships(&canvas, frame, input, active);
    frame.effects.paint(&canvas, frame.now);
    if let Some(Pick::Unit(index)) = frame.hovered {
        hover_card(&painter, &projector, input, index);
    }
}

/// The path to a hovered move cell, and which enemies would be in reach from it.
fn move_preview(canvas: &Canvas, input: &SceneInput, me: usize, cell: Cell) {
    let from = ScenePoint::of_cell(input.battle.units[me].cell);
    let to = ScenePoint::of_cell(cell);
    canvas.line(from, to, Stroke::new(2.0_f32, GREEN.gamma_multiply(0.85)));
    canvas.dashed(
        to,
        to.on_floor(),
        0.2,
        Stroke::new(1.0_f32, GREEN.gamma_multiply(0.4)),
    );
    canvas.ring(to, RING, Stroke::new(1.5_f32, GREEN.gamma_multiply(0.7)));
    let reach = input.battle.max_range(me, input.weapons);
    for unit in &input.battle.units {
        if unit.side == Side::Hostile && unit.active() && cell.distance(unit.cell) <= reach {
            let enemy = ScenePoint::of_cell(unit.cell);
            canvas.dashed(
                to,
                enemy,
                0.3,
                Stroke::new(1.0_f32, HOSTILE.gamma_multiply(0.6)),
            );
        }
    }
}

/// Ships far to near: stalk and footprint, ring, model or sprite, status bars.
fn ships(canvas: &Canvas, frame: &Frame, input: &SceneInput, active: Option<usize>) {
    let mut ships: Vec<_> = input
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.active())
        .filter_map(|(index, unit)| {
            let point = frame.effects.position(index, unit.cell, frame.now);
            let at = canvas.projector.project(point)?;
            Some((index, unit, point, at))
        })
        .collect();
    ships.sort_by(|a, b| b.3.depth.total_cmp(&a.3.depth));
    for (index, unit, point, at) in ships {
        let color = side_color(unit.side);
        canvas.dashed(
            point,
            point.on_floor(),
            0.18,
            Stroke::new(1.0_f32, color.gamma_multiply(0.5)),
        );
        canvas.ring(
            point.on_floor(),
            0.25,
            Stroke::new(1.0_f32, color.gamma_multiply(0.5)),
        );
        let ring = if Some(index) == active {
            Some((GREEN, 2.5_f32))
        } else if input.targets.contains(&index) {
            let lit = frame.hovered == Some(Pick::Unit(index));
            Some((HOSTILE, if lit { 3.0 } else { 1.5 }))
        } else {
            None
        };
        if let Some((ring, width)) = ring {
            canvas.ring(point, RING, Stroke::new(width, ring));
        }
        let facing_left = unit.side == Side::Hostile;
        if let Some(model) = model::for_kind(unit.kind) {
            let side_heading = if facing_left {
                -glam::Vec2::X
            } else {
                glam::Vec2::X
            };
            let heading = frame
                .effects
                .heading(index, unit.cell, frame.now)
                .unwrap_or(side_heading);
            model.paint(canvas.painter, canvas.projector, point, MODEL_SIZE, heading);
        } else if !input.artwork.paint_facing(
            ArtKind::from(unit.kind),
            canvas.painter,
            at.pos,
            at.scale * SHIP_SIZE,
            facing_left,
        ) {
            canvas.painter.circle_filled(at.pos, at.scale * 0.3, color);
        }
        bars(canvas.painter, unit, at.pos, at.scale);
    }
}

fn bars(painter: &egui::Painter, unit: &Unit, centre: Pos2, scale: f32) {
    let width = (scale * 0.9).clamp(22.0, 70.0);
    let left = centre.x - width / 2.0;
    let top = centre.y - scale * 0.75;
    let bar = |row: f32, fraction: f32, color: Color32| {
        let back = Rect::from_min_size(Pos2::new(left, top + row), Vec2::new(width, 3.0));
        painter.rect_filled(back.expand(1.0), 1.0, Color32::from_black_alpha(190));
        let mut fill = back;
        fill.set_width(width * fraction.clamp(0.0, 1.0));
        painter.rect_filled(fill, 1.0, color);
    };
    let hull = hull_left(unit);
    bar(0.0, hull, hull_color(hull));
    if unit.profile.shields > 0 {
        bar(5.0, unit.shields as f32 / unit.profile.shields as f32, CYAN);
    }
}

/// Fraction of hull points left.
pub fn hull_left(unit: &Unit) -> f32 {
    (unit.profile.hull - unit.damage) as f32 / unit.profile.hull.max(1) as f32
}

pub fn hull_color(fraction: f32) -> Color32 {
    match fraction {
        f if f > 0.6 => GREEN,
        f if f > 0.3 => AMBER,
        _ => HOSTILE,
    }
}

/// Name, condition and (for an enemy) distance and hit chances, by the ship.
fn hover_card(painter: &egui::Painter, projector: &Projector, input: &SceneInput, index: usize) {
    let battle = input.battle;
    let unit = &battle.units[index];
    let Some(at) = projector.project(ScenePoint::of_cell(unit.cell)) else {
        return;
    };
    let mut lines = vec![
        (unit.name.clone(), side_color(unit.side)),
        (
            format!(
                "{} · hull {}/{} · shields {}/{}",
                unit.kind.label(),
                unit.profile.hull - unit.damage,
                unit.profile.hull,
                unit.shields,
                unit.profile.shields
            ),
            Color32::from_gray(200),
        ),
    ];
    if let Some(me) = battle
        .active()
        .filter(|me| battle.units[*me].side != unit.side)
    {
        let distance = battle.units[me].cell.distance(unit.cell);
        let chances: Vec<String> = battle.units[me]
            .profile
            .loadout
            .iter()
            .filter_map(|id| input.weapons.weapon(id))
            .filter(|weapon| weapon.range >= distance)
            .map(|weapon| format!("{}%", hit_chance(weapon, unit.profile.engine, distance)))
            .collect();
        let line = if chances.is_empty() {
            (format!("{distance} cells · out of range"), Color32::GRAY)
        } else {
            (
                format!("{distance} cells · hit {}", chances.join(" / ")),
                GREEN,
            )
        };
        lines.push(line);
    }
    let galleys: Vec<_> = lines
        .into_iter()
        .map(|(text, color)| painter.layout_no_wrap(text, FontId::proportional(13.0), color))
        .collect();
    let width = galleys
        .iter()
        .map(|galley| galley.size().x)
        .fold(0.0, f32::max)
        + 16.0;
    let height = galleys.iter().map(|galley| galley.size().y).sum::<f32>() + 10.0;
    let origin = at.pos + Vec2::new(at.scale * 0.7, -height / 2.0);
    let card = Rect::from_min_size(origin, Vec2::new(width, height));
    painter.rect_filled(card, 4.0, Color32::from_rgba_unmultiplied(8, 14, 18, 230));
    painter.rect_stroke(
        card,
        4.0,
        Stroke::new(1.0_f32, side_color(unit.side).gamma_multiply(0.6)),
        egui::StrokeKind::Inside,
    );
    let mut y = card.top() + 5.0;
    for galley in galleys {
        let height = galley.size().y;
        painter.galley(Pos2::new(card.left() + 8.0, y), galley, Color32::WHITE);
        y += height;
    }
}
