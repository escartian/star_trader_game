//! Small code-drawn portraits for the species chooser.
use egui::{Color32, Pos2, Shape, Stroke, Ui, Vec2};
use game_core::species::Species;

pub(super) fn show(ui: &mut Ui, species: Species) {
    let size = 68.0_f32;
    let scale = size / 42.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let center = rect.center();
    let at = |x: f32, y: f32| center + Vec2::new(x * scale, y * scale);
    let painter = ui.painter();
    painter.circle_filled(center, 20.0 * scale, Color32::from_rgb(14, 24, 39));
    match species {
        Species::Human => {
            ellipse(
                painter,
                at(0.0, -4.0),
                10.0 * scale,
                12.0 * scale,
                Color32::from_rgb(197, 159, 126),
            );
            ellipse(
                painter,
                at(0.0, 19.0),
                16.0 * scale,
                11.0 * scale,
                Color32::from_rgb(68, 111, 156),
            );
            painter.line_segment(
                [at(-4.0, -4.0), at(4.0, -4.0)],
                Stroke::new(scale, Color32::from_rgb(61, 47, 43)),
            );
        }
        Species::Miri => {
            ellipse(
                painter,
                at(0.0, 1.0),
                13.0 * scale,
                15.0 * scale,
                Color32::from_rgb(114, 210, 94),
            );
            ellipse(
                painter,
                at(-5.0, -2.0),
                3.3 * scale,
                4.8 * scale,
                Color32::from_rgb(19, 49, 39),
            );
            ellipse(
                painter,
                at(5.0, -2.0),
                3.3 * scale,
                4.8 * scale,
                Color32::from_rgb(19, 49, 39),
            );
            painter.line_segment(
                [at(-3.0, 8.0), at(3.0, 8.0)],
                Stroke::new(1.2 * scale, Color32::from_rgb(39, 111, 63)),
            );
            for x in [-9.0, 9.0] {
                painter.line_segment(
                    [at(x, -12.0), at(x * 1.2, -17.0)],
                    Stroke::new(1.4 * scale, Color32::from_rgb(114, 210, 94)),
                );
                painter.circle_filled(
                    at(x * 1.2, -17.0),
                    1.7 * scale,
                    Color32::from_rgb(180, 247, 134),
                );
            }
        }
        Species::Auralith => {
            let aura = Color32::from_rgb(73, 159, 237);
            let light = Color32::from_rgb(174, 237, 255);
            painter.circle_stroke(center, 17.0 * scale, Stroke::new(1.2 * scale, aura));
            painter.circle_stroke(center, 12.0 * scale, Stroke::new(scale, aura));
            painter.add(Shape::convex_polygon(
                vec![at(0.0, -15.0), at(9.0, 0.0), at(0.0, 15.0), at(-9.0, 0.0)],
                aura,
                Stroke::new(scale, light),
            ));
            painter.circle_filled(center, 4.5 * scale, light);
            for (x, y) in [(-15.0, -7.0), (15.0, -7.0), (-15.0, 7.0), (15.0, 7.0)] {
                painter.circle_filled(at(x, y), 1.7 * scale, light);
            }
        }
    }
}

fn ellipse(painter: &egui::Painter, center: Pos2, rx: f32, ry: f32, fill: Color32) {
    let points = (0..24)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / 24.0;
            center + Vec2::new(rx * angle.cos(), ry * angle.sin())
        })
        .collect();
    painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
}
