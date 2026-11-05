use super::LocalView;
use egui::{Color32, Painter, Rect, Stroke};
use glam::DVec3;

impl LocalView {
    pub(super) fn draw_boundary(&self, painter: &Painter, rect: Rect) {
        if !self.show_boundary {
            return;
        }
        let half = self.half_extent_km as f64 / super::KM_PER_VIEW_UNIT;
        let corners: Vec<_> = (0..8)
            .map(|index| {
                let component = |axis| {
                    if index & (1 << axis) == 0 {
                        -half
                    } else {
                        half
                    }
                };
                self.camera
                    .project(DVec3::new(component(0), component(1), component(2)), rect)
            })
            .collect();
        for index in 0..8 {
            for axis in 0..3 {
                let other = index ^ (1 << axis);
                if index < other {
                    painter.line_segment(
                        [corners[index], corners[other]],
                        Stroke::new(0.7_f32, Color32::from_rgba_unmultiplied(130, 200, 230, 70)),
                    );
                }
            }
        }
    }
}
