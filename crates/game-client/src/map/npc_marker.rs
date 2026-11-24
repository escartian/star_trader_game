//! The NPC marker both maps draw for traders, raider bands and police
//! patrols: the lead hull's model (or flat art) in a small dark disc ringed
//! friend-or-foe (`theme::iff`), with a pip in its faction's colour (traders:
//! grey if independent; police) and a ship count when a group has several.
//! Smaller than own fleet chips.
use super::art::{ArtKind, Artwork};
use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use game_core::fleet::ShipKind;

/// Marker radius in points.
pub(crate) const TRADER: f32 = 12.0;

/// Where the `index`th trader resting at a body goes, `clearance` points
/// from its centre: below-right, below, upper-left, left, then right (names
/// sit up-right; own fleet chips down-left), then on a wider ring.
pub(crate) fn berth_spot(anchor: Pos2, clearance: f32, index: usize) -> Pos2 {
    const ANGLES: [f32; 5] = [0.8, 1.6, -2.4, -3.1, 0.0];
    let ring = (index / ANGLES.len()) as f32;
    let distance = clearance + TRADER + 10.0 + ring * (TRADER * 2.0 + 4.0);
    anchor + Vec2::angled(ANGLES[index % ANGLES.len()]) * distance
}

/// The lead hull to show for a group: its heaviest ship.
pub(crate) fn lead(ships: &[ShipKind]) -> Option<ShipKind> {
    ships
        .iter()
        .copied()
        .find(|kind| *kind == ShipKind::Patrol)
        .or_else(|| ships.first().copied())
}

pub(crate) fn paint(
    painter: &Painter,
    art: &Artwork,
    at: Pos2,
    kind: ShipKind,
    ring: Color32,
    pip: Option<Color32>,
    count: usize,
) {
    painter.circle_filled(at, TRADER, Color32::from_black_alpha(190));
    painter.circle_stroke(at, TRADER, Stroke::new(1.2_f32, ring));
    if let Some(model) = crate::battle::model::for_kind(kind) {
        model.paint_marker(painter, at, TRADER * 1.8);
    } else if !art.paint(ArtKind::from(kind), painter, at, TRADER * 1.65) {
        painter.circle_filled(at, 3.0, ring);
    }
    if let Some(pip) = pip {
        let dot = at + Vec2::splat(TRADER * 0.7) * Vec2::new(1.0, -1.0);
        painter.circle_filled(dot, 3.5, pip);
        painter.circle_stroke(dot, 3.5, Stroke::new(1.0_f32, Color32::BLACK));
    }
    if count > 1 {
        let badge = at + Vec2::new(-TRADER * 0.75, -TRADER * 0.75);
        painter.circle_filled(badge, 6.5, ring);
        painter.text(
            badge,
            egui::Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::proportional(10.0),
            Color32::BLACK,
        );
    }
}
