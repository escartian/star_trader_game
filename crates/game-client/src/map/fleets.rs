//! Fleet markers, shared by painting and picking. Fleets at the same place merge
//! into one compact chip (with a count when there are several), drawn beside
//! the body or system they rest at, or right on their position in flight. No
//! tether lines and no status badges: the selection gets a ring and its name, a
//! fleet with an unresolved contact an amber ring. Tapping a chip selects the
//! next fleet in it, cycling through the stack.
use super::art::{ArtKind, Artwork};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use game_core::{
    fleet::{Fleet, FleetId},
    travel::{Ship, ShipId},
};

/// Chip radius in points.
const CHIP: f32 = 15.0;
/// Touch-friendly hit size around a chip, in points.
const HIT: f32 = 40.0;
/// Anchors closer than this (points) count as the same place.
const SAME_PLACE: f32 = 4.0;

#[derive(Clone, Debug)]
pub(super) struct FleetHit {
    /// The fleet a tap selects (the next one in the stack).
    pub fleet: FleetId,
    pub ship: ShipId,
    pub bounds: Rect,
}

/// How a fleet is resting, for presentation only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Berth {
    /// Flying, waiting at a waypoint, or in deep space: drawn on its position.
    Free,
    /// Resting at a body or system: drawn beside it, `clearance` points from
    /// the anchor so the chip does not cover it.
    AtBody { clearance: f32, landed: bool },
}

impl Berth {
    pub fn new(location: &game_core::travel::Location, clearance: f32) -> Self {
        use game_core::travel::Location;
        match location {
            Location::Docked(_) | Location::Orbiting { .. } => Self::AtBody {
                clearance,
                landed: false,
            },
            Location::Landed { .. } => Self::AtBody {
                clearance,
                landed: true,
            },
            _ => Self::Free,
        }
    }
}

/// One fleet to draw.
pub(super) struct StackEntry<'a> {
    pub fleet: &'a Fleet,
    pub ship: &'a Ship,
    /// Screen point of the fleet's position or resting place.
    pub anchor: Pos2,
    pub berth: Berth,
    /// Paused by an unresolved contact.
    pub contact: bool,
    /// Screen point it is heading for, when under way.
    pub heading: Option<Pos2>,
    /// Radius in points of the orbit ring drawn when this fleet is selected
    /// and in orbit (0 for none).
    pub ring: f32,
}

/// Paints every fleet, merged into stacks, and returns their tap targets.
/// Resting chips steer clear of `reserved` areas (labels) and of each other.
pub(super) fn paint_stacks(
    painter: &Painter,
    art: &Artwork,
    viewport: Rect,
    entries: &[StackEntry],
    selected: Option<FleetId>,
    reserved: &[Rect],
) -> Vec<FleetHit> {
    let mut hits = Vec::new();
    let mut placed: Vec<Rect> = reserved.to_vec();
    let mut stacks = group(entries);
    // Fleets in flight sit on their exact spot; resting stacks then go around them.
    stacks.sort_by_key(|stack| matches!(stack[0].berth, Berth::AtBody { .. }));
    for stack in stacks {
        let lead = stack
            .iter()
            .copied()
            .find(|entry| Some(entry.fleet.id) == selected)
            .unwrap_or(stack[0]);
        let chosen = stack.iter().any(|entry| Some(entry.fleet.id) == selected);
        let centre = chip_centre(lead, &placed);
        let bounds = Rect::from_center_size(centre, Vec2::splat(HIT));
        if !viewport.intersects(bounds) {
            continue;
        }
        placed.push(Rect::from_center_size(
            centre,
            Vec2::splat(CHIP * 2.0 + 4.0),
        ));
        if chosen && lead.ring > 0.0 {
            orbit_ring(painter, lead.anchor, lead.ring, crate::theme::CYAN);
        }
        if let Berth::AtBody { landed: true, .. } = lead.berth {
            painter.circle_filled(lead.anchor, 3.5, crate::theme::AMBER);
        }
        let colour = if stack.iter().any(|entry| entry.contact) {
            crate::theme::AMBER
        } else if chosen {
            crate::theme::GREEN
        } else {
            Color32::from_rgb(150, 185, 205)
        };
        if let Some(heading) = lead.heading.filter(|_| lead.berth == Berth::Free) {
            let direction = (heading - centre).normalized();
            if direction.is_finite() {
                painter.line_segment(
                    [
                        centre + direction * CHIP,
                        centre + direction * (CHIP + 10.0),
                    ],
                    Stroke::new(2.0_f32, colour.gamma_multiply(0.8)),
                );
            }
        }
        painter.circle_filled(
            centre,
            CHIP,
            Color32::from_rgba_unmultiplied(6, 21, 34, 230),
        );
        painter.circle_stroke(
            centre,
            CHIP,
            Stroke::new(if chosen { 2.0_f32 } else { 1.0_f32 }, colour),
        );
        art.paint(ArtKind::from(lead.ship.kind), painter, centre, CHIP * 1.7);
        if stack.len() > 1 {
            let badge = centre + Vec2::new(CHIP * 0.75, -CHIP * 0.75);
            painter.circle_filled(badge, 7.5, crate::theme::GREEN.gamma_multiply(0.85));
            painter.text(
                badge,
                Align2::CENTER_CENTER,
                stack.len().to_string(),
                FontId::proportional(11.0),
                Color32::BLACK,
            );
        }
        if chosen {
            painter.text(
                centre + Vec2::new(0.0, CHIP + 3.0),
                Align2::CENTER_TOP,
                &lead.fleet.name,
                FontId::proportional(11.0),
                colour,
            );
        }
        let next = next_in_stack(&stack, selected);
        hits.push(FleetHit {
            fleet: next.fleet.id,
            ship: next.ship.id,
            bounds,
        });
    }
    hits
}

