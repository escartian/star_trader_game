use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use game_core::StarKind;

pub fn appearance(kind: StarKind) -> (Color32, f32) {
    match kind {
        StarKind::YellowDwarf => (Color32::from_rgb(255, 208, 112), 5.5),
        StarKind::RedDwarf => (Color32::from_rgb(255, 108, 78), 3.8),
        StarKind::BlueGiant => (Color32::from_rgb(113, 180, 255), 8.0),
        StarKind::RedGiant => (Color32::from_rgb(255, 143, 72), 10.0),
        StarKind::WhiteDwarf => (Color32::from_rgb(217, 235, 255), 3.0),
    }
}

/// A transparent radial fan gives soft light without textures or a bloom pass.
fn glow(painter: &Painter, center: Pos2, radius: Vec2, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..=48 {
        let angle = i as f32 * std::f32::consts::TAU / 48.0;
        mesh.colored_vertex(
            center + Vec2::new(angle.cos(), angle.sin()) * radius,
            Color32::TRANSPARENT,
        );
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

pub fn background(painter: &Painter, rect: Rect, caption: &str) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(1, 2, 6));
    for (x, y, r, color) in [
        (
            0.18,
            0.70,
            0.9,
            Color32::from_rgba_unmultiplied(30, 50, 105, 45),
        ),
        (
            0.65,
            0.30,
            0.8,
            Color32::from_rgba_unmultiplied(62, 34, 95, 38),
        ),
        (
            0.88,
            0.65,
            0.6,
            Color32::from_rgba_unmultiplied(16, 75, 95, 30),
        ),
    ] {
        glow(
            painter,
            rect.min + rect.size() * Vec2::new(x, y),
            rect.size() * r,
            color,
        );
    }
    // Fixed decorative points: no simulation RNG, flicker, picking, or redraw timer.
    for i in 0..220_u32 {
        let hash = i.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
        let x = (hash & 0xffff) as f32 / 65535.0;
        let y = (hash.rotate_left(13) & 0xffff) as f32 / 65535.0;
        let brightness = 45 + (hash % 65) as u8;
        painter.circle_filled(
            rect.min + rect.size() * Vec2::new(x, y),
            if i % 11 == 0 { 1.0 } else { 0.55 },
            Color32::from_rgb(brightness, brightness, brightness.saturating_add(25)),
        );
    }
    painter.text(
        rect.right_bottom() - Vec2::splat(10.0),
        egui::Align2::RIGHT_BOTTOM,
        caption,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(100, 125, 150),
    );
}

pub fn star(painter: &Painter, center: Pos2, kind: StarKind, selected: bool, hovered: bool) {
    draw_star(painter, center, kind, appearance(kind).1, selected, hovered);
}

pub fn star_with_radius(painter: &Painter, center: Pos2, kind: StarKind, radius: f32) {
    draw_star(painter, center, kind, radius, false, false);
}

fn draw_star(
    painter: &Painter,
    center: Pos2,
    kind: StarKind,
    radius: f32,
    selected: bool,
    hovered: bool,
) {
    let color = appearance(kind).0;
    glow(
        painter,
        center,
        Vec2::splat(radius * 5.0),
        color.gamma_multiply(0.2),
    );
    glow(
        painter,
        center,
        Vec2::splat(radius * 2.4),
        color.gamma_multiply(0.65),
    );
    painter.circle_filled(center, radius, color);
    glow(
        painter,
        center,
        Vec2::splat(radius),
        Color32::from_white_alpha(210),
    );
    painter.circle_filled(center, radius * 0.28, Color32::from_rgb(255, 249, 232));
    if matches!(kind, StarKind::BlueGiant | StarKind::RedGiant) {
        for axis in [Vec2::X, Vec2::Y] {
            painter.line_segment(
                [center - axis * radius * 1.9, center + axis * radius * 1.9],
                Stroke::new(0.7_f32, color.gamma_multiply(0.5)),
            );
        }
    }
    if selected || hovered {
        painter.circle_stroke(
            center,
            radius + 7.0,
            Stroke::new(
                if selected { 1.5_f32 } else { 1.0_f32 },
                if selected {
                    Color32::from_rgb(130, 230, 226)
                } else {
                    Color32::from_white_alpha(130)
                },
            ),
        );
    }
}

/// Orientation stays visible even when the focused object has been panned away.
pub fn axes(painter: &Painter, rect: Rect, rotation: glam::DQuat) {
    // Bottom-right, clear of the floating run card at the bottom-left.
    let origin = rect.right_bottom() + Vec2::new(-40.0, -55.0);
    for (axis, label, color) in [
        (glam::DVec3::X, "X", Color32::LIGHT_RED),
        (glam::DVec3::Y, "Y", Color32::LIGHT_GREEN),
        (glam::DVec3::Z, "Z", Color32::LIGHT_BLUE),
    ] {
        let axis = rotation * axis * 23.0;
        let end = origin + Vec2::new(axis.x as f32, -axis.y as f32);
        painter.line_segment([origin, end], Stroke::new(1.0_f32, color));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            color,
        );
    }
}
