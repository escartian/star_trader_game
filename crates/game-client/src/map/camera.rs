use egui::{Pos2, Rect, Vec2};
use glam::{DQuat, DVec3};

pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub zoom: f32,
    pub pan: Vec2,
    pub focus: DVec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.5,
            pitch: 0.5,
            zoom: 1.0,
            pan: Vec2::ZERO,
            focus: DVec3::ZERO,
        }
    }
}

impl Camera {
    pub fn rotation(&self) -> DQuat {
        DQuat::from_rotation_x(self.pitch) * DQuat::from_rotation_y(self.yaw)
    }

    pub fn project(&self, position: DVec3, rect: Rect) -> Pos2 {
        let p = self.rotation() * (position - self.focus);
        let scale = rect.width().min(rect.height()) / 38.0 * self.zoom;
        rect.center() + self.pan + Vec2::new(p.x as f32, -p.y as f32) * scale
    }

    pub fn focus_on(&mut self, position: DVec3) {
        self.focus = position;
        self.pan = Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_centers_target_at_every_orientation_and_zoom() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
        let target = DVec3::new(8.0, 4.0, 12.0);
        for (yaw, pitch, zoom) in [(0.0, 0.0, 1.0), (1.2, -0.8, 4.0), (-2.0, 1.5, 0.3)] {
            let mut camera = Camera {
                yaw,
                pitch,
                zoom,
                pan: Vec2::new(70.0, -90.0),
                ..Camera::default()
            };
            camera.focus_on(target);
            assert_eq!(camera.project(target, rect), rect.center());
        }
    }
}
