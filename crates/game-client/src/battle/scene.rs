//! Paints the hex board and finds what the pointer is over. Ships are 3D
//! models (or sprites) floating at their band's height over their hex, with a
//! shadow and a stalk down to the board, so the band reads at a glance.
use super::camera::{BoardCamera, Projector, ScenePoint};
use super::canvas::Canvas;
use super::effects::Effects;
use super::{backdrop, model};
use crate::map::art::{ArtKind, Artwork};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};
use game_core::combat::{
    CombatCatalog, HullSize, Side,
    tactical::{Band, Battle, Hex, Position, Unit},
};
use game_core::fleet::ShipKind;

pub const HOSTILE: Color32 = Color32::from_rgb(255, 104, 84);

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Hex(Hex),
    Unit(usize),
}

pub struct SceneInput<'a> {
    pub battle: &'a Battle,
    pub weapons: &'a CombatCatalog,
    /// Hexes the acting ship may still move to on its own band.
    pub reachable: &'a [Hex],
    /// Enemies the acting ship can fire on now.
    pub targets: &'a [usize],
    /// The enemy picked for the next volley, waiting for Fire.
    pub aimed: Option<usize>,
    pub artwork: &'a Artwork,
}

impl SceneInput<'_> {
    /// Where the acting ship would stand on `hex`, keeping its band.
    fn on_my_band(&self, hex: Hex) -> Option<Position> {
        let me = self.battle.active()?;
        Some(Position::new(hex, self.battle.units[me].at.band))
    }
}

pub fn side_color(side: Side) -> Color32 {
    match side {
        Side::Player => CYAN,
        Side::Hostile => HOSTILE,
    }
}

/// Model lengths in world units; adjacent hex centres are 1.0 apart.
fn ship_length(size: HullSize, kind: ShipKind) -> f32 {
    match (size, kind) {
        (HullSize::Small, _) => 0.72,
        (HullSize::Medium, ShipKind::Freighter) => 1.22,
        (HullSize::Medium, _) => 1.05,
        (HullSize::Large, _) => 1.38,
    }
}

/// Transparent margins make the sprite's billboard larger than its model length.
const SPRITE_TO_MODEL_RATIO: f32 = 1.38;
/// Radius of the ring around a ship, in world units.
const RING: f32 = 0.46;

/// What is under `pointer`: ships first (nearest wins), then lit move hexes.
pub fn pick(
    rect: Rect,
    camera: &BoardCamera,
    input: &SceneInput,
    pointer: Option<Pos2>,
) -> Option<Pick> {
    let pointer = pointer.filter(|pointer| rect.contains(*pointer))?;
    let projector = camera.projector(rect, input.battle.board);
    let ship = input
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.active())
        .filter_map(|(index, unit)| {
            let at = projector.project(ScenePoint::of(unit.at))?;
            let radius = (at.scale * ship_length(unit.profile.size, unit.kind) * 0.42).max(14.0);
            (at.pos.distance(pointer) <= radius).then_some((at.depth, index))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, index)) = ship {
        return Some(Pick::Unit(index));
    }
    input
        .reachable
        .iter()
        .filter_map(|hex| {
            let at = projector.project(ScenePoint::of_hex(*hex))?;
            let reach = (at.scale * 0.5).max(10.0);
            let distance = at.pos.distance(pointer);
            (distance <= reach).then_some((distance, *hex))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, hex)| Pick::Hex(hex))
}

/// Everything in the view, back to front, at UI time `now`.
pub struct Frame<'a> {
    pub rect: Rect,
    pub camera: &'a BoardCamera,
    pub hovered: Option<Pick>,
    pub effects: &'a Effects,
    pub now: f64,
}

pub fn paint(painter: &egui::Painter, frame: &Frame, input: &SceneInput) {
    let painter = painter.with_clip_rect(frame.rect);
    let projector = frame.camera.projector(frame.rect, input.battle.board);
    let canvas = Canvas {
        painter: &painter,
        projector: &projector,
    };
    backdrop::sky(&painter, frame.rect, -frame.camera.pan.x * 6.0);
    backdrop::board(&canvas, input.battle.board);
    for hex in input.reachable {
        let lit = frame.hovered == Some(Pick::Hex(*hex));
        let fill = GREEN.gamma_multiply(if lit { 0.45 } else { 0.16 });
        let stroke = Stroke::new(1.5_f32, GREEN.gamma_multiply(if lit { 1.0 } else { 0.55 }));
        backdrop::hex(&canvas, *hex, 0.12, fill, stroke);
    }
    let active = input.battle.active();
    if let (Some(me), Some(Pick::Hex(hex))) = (active, frame.hovered) {
        move_preview(&canvas, input, me, hex);
    }
    // The aimed target, or a ringed one under the pointer: the line of fire.
    let sighted = input.aimed.or(match frame.hovered {
        Some(Pick::Unit(target)) => Some(target),
        _ => None,
    });
    if let (Some(me), Some(target)) = (active, sighted)
        && input.targets.contains(&target)
    {
        let from = ScenePoint::of(input.battle.units[me].at);
        let to = ScenePoint::of(input.battle.units[target].at);
        let width = if input.aimed == Some(target) {
            2.5_f32
        } else {
            1.5
        };
        canvas.dashed(from, to, 0.18, Stroke::new(width, HOSTILE));
    }
    ships(&canvas, frame, input, active);
    frame.effects.paint(&canvas, frame.now);
    if let Some(Pick::Unit(index)) = frame.hovered {
        hover_card(&painter, &projector, input, index);
    }
}

