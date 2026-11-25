//! Original, code-drawn hit details. No sprite textures or asset loading.
use egui::{Color32, Mesh, Painter, Pos2, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub enum Kind {
    Ring,
    Spark,
    Smoke,
}

pub fn paint(painter: &Painter, kind: Kind, center: Pos2, diameter: f32, tint: Color32) {
    if diameter <= 0.0 || tint.a() == 0 {
        return;
    }
    match kind {
        Kind::Ring => ring(painter, center, diameter, tint),
        Kind::Spark => spark(painter, center, diameter, tint),
        Kind::Smoke => smoke(painter, center, diameter, tint),
    }
}

fn ring(painter: &Painter, center: Pos2, diameter: f32, tint: Color32) {
    painter.circle_stroke(
        center,
        diameter * 0.40,
        Stroke::new(1.3_f32, tint.gamma_multiply(0.75)),
    );
    painter.circle_stroke(
        center,
        diameter * 0.48,
        Stroke::new(2.2_f32, tint.gamma_multiply(0.18)),
    );
}

fn spark(painter: &Painter, center: Pos2, diameter: f32, tint: Color32) {
    let radius = diameter * 0.5;
    soft_disc(painter, center, radius * 0.35, tint.gamma_multiply(0.55));
    for index in 0..6 {
        let angle = TAU * index as f32 / 6.0 + 0.22;
        let direction = Vec2::angled(angle);
        let length = radius * if index % 2 == 0 { 0.90 } else { 0.56 };
        painter.line_segment(
            [
                center + direction * radius * 0.13,
                center + direction * length,
            ],
            Stroke::new(1.1_f32, tint.gamma_multiply(0.8)),
        );
    }
}

fn smoke(painter: &Painter, center: Pos2, diameter: f32, tint: Color32) {
    let radius = diameter * 0.5;
    let gray = Color32::from_rgba_unmultiplied(78, 83, 92, tint.a());
    for (offset, size, opacity) in [
        (Vec2::new(-0.25, 0.10), 0.45, 0.38),
        (Vec2::new(0.18, -0.17), 0.52, 0.32),
        (Vec2::new(0.07, 0.25), 0.40, 0.25),
    ] {
        soft_disc(
            painter,
            center + offset * radius,
            size * radius,
            gray.gamma_multiply(opacity),
        );
    }
}

/// Soft radial fan: opaque at the center, transparent at its edge.
fn soft_disc(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for index in 0..=20 {
        let angle = TAU * index as f32 / 20.0;
        mesh.colored_vertex(center + Vec2::angled(angle) * radius, Color32::TRANSPARENT);
        if index > 0 {
            mesh.add_triangle(0, index, index + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}
