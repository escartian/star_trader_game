//! Small code-drawn alien portraits for the factions panel.
use egui::{Color32, Pos2, Shape, Stroke, Ui, Vec2};
use game_core::factions::Faction;

pub(super) fn show(ui: &mut Ui, faction: Faction) {
    if !matches!(faction, Faction::Miri | Faction::Auralith) {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_filled(center, 20.0, Color32::from_rgb(14, 24, 39));
    match faction {
        Faction::Miri => {
            ellipse(
                painter,
                center + Vec2::new(0.0, 1.0),
                13.0,
                15.0,
                Color32::from_rgb(114, 210, 94),
            );
            ellipse(
                painter,
                center + Vec2::new(-5.0, -2.0),
                3.3,
                4.8,
                Color32::from_rgb(19, 49, 39),
            );
            ellipse(
                painter,
                center + Vec2::new(5.0, -2.0),
                3.3,
                4.8,
                Color32::from_rgb(19, 49, 39),
            );
            painter.line_segment(
                [center + Vec2::new(-3.0, 8.0), center + Vec2::new(3.0, 8.0)],
                Stroke::new(1.2_f32, Color32::from_rgb(39, 111, 63)),
            );
            for x in [-9.0, 9.0] {
                painter.line_segment(
                    [
                        center + Vec2::new(x, -12.0),
                        center + Vec2::new(x * 1.2, -17.0),
                    ],
                    Stroke::new(1.4_f32, Color32::from_rgb(114, 210, 94)),
                );
                painter.circle_filled(
                    center + Vec2::new(x * 1.2, -17.0),
                    1.7,
                    Color32::from_rgb(180, 247, 134),
                );
            }
        }
        Faction::Auralith => {
            let aura = Color32::from_rgb(73, 159, 237);
            let light = Color32::from_rgb(174, 237, 255);
            painter.circle_stroke(center, 17.0, Stroke::new(1.2_f32, aura));
            painter.circle_stroke(center, 12.0, Stroke::new(1.0_f32, aura));
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(0.0, -15.0),
                    center + Vec2::new(9.0, 0.0),
                    center + Vec2::new(0.0, 15.0),
                    center + Vec2::new(-9.0, 0.0),
                ],
                aura,
                Stroke::new(1.0_f32, light),
            ));
            painter.circle_filled(center, 4.5, light);
            for (x, y) in [(-15.0, -7.0), (15.0, -7.0), (-15.0, 7.0), (15.0, 7.0)] {
                painter.circle_filled(center + Vec2::new(x, y), 1.7, light);
            }
        }
        _ => {}
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
