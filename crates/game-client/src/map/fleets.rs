//! Fleet marker placement is shared by painting and picking.
use super::art::{ArtKind, Artwork};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use game_core::{
    fleet::{Fleet, FleetId},
    travel::{Ship, ShipId},
};

#[derive(Clone, Debug)]
pub(super) struct FleetHit {
    pub fleet: FleetId,
    pub ship: ShipId,
    pub bounds: Rect,
}

pub(super) struct FleetSymbol<'a> {
    pub fleet: &'a Fleet,
    pub ship: &'a Ship,
    pub selected: bool,
}

pub(super) fn paint(
    painter: &Painter,
    art: &Artwork,
    viewport: Rect,
    anchor: Pos2,
    symbol: FleetSymbol<'_>,
    reserved: &mut Vec<Rect>,
) -> Option<FleetHit> {
    let FleetSymbol {
        fleet,
        ship,
        selected,
    } = symbol;
    let bounds = place(anchor, viewport, reserved)?;
    reserved.push(bounds.expand(3.0));
    let center = bounds.center() - Vec2::new(0.0, 8.0);
    let color = if selected {
        crate::theme::GREEN
    } else {
        Color32::from_rgb(150, 185, 205)
    };
    // Tethers do not place fleet symbols at body/star centers.
    if anchor.distance(center) > 32.0 {
        painter.line_segment(
            [anchor, center],
            Stroke::new(0.7_f32, color.gamma_multiply(0.5)),
        );
        painter.circle_stroke(anchor, 3.0, Stroke::new(1.0_f32, color));
    }
    painter.rect_filled(
        Rect::from_center_size(center, Vec2::splat(42.0)),
        7.0,
        Color32::from_rgba_unmultiplied(6, 21, 34, 230),
    );
    painter.circle_stroke(
        center,
        20.0,
        Stroke::new(if selected { 1.8_f32 } else { 0.7_f32 }, color),
    );
    art.paint(ArtKind::from(ship.kind), painter, center, 34.0);
    painter.text(
        center + Vec2::new(0.0, 23.0),
        Align2::CENTER_TOP,
        format!("F{} · {}", fleet.id.0, fleet.ships.len()),
        FontId::monospace(11.0),
        color,
    );
    Some(FleetHit {
        fleet: fleet.id,
        ship: ship.id,
        bounds,
    })
}

pub(super) fn place(anchor: Pos2, viewport: Rect, reserved: &[Rect]) -> Option<Rect> {
    // Free-flying fleets sit on their course position. Occupied anchors need offsets.
    let direct = Rect::from_center_size(anchor + Vec2::new(0.0, 8.0), Vec2::new(50.0, 66.0));
    if viewport.contains_rect(direct)
        && !reserved
            .iter()
            .any(|other| other.expand(5.0).intersects(direct))
    {
        return Some(direct);
    }
    for radius in [72.0, 102.0, 138.0, 180.0, 230.0] {
        for offset in [
            Vec2::new(1.0, -0.65),
            Vec2::new(-1.0, 0.65),
            Vec2::new(1.0, 0.65),
            Vec2::new(-1.0, -0.65),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
        ] {
            let bounds = Rect::from_center_size(anchor + offset * radius, Vec2::new(50.0, 66.0));
            if viewport.contains_rect(bounds)
                && !reserved
                    .iter()
                    .any(|other| other.expand(5.0).intersects(bounds))
            {
                return Some(bounds);
            }
        }
    }
    None // The roster remains selectable when the viewport has no safe symbol space.
}

pub(super) fn pick(hits: &[FleetHit], pointer: Pos2) -> Option<&FleetHit> {
    hits.iter().find(|hit| hit.bounds.contains(pointer))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projected_fleets_never_cover_a_star_and_pick_uses_the_painted_bounds() {
        let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let star = Rect::from_center_size(view.center(), Vec2::splat(120.0));
        let mut occupied = vec![star];
        for id in 1..6 {
            let bounds = place(view.center(), view, &occupied).unwrap();
            assert!(!bounds.intersects(star));
            let hit = FleetHit {
                fleet: FleetId(id),
                ship: ShipId(id),
                bounds,
            };
            assert_eq!(pick(&[hit], bounds.center()).unwrap().fleet, FleetId(id));
            occupied.push(bounds);
        }
        assert!(place(view.center(), view, &[view]).is_none());
    }

    #[test]
    fn free_flying_fleet_symbol_stays_at_its_physical_anchor() {
        let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let anchor = view.center();
        let bounds = place(anchor, view, &[]).unwrap();
        assert_eq!(bounds.center() - Vec2::new(0.0, 8.0), anchor);
    }
}
