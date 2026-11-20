//! The battle's backdrop: deep-space gradient and stars, and the hex board
//! with both withdrawal edges.
use super::camera::{HEX_RADIUS, ScenePoint};
use super::canvas::Canvas;
use super::scene::HOSTILE;
use crate::theme::CYAN;
use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke};
use game_core::combat::{
    Side,
    tactical::{Board, Hex},
};
use glam::Vec3;

const GRID: Color32 = Color32::from_rgb(70, 112, 116);
const SKY_TOP: Color32 = Color32::from_rgb(12, 20, 32);
const SKY_BOTTOM: Color32 = Color32::from_rgb(3, 6, 10);

/// Gradient sky and a fixed starfield that drifts slightly as the view pans.
pub fn sky(painter: &egui::Painter, rect: Rect, drift: f32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), SKY_TOP);
    mesh.colored_vertex(rect.right_top(), SKY_TOP);
    mesh.colored_vertex(rect.right_bottom(), SKY_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), SKY_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
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

/// The six corners of `hex` on the board plane, shrunk by `inset` (0–1).
pub fn hex_corners(hex: Hex, inset: f32) -> [ScenePoint; 6] {
    let centre = ScenePoint::of_hex(hex).0;
    std::array::from_fn(|corner| {
        let angle = corner as f32 * std::f32::consts::FRAC_PI_3;
        let offset = Vec3::new(angle.cos(), angle.sin(), 0.0) * HEX_RADIUS * (1.0 - inset);
        ScenePoint(centre + offset)
    })
}

/// A filled hex on the board plane.
pub fn hex(canvas: &Canvas, hex: Hex, inset: f32, fill: Color32, stroke: Stroke) {
    let points: Option<Vec<Pos2>> = hex_corners(hex, inset)
        .iter()
        .map(|corner| canvas.at(*corner))
        .collect();
    if let Some(points) = points {
        canvas
            .painter
            .add(Shape::convex_polygon(points, fill, stroke));
    }
}

/// The board: every hex outlined, the two withdrawal edges tinted in their
/// side's colour.
pub fn board(canvas: &Canvas, board: Board) {
    for cell in board.hexes() {
        let (fill, line) = if board.at_home_edge(Side::Player, cell) {
            (CYAN.gamma_multiply(0.14), CYAN.gamma_multiply(0.45))
        } else if board.at_home_edge(Side::Hostile, cell) {
            (HOSTILE.gamma_multiply(0.12), HOSTILE.gamma_multiply(0.45))
        } else {
            (GRID.gamma_multiply(0.06), GRID.gamma_multiply(0.5))
        };
        hex(canvas, cell, 0.04, fill, Stroke::new(1.0_f32, line));
    }
}
