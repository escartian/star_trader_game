//! Roaming raider bands on the system map: a red-ringed ship marker (its
//! heaviest hull and a count, `npc_marker`) tagged with the band's ships, and, only while it hunts one of your fleets or stalks an NPC trader,
//! a dashed line toward its quarry. Positions are true kilometres, like fleets in flight.
use super::{KM_PER_VIEW_UNIT, LocalView, kilometers};
use crate::map::npc_marker;
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

const RAIDER: Color32 = crate::theme::iff::HOSTILE;
/// Gap between a band's or patrol's marker and its tag, in points: clear of a
/// fleet chip (radius 15) drawn on top when the two meet.
pub(super) const TAG_GAP: f32 = 22.0;

impl LocalView {
    pub(super) fn draw_bands(&mut self, painter: &Painter, rect: Rect) {
        let mut hits = Vec::new();
        for band in self
            .bands
            .iter()
            .filter(|band| band.system == self.system.id && band.transit.is_none())
        {
            let at = self
                .camera
                .project(kilometers(band.position) / KM_PER_VIEW_UNIT, rect);
            let towards = self
                .camera
                .project(kilometers(band.heading) / KM_PER_VIEW_UNIT, rect);
            if !rect.expand(40.0).contains(at) {
                continue;
            }
            let stalking = band.stalking.is_some();
            if (band.hunting.is_some() || stalking) && at.distance(towards) > 6.0 {
                dash(
                    painter,
                    at,
                    towards,
                    Stroke::new(1.2_f32, RAIDER.gamma_multiply(0.9)),
                );
            }
            hits.push(npc_marker::NpcHit {
                quarry: game_core::pursuit::Quarry::Band(band.id),
                at,
            });
            if let (Some(art), Some(kind)) = (&self.art, npc_marker::lead(&band.ships)) {
                npc_marker::paint(painter, art, at, kind, RAIDER, None, band.ships.len());
            }
            let label = format!(
                "{}RAIDERS · {}",
                match (band.hunting.is_some(), stalking) {
                    (true, _) => "HUNTING · ",
                    (false, true) => "STALKING TRADER · ",
                    _ => "",
                },
                crew(&band.ships)
            );
            let galley = painter.layout_no_wrap(label, egui::FontId::proportional(11.0), RAIDER);
            let origin = at + Vec2::new(TAG_GAP, -galley.size().y / 2.0);
            painter.rect_filled(
                Rect::from_min_size(origin, galley.size()).expand(2.0),
                2.0,
                Color32::from_black_alpha(170),
            );
            painter.galley(origin, galley, RAIDER);
        }
        self.npc_hits.extend(hits);
    }
}

pub(crate) fn dash(painter: &Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    let length = from.distance(to);
    let steps = (length / 6.0).max(1.0) as usize;
    for step in (0..steps).step_by(2) {
        let a = from.lerp(to, step as f32 / steps as f32);
        let b = from.lerp(to, ((step + 1) as f32 / steps as f32).min(1.0));
        painter.line_segment([a, b], stroke);
    }
}

pub(super) use crate::session::crew;
