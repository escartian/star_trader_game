//! The main menu's backdrop: parallax stars drifting left and faint nebula
//! glows behind the 3D ships and planet of `menu_scene`. Decorative only. Star placement is a
//! fixed pseudo-random pattern, so every launch shows the same sky.
use super::art::Artwork;
use egui::{Color32, Painter, Pos2, Rect, Vec2};
use std::f64::consts::TAU;

const STARS: usize = 260;
/// Seconds for the nearest stars to drift across the screen once.
const CROSSING_SECONDS: f64 = 180.0;
/// The moment shown when animations are off (ships in view).
pub(super) const STILL_TIME: f64 = 15.0;

/// Nebula glows: centre as a fraction of the screen, radius as a fraction of
/// the shorter side, and colour.
const NEBULAE: [(f32, f32, f32, [u8; 4]); 3] = [
    (0.15, 0.30, 0.65, [30, 75, 110, 70]),
    (0.82, 0.15, 0.50, [65, 35, 100, 60]),
    (0.50, 0.95, 0.55, [25, 70, 60, 50]),
];

/// A small deterministic generator (SplitMix64) giving floats in [0, 1).
struct Sky(u64);

impl Sky {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }
}

/// Paints the backdrop into `rect` as it looks at `time` seconds.
/// `launch_seed` picks the planet and ships (see `menu_scene`).
pub(super) fn paint(painter: &Painter, rect: Rect, time: f64, art: &Artwork, launch_seed: u64) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(3, 6, 12));
    let short_side = rect.width().min(rect.height());
    for (x, y, radius, [r, g, b, a]) in NEBULAE {
        let center = rect.min + Vec2::new(x * rect.width(), y * rect.height());
        let color = Color32::from_rgba_unmultiplied(r, g, b, a);
        painter.add(glow(center, radius * short_side, color));
    }
    let mut sky = Sky(0x5EED_57A2);
    for _ in 0..STARS {
        let (x0, y, depth, phase, warm) =
            (sky.next(), sky.next(), sky.next(), sky.next(), sky.next());
        // Nearer stars (depth near 1) are larger, brighter and drift faster.
        let speed = (0.15 + 0.85 * depth * depth) / CROSSING_SECONDS;
        let x = (x0 - time * speed).rem_euclid(1.0);
        let position = rect.min + Vec2::new(x as f32 * rect.width(), y as f32 * rect.height());
        let twinkle = 0.75 + 0.25 * (time * (0.4 + depth * 1.6) + phase * TAU).sin();
        let alpha = ((0.3 + 0.7 * depth) * twinkle * 255.0) as u8;
        let tint = if warm > 0.85 {
            [255, 214, 170]
        } else if warm < 0.25 {
            [170, 205, 255]
        } else {
            [230, 236, 245]
        };
        let color = Color32::from_rgba_unmultiplied(tint[0], tint[1], tint[2], alpha);
        painter.circle_filled(position, 0.5 + 1.5 * (depth * depth) as f32, color);
    }
    super::menu_scene::paint(painter, rect, time, art, launch_seed);
}

/// A soft radial glow: `color` at the centre fading to transparent at `radius`.
fn glow(center: Pos2, radius: f32, color: Color32) -> egui::Shape {
    const SEGMENTS: u32 = 48;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    for step in 0..SEGMENTS {
        let angle = step as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let rim = center + radius * Vec2::angled(angle);
        mesh.colored_vertex(rim, Color32::TRANSPARENT);
        mesh.add_triangle(0, 1 + step, 1 + (step + 1) % SEGMENTS);
    }
    egui::Shape::mesh(mesh)
}
