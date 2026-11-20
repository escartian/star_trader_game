use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use game_core::StarKind;

pub fn appearance(kind: StarKind) -> (Color32, f32) {
    match kind {
        StarKind::YellowDwarf => (Color32::from_rgb(255, 208, 112), 5.5),
        StarKind::RedDwarf => (Color32::from_rgb(255, 108, 78), 3.8),
        StarKind::BlueGiant => (Color32::from_rgb(113, 180, 255), 8.0),
        StarKind::RedGiant => (Color32::from_rgb(255, 143, 72), 10.0),
        StarKind::WhiteDwarf => (Color32::from_rgb(217, 235, 255), 3.0),
        StarKind::OrangeDwarf => (Color32::from_rgb(255, 170, 81), 5.3),
        StarKind::BlueWhiteStar => (Color32::from_rgb(171, 223, 255), 6.4),
        StarKind::BrownDwarf => (Color32::from_rgb(174, 97, 64), 3.5),
        StarKind::NeutronStar => (Color32::from_rgb(174, 255, 244), 3.3),
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
    // Each coordinate is an independent, well-mixed hash, so no rows or lattices.
    for i in 0..300_u32 {
        let x = unit(i, 1);
        let y = unit(i, 2);
        let glint = unit(i, 3);
        let brightness = 55 + (95.0 * glint * glint) as u8;
        let tint = unit(i, 4);
        let color = if tint < 0.15 {
            Color32::from_rgb(
                brightness.saturating_add(25),
                brightness,
                brightness / 2 + 20,
            )
        } else {
            Color32::from_rgb(brightness, brightness, brightness.saturating_add(25))
        };
        let radius = if glint > 0.93 {
            1.1
        } else {
            0.4 + 0.3 * unit(i, 5)
        };
        painter.circle_filled(rect.min + rect.size() * Vec2::new(x, y), radius, color);
    }
    painter.text(
        rect.right_bottom() - Vec2::splat(10.0),
        egui::Align2::RIGHT_BOTTOM,
        caption,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(100, 125, 150),
    );
}

/// A stable pseudo-random value in [0, 1) for star `index` and channel `salt`.
/// Integer avalanche hash (lowbias32): neighbouring inputs give unrelated outputs.
fn unit(index: u32, salt: u32) -> f32 {
    let mut value = index.wrapping_mul(0x9e37_79b9) ^ salt.wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    (value >> 8) as f32 / (1 << 24) as f32
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
    let halo = if kind == StarKind::NeutronStar {
        7.0
    } else {
        5.0
    };
    let strength = if kind == StarKind::BrownDwarf {
        0.1
    } else {
        0.2
    };
    glow(
        painter,
        center,
        Vec2::splat(radius * halo),
        color.gamma_multiply(strength),
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
    if kind != StarKind::BrownDwarf {
        painter.circle_filled(center, radius * 0.28, Color32::from_rgb(255, 249, 232));
    }
    if kind == StarKind::NeutronStar {
        painter.circle_stroke(
            center,
            radius * 1.7,
            Stroke::new(0.8_f32, color.gamma_multiply(0.6)),
        );
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