/// Fleets at the same place (same kind of berth, anchors within [`SAME_PLACE`]),
/// each stack ordered by fleet ID.
fn group<'a, 'b>(entries: &'b [StackEntry<'a>]) -> Vec<Vec<&'b StackEntry<'a>>> {
    let mut stacks: Vec<Vec<&StackEntry>> = Vec::new();
    for entry in entries {
        let resting = matches!(entry.berth, Berth::AtBody { .. });
        match stacks.iter_mut().find(|stack| {
            let first = stack[0];
            matches!(first.berth, Berth::AtBody { .. }) == resting
                && first.anchor.distance(entry.anchor) <= SAME_PLACE
        }) {
            Some(stack) => stack.push(entry),
            None => stacks.push(vec![entry]),
        }
    }
    for stack in &mut stacks {
        stack.sort_by_key(|entry| entry.fleet.id.0);
    }
    stacks
}

/// Where a stack's chip goes: on the fleet in flight, or down-left of the body
/// it rests at (labels go up and right), turning around the body if that spot
/// is taken.
fn chip_centre(lead: &StackEntry, placed: &[Rect]) -> Pos2 {
    let Berth::AtBody { clearance, .. } = lead.berth else {
        return lead.anchor;
    };
    let distance = clearance + CHIP + 2.0;
    let mut first = None;
    for step in 0..8 {
        let angle = 3.0 * std::f32::consts::FRAC_PI_4 + step as f32 * std::f32::consts::FRAC_PI_4;
        let centre = lead.anchor + Vec2::angled(angle) * distance;
        let spot = Rect::from_center_size(centre, Vec2::splat(CHIP * 2.0));
        if !placed.iter().any(|other| other.intersects(spot)) {
            return centre;
        }
        first.get_or_insert(centre);
    }
    first.unwrap_or(lead.anchor)
}

/// The fleet a tap on this stack selects: the one after the current selection,
/// or the first.
fn next_in_stack<'a, 'b>(
    stack: &[&'b StackEntry<'a>],
    selected: Option<FleetId>,
) -> &'b StackEntry<'a> {
    match stack
        .iter()
        .position(|entry| Some(entry.fleet.id) == selected)
    {
        Some(index) => stack[(index + 1) % stack.len()],
        None => stack[0],
    }
}

/// Dashed ellipse around the orbited body; flattened to suggest the orbital plane.
fn orbit_ring(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let segments = 28;
    for step in (0..segments).step_by(2) {
        let point = |i: i32| {
            let angle = i as f32 / segments as f32 * std::f32::consts::TAU;
            center + Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.45)
        };
        painter.line_segment(
            [point(step), point(step + 1)],
            Stroke::new(1.2_f32, color.gamma_multiply(0.6)),
        );
    }
}

pub(super) fn pick(hits: &[FleetHit], pointer: Pos2) -> Option<&FleetHit> {
    hits.iter().find(|hit| hit.bounds.contains(pointer))
}

#[cfg(test)]
mod tests;