/// The path to a hovered move hex, and which enemies would be in reach from it.
fn move_preview(canvas: &Canvas, input: &SceneInput, me: usize, hex: Hex) {
    let Some(target) = input.on_my_band(hex) else {
        return;
    };
    let from = ScenePoint::of(input.battle.units[me].at);
    let to = ScenePoint::of(target);
    canvas.line(from, to, Stroke::new(2.0_f32, GREEN.gamma_multiply(0.85)));
    canvas.ring(to, RING, Stroke::new(1.5_f32, GREEN.gamma_multiply(0.7)));
    let reach = input.battle.max_range(me, input.weapons);
    for unit in &input.battle.units {
        if unit.side == Side::Hostile && unit.active() && target.range(unit.at) <= reach {
            let enemy = ScenePoint::of(unit.at);
            canvas.dashed(
                to,
                enemy,
                0.3,
                Stroke::new(1.0_f32, HOSTILE.gamma_multiply(0.6)),
            );
        }
    }
}

/// Ships far to near: shadow and stalk, ring, model or sprite, status bars.
fn ships(canvas: &Canvas, frame: &Frame, input: &SceneInput, active: Option<usize>) {
    let mut ships: Vec<_> = input
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.active())
        .filter_map(|(index, unit)| {
            let point = frame.effects.position(index, unit.at, frame.now);
            let at = canvas.projector.project(point)?;
            Some((index, unit, point, at))
        })
        .collect();
    ships.sort_by(|a, b| b.3.depth.total_cmp(&a.3.depth));
    for (index, unit, point, at) in ships {
        let color = side_color(unit.side);
        let length = ship_length(unit.profile.size, unit.kind);
        shadow(canvas, point.on_board(), length);
        canvas.dashed(
            point,
            point.on_board(),
            0.12,
            Stroke::new(1.0_f32, color.gamma_multiply(0.55)),
        );
        let ring = if Some(index) == active {
            Some((GREEN, 2.5_f32))
        } else if input.targets.contains(&index) {
            let lit = frame.hovered == Some(Pick::Unit(index)) || input.aimed == Some(index);
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
                .heading(index, unit.at, frame.now)
                .unwrap_or(side_heading);
            model.paint(canvas.painter, canvas.projector, point, length, heading);
        } else if !input.artwork.paint_facing(
            ArtKind::from(unit.kind),
            canvas.painter,
            at.pos,
            at.scale * length * SPRITE_TO_MODEL_RATIO,
            facing_left,
        ) {
            canvas.painter.circle_filled(at.pos, at.scale * 0.3, color);
        }
        bars(canvas.painter, unit, at.pos, at.scale);
    }
}

/// A soft dark disc on the board under a ship.
fn shadow(canvas: &Canvas, on_board: ScenePoint, ship_length: f32) {
    let points: Option<Vec<Pos2>> = (0..24)
        .map(|step| {
            let angle = step as f32 / 24.0 * std::f32::consts::TAU;
            let offset = glam::Vec3::new(angle.cos(), angle.sin(), 0.0) * ship_length * 0.29;
            canvas.at(ScenePoint(on_board.0 + offset))
        })
        .collect();
    if let Some(points) = points {
        canvas.painter.add(egui::Shape::convex_polygon(
            points,
            Color32::from_black_alpha(150),
            Stroke::NONE,
        ));
    }
}

/// A short band tag over the status bars: "HIGH" or "LOW" (Level is unmarked).
fn band_tag(band: Band) -> Option<&'static str> {
    match band {
        Band::High => Some("HIGH"),
        Band::Low => Some("LOW"),
        Band::Level => None,
    }
}

fn bars(painter: &egui::Painter, unit: &Unit, centre: Pos2, scale: f32) {
    let width = (scale * 0.8).clamp(22.0, 60.0);
    let left = centre.x - width / 2.0;
    let top = centre.y - (scale * 0.45).max(16.0);
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
    if let Some(tag) = band_tag(unit.at.band) {
        painter.text(
            Pos2::new(centre.x, top - 3.0),
            egui::Align2::CENTER_BOTTOM,
            tag,
            FontId::proportional(10.0),
            side_color(unit.side).gamma_multiply(0.9),
        );
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
    let Some(at) = projector.project(ScenePoint::of(unit.at)) else {
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
        (
            format!("{} band", unit.at.band.label()),
            Color32::from_gray(170),
        ),
    ];
    if let Some(me) = battle
        .active()
        .filter(|me| battle.units[*me].side != unit.side)
    {
        let distance = battle.units[me].at.range(unit.at);
        let chances: Vec<String> = battle.units[me]
            .profile
            .loadout
            .iter()
            .filter_map(|id| input.weapons.weapon(id))
            .filter(|weapon| weapon.range >= distance)
            .map(|weapon| format!("{}%", battle.shot_chance(me, index, weapon)))
            .collect();
        let line = if chances.is_empty() {
            (format!("range {distance} · out of reach"), Color32::GRAY)
        } else {
            (
                format!("range {distance} · hit {}", chances.join(" / ")),
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

#[cfg(test)]
mod scale_tests {
    use super::*;

    #[test]
    fn battle_hulls_read_from_scout_to_heavy_ship() {
        let scout = ship_length(HullSize::Small, ShipKind::Scout);
        let patrol = ship_length(HullSize::Medium, ShipKind::Patrol);
        let freighter = ship_length(HullSize::Medium, ShipKind::Freighter);
        let prospector = ship_length(HullSize::Large, ShipKind::Prospector);
        assert!(scout < patrol && patrol < freighter && freighter < prospector);
        assert!(freighter / scout > 1.5);
    }
}
