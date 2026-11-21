//! Roaming raider bands on the system map: a red diamond tagged with the band's
//! ships, and, only while it hunts one of your fleets or stalks an NPC trader,
//! a dashed line toward its quarry. Positions are true kilometres, like fleets in flight.
use super::{KM_PER_VIEW_UNIT, LocalView, kilometers};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use game_core::fleet::ShipKind;

const RAIDER: Color32 = Color32::from_rgb(255, 96, 80);

impl LocalView {
    pub(super) fn draw_bands(&self, painter: &Painter, rect: Rect) {
        for band in self
            .bands
            .iter()
            .filter(|band| band.system == self.system.id)
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
            diamond(painter, at, 9.0);
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
            let origin = at + Vec2::new(12.0, -galley.size().y / 2.0);
            painter.rect_filled(
                Rect::from_min_size(origin, galley.size()).expand(2.0),
                2.0,
                Color32::from_black_alpha(170),
            );
            painter.galley(origin, galley, RAIDER);
        }
    }
}

fn dash(painter: &Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    let length = from.distance(to);
    let steps = (length / 6.0).max(1.0) as usize;
    for step in (0..steps).step_by(2) {
        let a = from.lerp(to, step as f32 / steps as f32);
        let b = from.lerp(to, ((step + 1) as f32 / steps as f32).min(1.0));
        painter.line_segment([a, b], stroke);
    }
}

fn diamond(painter: &Painter, at: Pos2, size: f32) {
    let points = vec![
        at + Vec2::new(0.0, -size),
        at + Vec2::new(size, 0.0),
        at + Vec2::new(0.0, size),
        at + Vec2::new(-size, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        RAIDER.gamma_multiply(0.35),
        Stroke::new(1.6_f32, RAIDER),
    ));
}

/// "2 scouts", "patrol + scout".
pub(super) fn crew(ships: &[ShipKind]) -> String {
    let count = |kind| ships.iter().filter(|ship| **ship == kind).count();
    let parts: Vec<String> = [
        (ShipKind::Patrol, "patrol", "patrols"),
        (ShipKind::Scout, "scout", "scouts"),
        (ShipKind::Freighter, "freighter", "freighters"),
        (ShipKind::Prospector, "prospector", "prospectors"),
    ]
    .into_iter()
    .filter_map(|(kind, one, many)| match count(kind) {
        0 => None,
        1 => Some(one.to_owned()),
        n => Some(format!("{n} {many}")),
    })
    .collect();
    parts.join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crews_read_naturally() {
        assert_eq!(crew(&[ShipKind::Scout, ShipKind::Scout]), "2 scouts");
        assert_eq!(crew(&[ShipKind::Patrol, ShipKind::Scout]), "patrol + scout");
    }
}
