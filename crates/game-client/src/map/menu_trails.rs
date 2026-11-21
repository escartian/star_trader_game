//! Engine trails for the main menu's ships: a tapering, fading ribbon traced
//! back along each ship's recent route from its stern, as a smooth mesh whose
//! vertex colours fade out. A wide glow under a white-hot core reads as exhaust.
use crate::battle::camera::{Projector, ScenePoint};
use egui::{Color32, Painter};
use game_core::fleet::ShipKind;
use glam::Vec3;

/// Samples along a trail (more makes the taper smoother).
const SEGMENTS: usize = 16;
/// Trail length as a multiple of the ship's length.
const LENGTH_IN_HULLS: f32 = 2.6;

/// Exhaust colour: warm for freighters and prospectors, cyan for warships.
fn exhaust(kind: ShipKind) -> [u8; 3] {
    match kind {
        ShipKind::Freighter | ShipKind::Prospector => [255, 170, 90],
        ShipKind::Patrol | ShipKind::Scout => [120, 220, 255],
    }
}

/// Paints the trail of a ship of `kind` and `size` (world units) whose route
/// position at a given progress (0-1 through the pass) is `route`. The trail
/// starts at the stern and never reaches back before the pass began.
pub(super) fn paint(
    painter: &Painter,
    projector: &Projector,
    kind: ShipKind,
    size: f32,
    progress: f32,
    route_length: f32,
    route: impl Fn(f32) -> Vec3,
) {
    let head = route(progress);
    let span = (size * LENGTH_IN_HULLS / route_length.max(f32::EPSILON)).min(progress);
    if span <= 0.0 {
        return;
    }
    let forward = (head - route(progress - span)).normalize_or_zero();
    let stern = forward * size * 0.45;
    let points: Vec<_> = (0..=SEGMENTS)
        .map(|step| {
            let back = span * step as f32 / SEGMENTS as f32;
            projector.project(ScenePoint(route(progress - back) - stern))
        })
        .collect();
    let points: Vec<_> = points.into_iter().map_while(|point| point).collect();
    if points.len() < 2 {
        return;
    }
    let [r, g, b] = exhaust(kind);
    for (glow, width_scale, alpha_scale) in [(true, 2.8, 0.2), (false, 1.0, 1.0)] {
        let mut mesh = egui::Mesh::default();
        for (step, point) in points.iter().enumerate() {
            // 1 at the stern, falling to 0 at the tail.
            let fresh = 1.0 - step as f32 / SEGMENTS as f32;
            let half_width = point.scale * size * 0.035 * fresh * width_scale;
            let along =
                points[(step + 1).min(points.len() - 1)].pos - points[step.saturating_sub(1)].pos;
            let across = along.normalized().rot90() * half_width.max(0.3);
            let alpha = 210.0 * fresh.powf(1.6) * alpha_scale;
            let (r, g, b) = if glow {
                (r, g, b)
            } else {
                lighten(r, g, b, fresh)
            };
            let color = Color32::from_rgba_unmultiplied(r, g, b, alpha as u8);
            mesh.colored_vertex(point.pos + across, color);
            mesh.colored_vertex(point.pos - across, color);
            if step > 0 {
                let base = 2 * step as u32 - 2;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 3, base + 2);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// The core runs white-hot near the nozzle and takes the exhaust colour later.
fn lighten(r: u8, g: u8, b: u8, heat: f32) -> (u8, u8, u8) {
    let mix = |channel: u8| (f32::from(channel) + (255.0 - f32::from(channel)) * heat * 0.7) as u8;
    (mix(r), mix(g), mix(b))
}
