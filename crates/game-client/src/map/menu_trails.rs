//! Engine trails for the main menu's ships: a tapering ribbon traced back along
//! each ship's recent route from its stern, as a smooth mesh whose vertex
//! colours fade out. A wide glow sits under a white-hot core. Bands of
//! brightness flow back along it, its length and width flicker, and a flare
//! pulses at the nozzle, so it reads as live exhaust rather than a streak.
use crate::battle::camera::{Projector, ScenePoint};
use egui::{Color32, Painter};
use game_core::fleet::ShipKind;
use glam::Vec3;

/// Samples along a trail (more makes the taper smoother).
const SEGMENTS: usize = 20;
/// Trail length as a multiple of the ship's length, before flicker.
const LENGTH_IN_HULLS: f32 = 2.6;

pub(super) struct Trail {
    pub kind: ShipKind,
    /// Ship length in world units.
    pub size: f32,
    /// How far along its route (0-1) the ship is; the trail never reaches back
    /// before 0.
    pub progress: f32,
    /// Seconds, for the flicker and flow.
    pub time: f32,
    /// Per-ship offset, so trails do not pulse in step.
    pub phase: f32,
}

/// Exhaust colour: warm for freighters and prospectors, cyan for warships.
fn exhaust(kind: ShipKind) -> [u8; 3] {
    match kind {
        ShipKind::Freighter | ShipKind::Prospector => [255, 170, 90],
        ShipKind::Patrol | ShipKind::Scout => [120, 220, 255],
    }
}

impl Trail {
    /// How far back in progress the trail reaches to cover `length` world
    /// units of route actually flown. Measured along the route, so eased
    /// (braking or accelerating) and short station legs get true lengths.
    fn span_for(&self, length: f32, route: &impl Fn(f32) -> Vec3) -> f32 {
        const STEP: f32 = 1.0 / 1500.0;
        let (mut back, mut travelled) = (self.progress, 0.0);
        let mut previous = route(back);
        while travelled < length && back > 0.0 {
            back = (back - STEP).max(0.0);
            let point = route(back);
            travelled += point.distance(previous);
            previous = point;
        }
        self.progress - back
    }

    /// Paints the trail of a ship whose position at a given progress is `route`.
    pub fn paint(&self, painter: &Painter, projector: &Projector, route: impl Fn(f32) -> Vec3) {
        let (time, phase) = (self.time, self.phase);
        let flicker = 0.85 + 0.1 * (time * 7.0 + phase).sin() + 0.05 * (time * 23.0 + phase).sin();
        let length = self.size * LENGTH_IN_HULLS * flicker;
        let span = self.span_for(length, &route);
        if span <= 0.0 {
            return;
        }
        let head = route(self.progress);
        let forward = (head - route(self.progress - span)).normalize_or_zero();
        let stern = forward * self.size * 0.45;
        let points: Vec<_> = (0..=SEGMENTS)
            .map_while(|step| {
                let back = span * step as f32 / SEGMENTS as f32;
                projector.project(ScenePoint(route(self.progress - back) - stern))
            })
            .collect();
        if points.len() < 2 {
            return;
        }
        let [r, g, b] = exhaust(self.kind);
        let breathe = 1.0 + 0.18 * (time * 31.0 + phase * 3.0).sin();
        for (glow, width_scale, alpha_scale) in [(true, 2.8, 0.2), (false, 1.0, 1.0)] {
            let mut mesh = egui::Mesh::default();
            for (step, point) in points.iter().enumerate() {
                // 1 at the stern, falling to 0 at the tail.
                let fresh = 1.0 - step as f32 / SEGMENTS as f32;
                // Bright bands travelling back along the trail.
                let flow = 0.72 + 0.28 * (step as f32 * 0.9 - time * 16.0 + phase).sin();
                let half_width = point.scale * self.size * 0.035 * fresh * width_scale * breathe;
                let along = points[(step + 1).min(points.len() - 1)].pos
                    - points[step.saturating_sub(1)].pos;
                let across = along.normalized().rot90() * half_width.max(0.3);
                let alpha = 215.0 * fresh.powf(1.5) * alpha_scale * flow;
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
        // A pulsing flare at the nozzle.
        let nozzle = points[0];
        let pulse = 0.75 + 0.25 * (time * 19.0 + phase * 2.0).sin();
        let radius = nozzle.scale * self.size * 0.07 * pulse;
        let (fr, fg, fb) = lighten(r, g, b, 1.0);
        painter.circle_filled(
            nozzle.pos,
            radius * 2.2,
            Color32::from_rgba_unmultiplied(r, g, b, (60.0 * pulse) as u8),
        );
        painter.circle_filled(
            nozzle.pos,
            radius,
            Color32::from_rgba_unmultiplied(fr, fg, fb, (230.0 * pulse) as u8),
        );
    }
}

/// The core runs white-hot near the nozzle and takes the exhaust colour later.
fn lighten(r: u8, g: u8, b: u8, heat: f32) -> (u8, u8, u8) {
    let mix = |channel: u8| (f32::from(channel) + (255.0 - f32::from(channel)) * heat * 0.7) as u8;
    (mix(r), mix(g), mix(b))
}
