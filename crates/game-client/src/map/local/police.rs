//! Faction police patrols on the system map: a hexagon in the faction's
//! colour tagged with its ships, and, while it pursues one of your fleets or
//! closes on a raider band, a dashed line toward its quarry. Positions are
//! true kilometres, like raider bands.
use super::raiders::{TAG_GAP, crew, dash};
use super::{KM_PER_VIEW_UNIT, LocalView, kilometers};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

impl LocalView {
    pub(super) fn draw_police(&self, painter: &Painter, rect: Rect) {
        for (patrol, color) in self
            .police
            .iter()
            .filter(|(patrol, _)| patrol.system == self.system.id)
        {
            let at = self
                .camera
                .project(kilometers(patrol.position) / KM_PER_VIEW_UNIT, rect);
            if !rect.expand(40.0).contains(at) {
                continue;
            }
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
            hexagon(painter, at, 8.0, *color);
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
    }
}

fn hexagon(painter: &Painter, at: Pos2, size: f32, color: Color32) {
    let points = (0..6)
        .map(|corner| {
            let angle = std::f32::consts::FRAC_PI_3 * corner as f32;
            at + Vec2::new(angle.cos(), angle.sin()) * size
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(
        points,
        color.gamma_multiply(0.35),
        Stroke::new(1.6_f32, color),
    ));
}
