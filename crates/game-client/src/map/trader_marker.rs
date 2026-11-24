//! The NPC trader marker both maps draw: its hull model (or flat art) in a
//! small dark disc ringed friend-or-foe (`theme::iff`), with a pip in its
//! faction's colour (grey if independent). Smaller than own fleet chips.
use super::art::{ArtKind, Artwork};
use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use game_core::fleet::ShipKind;

/// Marker radius in points.
pub(super) const TRADER: f32 = 12.0;

/// Where the `index`th trader resting at a body goes, `clearance` points
/// from its centre: below-right, below, upper-left, left, then right (names
/// sit up-right; own fleet chips down-left), then on a wider ring.
pub(super) fn berth_spot(anchor: Pos2, clearance: f32, index: usize) -> Pos2 {
    const ANGLES: [f32; 5] = [0.8, 1.6, -2.4, -3.1, 0.0];
    let ring = (index / ANGLES.len()) as f32;
    let distance = clearance + TRADER + 10.0 + ring * (TRADER * 2.0 + 4.0);
    anchor + Vec2::angled(ANGLES[index % ANGLES.len()]) * distance
}

pub(super) fn paint(
    painter: &Painter,
    art: &Artwork,
    at: Pos2,
    kind: ShipKind,
    ring: Color32,
    pip: Color32,
) {
    painter.circle_filled(at, TRADER, Color32::from_black_alpha(190));
    painter.circle_stroke(at, TRADER, Stroke::new(1.2_f32, ring));
    if let Some(model) = crate::battle::model::for_kind(kind) {
        model.paint_marker(painter, at, TRADER * 1.8);
    } else if !art.paint(ArtKind::from(kind), painter, at, TRADER * 1.65) {
        painter.circle_filled(at, 3.0, ring);
    }
    let dot = at + Vec2::splat(TRADER * 0.7) * Vec2::new(1.0, -1.0);
    painter.circle_filled(dot, 3.5, pip);
    painter.circle_stroke(dot, 3.5, Stroke::new(1.0_f32, Color32::BLACK));
}
