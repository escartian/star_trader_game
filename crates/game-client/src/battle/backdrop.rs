//! The battle's backdrop: deep-space gradient and stars, the floor grid with
//! both deployment edges, the volume's outline and the shown layer.
use super::camera::ScenePoint;
use super::canvas::Canvas;
use super::scene::HOSTILE;
use crate::theme::{CYAN, GREEN};
use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke};
use game_core::combat::tactical::GridSize;
use glam::Vec3;

const GRID: Color32 = Color32::from_rgb(70, 112, 116);
const SKY_TOP: Color32 = Color32::from_rgb(12, 20, 32);
const SKY_BOTTOM: Color32 = Color32::from_rgb(3, 6, 10);

/// Gradient sky and a fixed starfield that drifts slightly as the camera turns.
pub fn sky(painter: &egui::Painter, rect: Rect, yaw: f32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), SKY_TOP);
    mesh.colored_vertex(rect.right_top(), SKY_TOP);
    mesh.colored_vertex(rect.right_bottom(), SKY_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), SKY_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
    let drift = yaw * 60.0;
    for star in 0..140_u32 {
        // Cheap integer hash: stable positions without storing a star list.
        let hash = |salt: u32| {
            let mut value = star.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
            value ^= value >> 15;
            value = value.wrapping_mul(0x2C1B_3C6D);
            value ^= value >> 12;
            (value % 10_000) as f32 / 10_000.0
        };
        let x = (hash(1) * rect.width() + drift).rem_euclid(rect.width());
        let position = Pos2::new(rect.left() + x, rect.top() + hash(2) * rect.height());
        let bright = 0.25 + 0.75 * hash(3).powi(3);
        let radius = if hash(4) > 0.94 { 1.4 } else { 0.8 };
        painter.circle_filled(
            position,
            radius,
            Color32::from_white_alpha((bright * 200.0) as u8),
        );
    }
}

/// The volume's bounds in scene space: (low corner, high corner). Cells sit on
/// integer points, so the volume reaches half a cell past the outer cells.
fn bounds(size: GridSize) -> (Vec3, Vec3) {
    let high = Vec3::new(
        f32::from(size.width),
        f32::from(size.depth),
        f32::from(size.height),
    ) - Vec3::splat(0.5);
    (Vec3::splat(-0.5), high)
}

/// Floor grid fading with distance, both deployment edges and the volume outline.
pub fn floor(canvas: &Canvas, size: GridSize) {
    let (low, high) = bounds(size);
    let floor = low.z;
    let strip = |x0: f32, x1: f32| {
        [
            Vec3::new(x0, low.y, floor),
            Vec3::new(x1, low.y, floor),
            Vec3::new(x1, high.y, floor),
            Vec3::new(x0, high.y, floor),
        ]
    };
    canvas.quad(
        strip(low.x, high.x),
        GRID.gamma_multiply(0.05),
        Stroke::NONE,
    );
    canvas.quad(
        strip(low.x, low.x + 1.0),
        CYAN.gamma_multiply(0.16),
        Stroke::NONE,
    );
    canvas.quad(
        strip(high.x - 1.0, high.x),
        HOSTILE.gamma_multiply(0.16),
        Stroke::NONE,
    );
    let middle = (low + high) / 2.0;
    let near = canvas
        .projector
        .project(ScenePoint(Vec3::new(middle.x, middle.y, floor)))
        .map_or(10.0, |at| at.depth);
    let faded = |a: Vec3, b: Vec3| {
        let depth = canvas
            .projector
            .project(ScenePoint((a + b) / 2.0))
            .map_or(near, |at| at.depth);
        // Lines nearer than the floor's centre are brighter, farther ones dimmer.
        let weight = (near / depth).powi(2).clamp(0.25, 1.4);
        Stroke::new(1.0_f32, GRID.gamma_multiply(0.45 * weight))
    };
    for step in 0..=size.width {
        let x = low.x + f32::from(step);
        let (a, b) = (Vec3::new(x, low.y, floor), Vec3::new(x, high.y, floor));
        canvas.line(ScenePoint(a), ScenePoint(b), faded(a, b));
    }
    for step in 0..=size.depth {
        let y = low.y + f32::from(step);
        let (a, b) = (Vec3::new(low.x, y, floor), Vec3::new(high.x, y, floor));
        canvas.line(ScenePoint(a), ScenePoint(b), faded(a, b));
    }
    let edge = Stroke::new(1.0_f32, GRID.gamma_multiply(0.35));
    let corners = [
        (low.x, low.y),
        (high.x, low.y),
        (high.x, high.y),
        (low.x, high.y),
    ];
    for (index, (x, y)) in corners.iter().enumerate() {
        let (nx, ny) = corners[(index + 1) % 4];
        canvas.line(
            ScenePoint(Vec3::new(*x, *y, floor)),
            ScenePoint(Vec3::new(*x, *y, high.z)),
            edge,
        );
        canvas.line(
            ScenePoint(Vec3::new(*x, *y, high.z)),
            ScenePoint(Vec3::new(nx, ny, high.z)),
            edge,
        );
    }
}

/// The layer whose move cells are shown: a faint plane with a bright rim.
pub fn layer(canvas: &Canvas, size: GridSize, layer: i8) {
    let (low, high) = bounds(size);
    let z = f32::from(layer);
    canvas.quad(
        [
            Vec3::new(low.x, low.y, z),
            Vec3::new(high.x, low.y, z),
            Vec3::new(high.x, high.y, z),
            Vec3::new(low.x, high.y, z),
        ],
        GREEN.gamma_multiply(0.035),
        Stroke::new(1.0_f32, GREEN.gamma_multiply(0.3)),
    );
}
