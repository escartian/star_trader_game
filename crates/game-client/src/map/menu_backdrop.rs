//! The main menu's backdrop: stars and faint nebula glows fixed to the sky,
//! behind the 3D ships, station and planet of `menu_scene`. Decorative only.
//!
//! Sky objects are directions, drawn far out along them through the menu
//! camera, so they only move when the camera swings (no parallax: they are
//! effectively at infinity). Stars twinkle in place. Star placement is a fixed
//! pseudo-random pattern, so every launch shows the same sky.
use super::art::Artwork;
use crate::battle::camera::ScenePoint;
use egui::{Color32, Painter, Pos2, Rect, Vec2};
use glam::{Mat3, Vec3};

const STARS: usize = 320;
/// Sky objects are drawn this far from the camera (inside the 400-unit far plane).
const SKY_DISTANCE: f32 = 350.0;
/// Half the sky's spread around the view direction, radians: wider than the
/// view plus the camera's swing, so no edge ever shows.
const SKY_YAW: f32 = 1.15;
const SKY_PITCH: f32 = 0.95;
/// The moment shown when animations are off (ships in view).
pub(super) const STILL_TIME: f64 = 15.0;

/// Nebula glows: yaw and pitch from the view direction (radians), radius in
/// world units at `SKY_DISTANCE`, and colour.
const NEBULAE: [(f32, f32, f32, [u8; 4]); 3] = [
    (-0.42, 0.12, 150.0, [30, 75, 110, 70]),
    (0.45, 0.3, 115.0, [65, 35, 100, 60]),
    (0.05, -0.42, 125.0, [25, 70, 60, 50]),
];

/// A small deterministic generator (SplitMix64) giving floats in [0, 1).
struct Sky(u64);

impl Sky {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) as f64 / u64::MAX as f64) as f32
    }
}

/// The sky direction `yaw` radians right and `pitch` radians up from the
/// menu's view direction.
fn sky_direction(yaw: f32, pitch: f32) -> Vec3 {
    let forward = super::menu_scene::view_direction();
    let right = forward.cross(Vec3::Z).normalize();
    Mat3::from_rotation_z(-yaw) * (Mat3::from_axis_angle(right, pitch) * forward)
}

/// Paints the sky and then the scene into `rect` as at `time` seconds.
/// `launch_seed` picks the planet and ships (see `menu_scene`).
pub(super) fn paint(painter: &Painter, rect: Rect, time: f64, art: &Artwork, launch_seed: u64) {
    let time = time as f32;
    let projector = super::menu_scene::camera(rect, time);
    let on_sky =
        |direction: Vec3| projector.project(ScenePoint(projector.eye + direction * SKY_DISTANCE));
    painter.rect_filled(rect, 0.0, Color32::from_rgb(3, 6, 12));
    for (yaw, pitch, radius, [r, g, b, a]) in NEBULAE {
        if let Some(at) = on_sky(sky_direction(yaw, pitch)) {
            let color = Color32::from_rgba_unmultiplied(r, g, b, a);
            painter.add(glow(at.pos, radius * at.scale, color));
        }
    }
    let mut sky = Sky(0x5EED_57A2);
    for _ in 0..STARS {
        let (yaw, pitch, brightness, phase, warm) =
            (sky.next(), sky.next(), sky.next(), sky.next(), sky.next());
        let direction = sky_direction((yaw * 2.0 - 1.0) * SKY_YAW, (pitch * 2.0 - 1.0) * SKY_PITCH);
        let Some(at) = on_sky(direction) else {
            continue;
        };
        if !rect.expand(4.0).contains(at.pos) {
            continue;
        }
        let rate = 0.4 + brightness * 1.6;
        let twinkle = 0.75 + 0.25 * (time * rate + phase * std::f32::consts::TAU).sin();
        let alpha = ((0.3 + 0.7 * brightness) * twinkle * 255.0) as u8;
        let tint = if warm > 0.85 {
            [255, 214, 170]
        } else if warm < 0.25 {
            [170, 205, 255]
        } else {
            [230, 236, 245]
        };
        let color = Color32::from_rgba_unmultiplied(tint[0], tint[1], tint[2], alpha);
        painter.circle_filled(at.pos, 0.5 + 1.5 * brightness * brightness, color);
    }
    super::menu_scene::paint(painter, &projector, time, art, launch_seed);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_stay_put_while_the_camera_is_still() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 720.0));
        // At the turn of a swing (a quarter period) the camera is still.
        let turn = 45.0;
        let star = sky_direction(0.1, 0.05);
        let at = |time| {
            let projector = super::super::menu_scene::camera(rect, time);
            projector
                .project(ScenePoint(projector.eye + star * SKY_DISTANCE))
                .unwrap()
                .pos
        };
        assert!(
            at(turn).distance(at(turn + 0.05)) < 0.05,
            "no drift without camera motion"
        );
        assert!(
            at(0.0).distance(at(turn)) > 20.0,
            "the camera swing moves the sky"
        );
    }
}
