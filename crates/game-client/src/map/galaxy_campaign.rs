//! Campaign marks on the galaxy map: a ring in its holder's colour around
//! every claimed system, the Raider Haven's fortress beside its system, and
//! shipping lanes between systems holding your chartered ports.
use super::{Map, position};
use crate::theme::CYAN;
use egui::{Painter, Rect, Stroke, Vec2};

impl Map {
    pub(super) fn draw_galaxy_campaign(&self, painter: &Painter, rect: Rect) {
        let marks = self.session.campaign_marks();
        let at_system = |id| {
            self.systems
                .iter()
                .find(|known| known.id == id)
                .map(|system| self.camera.project(position(system), rect))
        };
        for system in &self.systems {
            if let Some((_, color, _)) = self.session.system_holder(system.id) {
                let at = self.camera.project(position(system), rect);
                painter.circle_filled(at, 13.0, color.gamma_multiply(0.12));
                painter.circle_stroke(at, 13.0, Stroke::new(1.5_f32, color.gamma_multiply(0.8)));
            }
        }
        let mut systems: Vec<_> = marks.chartered.iter().map(|market| market.system).collect();
        systems.sort_by_key(|system| system.0);
        systems.dedup();
        let points: Vec<_> = systems.into_iter().filter_map(at_system).collect();
        for (index, from) in points.iter().enumerate() {
            for to in &points[index + 1..] {
                painter.line_segment([*from, *to], Stroke::new(5.0_f32, CYAN.gamma_multiply(0.3)));
                painter.line_segment([*from, *to], Stroke::new(2.0_f32, CYAN));
            }
        }
        if let Some(haven) = &marks.haven
            && let Some(at) = at_system(haven.system)
        {
            super::local::fortress(painter, at + Vec2::new(16.0, -18.0), haven.standing());
        }
    }
}
