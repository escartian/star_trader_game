//! Original vector console symbols: resolution independent, with named controls.
use egui::{Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Fleets,
    Save,
    Navigation,
    Market,
    Execute,
    Back,
    Focus,
}

pub fn button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        FontId::monospace(12.0),
        ui.visuals().text_color(),
    );
    let size = Vec2::new(text.size().x + 42.0, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let visual = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        3.0,
        visual.weak_bg_fill,
        visual.bg_stroke,
        StrokeKind::Inside,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            3.0,
            Stroke::new(1.0_f32, super::GREEN),
            StrokeKind::Outside,
        );
    }
    ui.painter().line_segment(
        [
            rect.left_top() + Vec2::splat(1.0),
            rect.right_top() + Vec2::new(-1.0, 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
    );
    paint(
        ui.painter(),
        icon,
        rect.left_center() + Vec2::new(15.0, 0.0),
        super::GREEN,
    );
    ui.painter().galley(
        rect.left_center() + Vec2::new(29.0, -text.size().y * 0.5),
        text,
        ui.visuals().text_color(),
    );
    response.on_hover_text(label)
}

pub fn paint(painter: &Painter, icon: Icon, center: Pos2, color: Color32) {
    let p = |x: f32, y: f32| center + Vec2::new(x, y);
    let stroke = Stroke::new(1.4_f32, color);
    let line = |a, b| {
        painter.line_segment([a, b], stroke);
    };
    match icon {
        Icon::Fleets => {
            for x in [-5.0, 5.0] {
                painter.add(egui::Shape::closed_line(
                    vec![p(x, -6.0), p(x + 3.0, 5.0), p(x, 2.0), p(x - 3.0, 5.0)],
                    stroke,
                ));
            }
        }
        Icon::Save => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(14.0)),
                1.0,
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(-3.0, -6.0), p(3.0, -1.0)),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
            line(p(-4.0, 4.0), p(4.0, 4.0));
        }
        Icon::Navigation => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.add(egui::Shape::closed_line(
                vec![p(3.0, -5.0), p(1.0, 3.0), p(-4.0, 5.0), p(-1.0, -2.0)],
                stroke,
            ));
        }
        Icon::Market => {
            for x in [-5.0, 0.0, 5.0] {
                line(p(x, 6.0), p(x, -x - 1.0));
            }
            line(p(-8.0, 8.0), p(8.0, 8.0));
        }
        Icon::Execute => {
            for x in [-5.0, 2.0] {
                line(p(x, -6.0), p(x + 5.0, 0.0));
                line(p(x + 5.0, 0.0), p(x, 6.0));
            }
        }
        Icon::Back => {
            line(p(7.0, 0.0), p(-7.0, 0.0));
            line(p(-7.0, 0.0), p(-1.0, -6.0));
            line(p(-7.0, 0.0), p(-1.0, 6.0));
        }
        Icon::Focus => {
            for sign in [-1.0, 1.0] {
                line(p(sign * 8.0, -3.0), p(sign * 8.0, -8.0));
                line(p(sign * 8.0, -8.0), p(sign * 3.0, -8.0));
                line(p(sign * 8.0, 3.0), p(sign * 8.0, 8.0));
                line(p(sign * 8.0, 8.0), p(sign * 3.0, 8.0));
            }
            painter.circle_stroke(center, 2.0, stroke);
        }
    }
}
