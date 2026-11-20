//! Drawing helpers that place scene-space geometry on screen for one frame.
use super::camera::{Projector, ScenePoint};
use egui::{Painter, Pos2, Shape, Stroke};
use glam::Vec3;

pub struct Canvas<'a> {
    pub painter: &'a Painter,
    pub projector: &'a Projector,
}

impl Canvas<'_> {
    pub fn at(&self, point: ScenePoint) -> Option<Pos2> {
        self.projector.project(point).map(|at| at.pos)
    }

    pub fn line(&self, a: ScenePoint, b: ScenePoint, stroke: Stroke) {
        if let (Some(a), Some(b)) = (self.at(a), self.at(b)) {
            self.painter.line_segment([a, b], stroke);
        }
    }

    /// A line broken into dashes `dash` world units long.
    pub fn dashed(&self, a: ScenePoint, b: ScenePoint, dash: f32, stroke: Stroke) {
        let length = a.0.distance(b.0);
        let steps = ((length / dash) as usize).max(1);
        for step in (0..steps).step_by(2) {
            let from = a.0.lerp(b.0, step as f32 / steps as f32);
            let to = a.0.lerp(b.0, ((step + 1) as f32 / steps as f32).min(1.0));
            self.line(ScenePoint(from), ScenePoint(to), stroke);
        }
    }

    /// A horizontal ring of `radius` world units around `centre`, drawn in
    /// perspective so it reads as lying level with the ship.
    pub fn ring(&self, centre: ScenePoint, radius: f32, stroke: Stroke) {
        let points: Option<Vec<Pos2>> = (0..=32)
            .map(|step| {
                let angle = step as f32 / 32.0 * std::f32::consts::TAU;
                let offset = Vec3::new(angle.cos(), angle.sin(), 0.0) * radius;
                self.at(ScenePoint(centre.0 + offset))
            })
            .collect();
        if let Some(points) = points {
            self.painter.add(Shape::line(points, stroke));
        }
    }
}
