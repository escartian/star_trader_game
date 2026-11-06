//! Fleet marker placement is shared by painting and picking.
use super::art::{ArtKind, Artwork};
use crate::theme::icons::Icon;
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
    pub berth: Berth,
}

/// How a fleet is resting, for presentation only. `trade` is decided by the
/// authoritative economy (a market is reachable from this exact location).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Berth {
    /// Flying, waiting at a waypoint, or in deep space.
    Free,
    /// Clamped to the system's trade port.
    Docked { trade: bool },
    /// Holding orbit around a body; `ring` is the on-screen orbit radius in points
    /// (0 hides the ring, e.g. at galaxy scale).
    Orbit { ring: f32, trade: bool },
}

impl Berth {
    pub fn new(location: &game_core::travel::Location, trade: bool, ring: f32) -> Self {
        match location {
            game_core::travel::Location::Docked(_) => Self::Docked { trade },
            game_core::travel::Location::Orbiting { .. } => Self::Orbit { ring, trade },
            _ => Self::Free,
        }
    }

    fn tag(self) -> Option<(&'static str, Color32, Icon)> {
        match self {
            Self::Free => None,
            Self::Docked { .. } => Some(("DOCKED", crate::theme::GREEN, Icon::Dock)),
            Self::Orbit { .. } => Some(("ORBIT", crate::theme::CYAN, Icon::Orbit)),
        }
    }

    fn trade(self) -> bool {
        matches!(
            self,
            Self::Docked { trade: true } | Self::Orbit { trade: true, .. }
        )
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
            Stroke::new(1.2_f32, color.gamma_multiply(0.9)),
        );
    }
}

fn badge(painter: &Painter, center: Pos2, icon: Icon, color: Color32) {
    painter.circle_filled(center, 8.5, Color32::from_rgb(8, 18, 24));
    painter.circle_stroke(center, 8.5, Stroke::new(1.0_f32, color));
    crate::theme::icons::paint_sized(painter, icon, center, 12.0, color);
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
        berth,
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
    match berth {
        Berth::Docked { .. } => {
            // A solid clamp line and bracket: physically attached to the port.
            let green = crate::theme::GREEN;
            painter.line_segment(
                [anchor, center],
                Stroke::new(2.0_f32, green.gamma_multiply(0.8)),
            );
            let clamp = Rect::from_center_size(anchor, Vec2::splat(14.0));
            painter.rect_stroke(
                clamp,
                2.0,
                Stroke::new(1.6_f32, green),
                egui::StrokeKind::Middle,
            );
        }
        Berth::Orbit { ring, .. } => {
            let cyan = crate::theme::CYAN;
            if ring > 0.0 {
                orbit_ring(painter, anchor, ring, cyan);
            }
            if anchor.distance(center) > 32.0 {
                painter.line_segment(
                    [anchor, center],
                    Stroke::new(0.7_f32, cyan.gamma_multiply(0.5)),
                );
            }
        }
        Berth::Free if anchor.distance(center) > 32.0 => {
            painter.line_segment(
                [anchor, center],
                Stroke::new(0.7_f32, color.gamma_multiply(0.5)),
            );
            painter.circle_stroke(anchor, 3.0, Stroke::new(1.0_f32, color));
        }
        Berth::Free => {}
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
    let (label, label_color) = match berth.tag() {
        Some((tag, tag_color, icon)) => {
            badge(painter, center + Vec2::new(17.0, -17.0), icon, tag_color);
            (format!("F{} {tag}", fleet.id.0), tag_color)
        }
        None => (format!("F{} · {}", fleet.id.0, fleet.ships.len()), color),
    };
    if berth.trade() {
        badge(
            painter,
            center + Vec2::new(-17.0, -17.0),
            Icon::Market,
            crate::theme::GREEN,
        );
    }
    painter.text(
        center + Vec2::new(0.0, 23.0),
        Align2::CENTER_TOP,
        label,
        FontId::proportional(12.0),
        label_color,
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
