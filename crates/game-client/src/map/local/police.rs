//! Faction police patrols on the system map: a ship marker (`npc_marker`)
//! ringed friend-or-foe with a pip in the faction's colour, tagged with its
//! ships, and, while it pursues one of your fleets or closes on a raider band,
//! a dashed friend-or-foe line toward its quarry. Positions are
//! true kilometres, like raider bands.
use super::raiders::{TAG_GAP, crew, dash};
use super::{KM_PER_VIEW_UNIT, LocalView, kilometers};
use crate::map::npc_marker;
use egui::{Color32, Painter, Rect, Stroke, Vec2};

impl LocalView {
    pub(super) fn draw_police(&mut self, painter: &Painter, rect: Rect) {
        let mut hits = Vec::new();
        for marker in self
            .police
            .iter()
            .filter(|marker| marker.patrol.system == self.system.id)
        {
            let patrol = &marker.patrol;
            let color = &marker.stance.color();
            let at = self
                .camera
                .project(kilometers(patrol.position) / KM_PER_VIEW_UNIT, rect);
            if !rect.expand(40.0).contains(at) {
                continue;
            }
            // Tappable, like traders and bands: select it, then Pursue.
            hits.push(npc_marker::NpcHit {
                quarry: game_core::pursuit::Quarry::Patrol(patrol.id),
                at,
            });
            let chasing = patrol.pursuing.is_some() || patrol.answering.is_some();
            let towards = self
                .camera
                .project(kilometers(patrol.heading) / KM_PER_VIEW_UNIT, rect);
            if chasing && at.distance(towards) > 6.0 {
                dash(
                    painter,
                    at,
                    towards,
                    Stroke::new(1.2_f32, color.gamma_multiply(0.9)),
                );
            }
            if let (Some(art), Some(kind)) = (&self.art, npc_marker::lead(&patrol.ships)) {
                npc_marker::paint(
                    painter,
                    art,
                    at,
                    kind,
                    *color,
                    Some(marker.faction),
                    patrol.ships.len(),
                );
            }
            let label = format!(
                "{}{} POLICE · {}",
                match (patrol.pursuing.is_some(), patrol.answering.is_some()) {
                    (true, _) => "PURSUING YOU · ",
                    (false, true) => "ENGAGING RAIDERS · ",
                    _ => "",
                },
                patrol.faction.label().to_uppercase(),
                crew(&patrol.ships)
            );
            let galley = painter.layout_no_wrap(label, egui::FontId::proportional(11.0), *color);
            let origin = at + Vec2::new(TAG_GAP, -galley.size().y / 2.0);
            painter.rect_filled(
                Rect::from_min_size(origin, galley.size()).expand(2.0),
                2.0,
                Color32::from_black_alpha(170),
            );
            painter.galley(origin, galley, *color);
        }
        self.npc_hits.extend(hits);
    }
}
