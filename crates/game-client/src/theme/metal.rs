//! Polished gunmetal surfaces: a vertical tone ramp with a soft diagonal specular
//! sheen, chamfered top-left/bottom-right corners and bevelled edges. Static
//! (no animation), painted behind content so text and controls stay crisp.
use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Ui, Vec2};

/// Chamfer size in points for the top-left and bottom-right corners.
const CHAMFER: f32 = 10.0;
const COLUMNS: usize = 24;

/// Vertical tone stops: (fraction of height, rgb).
const TONES: [(f32, [f32; 3]); 4] = [
    (0.0, [92.0, 104.0, 108.0]),
    (0.07, [66.0, 77.0, 81.0]),
    (0.5, [41.0, 49.0, 53.0]),
    (1.0, [25.0, 31.0, 34.0]),
];

fn tone(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    for pair in TONES.windows(2) {
        let ((a, ca), (b, cb)) = (pair[0], pair[1]);
        if t <= b {
            let f = (t - a) / (b - a);
            return [0, 1, 2].map(|i| ca[i] + (cb[i] - ca[i]) * f);
        }
    }
    TONES[TONES.len() - 1].1
}

/// Specular boost for normalized (u, t); bands run diagonally like a light strip.
fn sheen(u: f32, t: f32) -> f32 {
    let s = u + 0.45 * t;
    let band = |center: f32, width: f32, gain: f32| gain * (-((s - center) / width).powi(2)).exp();
    band(0.32, 0.11, 30.0) + band(0.62, 0.07, 12.0)
}

/// Left/right x-extent at height y for the chamfered outline.
fn extent(rect: Rect, y: f32) -> (f32, f32) {
    let left = rect.left() + (CHAMFER - (y - rect.top())).max(0.0);
    let right = rect.right() - (CHAMFER - (rect.bottom() - y)).max(0.0);
    (left, right)
}

fn outline(rect: Rect) -> [Pos2; 6] {
    [
        Pos2::new(rect.left() + CHAMFER, rect.top()),
        rect.right_top(),
        Pos2::new(rect.right(), rect.bottom() - CHAMFER),
        Pos2::new(rect.right() - CHAMFER, rect.bottom()),
        rect.left_bottom(),
        Pos2::new(rect.left(), rect.top() + CHAMFER),
    ]
}

/// Every shape for one metal plate; `alpha` < 255 lets the map glint through.
pub fn plate(rect: Rect, alpha: u8) -> Vec<Shape> {
    let mut rows: Vec<f32> = (0..=16).map(|i| i as f32 / 16.0).collect();
    // Chamfer breakpoints keep each strip's edges straight, so quads are exact.
    for y in [rect.top() + CHAMFER, rect.bottom() - CHAMFER] {
        rows.push(((y - rect.top()) / rect.height()).clamp(0.0, 1.0));
    }
    rows.sort_by(f32::total_cmp);
    rows.dedup();
    let mut mesh = Mesh::default();
    for &t in &rows {
        let y = rect.top() + t * rect.height();
        let (left, right) = extent(rect, y);
        let base = tone(t);
        for column in 0..=COLUMNS {
            let u = column as f32 / COLUMNS as f32;
            let x = left + (right - left) * u;
            let lift = sheen((x - rect.left()) / rect.width(), t);
            let [r, g, b] = base.map(|channel| (channel + lift).min(255.0) as u8);
            mesh.colored_vertex(
                Pos2::new(x, y),
                Color32::from_rgba_unmultiplied(r, g, b, alpha),
            );
        }
    }
    let stride = (COLUMNS + 1) as u32;
    for row in 0..rows.len() as u32 - 1 {
        for column in 0..COLUMNS as u32 {
            let a = row * stride + column;
            mesh.add_triangle(a, a + 1, a + stride);
            mesh.add_triangle(a + 1, a + stride + 1, a + stride);
        }
    }
    let points = outline(rect);
    let light = Stroke::new(1.0_f32, Color32::from_white_alpha(70));
    let dark = Stroke::new(1.0_f32, Color32::from_black_alpha(170));
    let mut shapes = vec![Shape::mesh(mesh)];
    // Bevel: lit along the top/left, shadowed along the right/bottom.
    for (index, stroke) in [
        (0, light),
        (1, dark),
        (2, dark),
        (3, dark),
        (4, light),
        (5, light),
    ] {
        let (a, b) = (points[index], points[(index + 1) % points.len()]);
        shapes.push(Shape::line_segment([a, b], stroke));
    }
    shapes.push(Shape::closed_line(
        outline(rect.expand(1.0)).to_vec(),
        Stroke::new(1.0_f32, Color32::from_black_alpha(120)),
    ));
    shapes.push(Shape::line_segment(
        [points[5], points[0]],
        Stroke::new(2.0_f32, super::GREEN),
    ));
    shapes.push(Shape::line_segment(
        [points[0], points[0] + Vec2::new(36.0, 0.0)],
        Stroke::new(1.5_f32, super::GREEN),
    ));
    shapes
}

/// Lay out content on a metal plate sized to fit it.
pub fn frame<R>(ui: &mut Ui, alpha: u8, add: impl FnOnce(&mut Ui) -> R) -> R {
    let slot = ui.painter().add(Shape::Noop);
    let inner = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, add);
    ui.painter()
        .set(slot, Shape::Vec(plate(inner.response.rect, alpha)));
    inner.inner
}

/// Below this screen width (points), panels open full screen like phone sheets.
pub const FULL_SCREEN_BELOW: f32 = 700.0;

/// A panel on a metal plate with a display-font title. On a wide screen it is a
/// centred window with a close glyph; on a narrow (phone) screen it fills the
/// whole screen with a Back button, like a full-screen sheet.
pub fn window(
    ctx: &egui::Context,
    title: &str,
    open: &mut bool,
    width: f32,
    add: impl FnOnce(&mut Ui),
) {
    if !*open {
        return;
    }
    let screen = ctx.screen_rect();
    let full = screen.width() < FULL_SCREEN_BELOW;
    // The plate adds 12 points of margin on each side.
    let width = if full { screen.width() - 24.0 } else { width };
    let mut close = false;
    let window = egui::Window::new(title)
        .title_bar(false)
        .frame(egui::Frame::NONE)
        .resizable(false);
    let window = if full {
        window.fixed_pos(screen.min).fixed_size(screen.size())
    } else {
        window
            .default_width(width)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen.center())
    };
    window.show(ctx, |ui| {
        frame(ui, 255, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                if full {
                    close = super::icons::button(ui, super::icons::Icon::Back, "Back").clicked();
                }
                ui.label(
                    egui::RichText::new(title.to_uppercase())
                        .font(super::fonts::display(ui.ctx(), 15.0))
                        .color(super::GREEN),
                );
                if !full {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = super::icons::icon_button(ui, super::icons::Icon::Cancel, "Close")
                            .clicked();
                    });
                }
            });
            ui.separator();
            if full {
                // Fill the screen (nothing behind shows through); wide content
                // such as the trade table scrolls sideways.
                let height = screen.height() - 70.0;
                ui.set_min_height(screen.height() - 20.0);
                egui::ScrollArea::both()
                    .max_height(height)
                    .min_scrolled_height(height)
                    .show(ui, add);
            } else {
                let height = screen.height() * 0.7;
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .min_scrolled_height(height)
                    .show(ui, add);
            }
        });
    });
    if close {
        *open = false;
    }
}
