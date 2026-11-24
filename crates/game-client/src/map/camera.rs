use egui::{Pos2, Rect, Vec2};
use glam::{DQuat, DVec3};

#[derive(Debug, PartialEq)]
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
            // Both maps use one north-up plane, seen at a fixed shallow tilt.
            yaw: 0.0,
            pitch: 1.05,
            zoom: 1.0,
            pan: Vec2::ZERO,
            focus: DVec3::ZERO,
        }
    }
}

/// Wheel/two-finger scroll and trackpad pinch both zoom the hovered map.
/// The map never sits inside a scroll area, so scrolling cannot also move the page.
/// `zoom_delta` is egui's multiplicative pinch (and ctrl+wheel) factor; 1.0 = none.
/// egui already eases mouse-wheel steps over several frames (`smooth_scroll_delta`).
pub fn zoom_input(ui: &egui::Ui, zoom: f32, minimum: f32) -> f32 {
    let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
    (zoom * (scroll * 0.002).exp() * pinch).clamp(minimum, 4.0)
}

/// Fit a large galaxy's farthest star at startup; smaller fixtures stay at 1×.
pub fn overview_zoom(systems: &[game_core::StarSystem]) -> f32 {
    if systems.len() <= 10 {
        return 1.0;
    }
    let radius = systems
        .iter()
        .map(|system| {
            let x = system.position.x as f64;
            let z = system.position.z as f64;
            x.hypot(z)
        })
        .fold(1.0_f64, f64::max);
    (17.0 / radius).clamp(0.15, 1.0) as f32
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

    /// Apply this frame's zoom input while the pointer hovers the map, keeping
    /// the point under the pointer fixed on screen so zooming homes in on it.
    pub fn zoom_toward_pointer(&mut self, ui: &egui::Ui, response: &egui::Response, minimum: f32) {
        let Some(anchor) = response.hover_pos() else {
            return;
        };
        let zoom = zoom_input(ui, self.zoom, minimum);
        self.zoom_about(zoom, anchor, response.rect);
    }

    /// Rescale to `zoom` around the screen point `anchor` inside `rect`.
    pub fn zoom_about(&mut self, zoom: f32, anchor: Pos2, rect: Rect) {
        if zoom == self.zoom {
            return;
        }
        // project() = centre + pan + offset; scaling the offset by `ratio`
        // and moving the pan by what it lost keeps `anchor` over one point.
        let ratio = zoom / self.zoom;
        let offset = anchor - rect.center() - self.pan;
        self.pan += offset * (1.0 - ratio);
        self.zoom = zoom;
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

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_still() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0));
        let mut camera = Camera {
            pan: Vec2::new(20.0, -10.0),
            ..Camera::default()
        };
        let world = DVec3::new(6.0, 0.0, -4.0);
        let anchor = camera.project(world, rect);
        camera.zoom_about(2.7, anchor, rect);
        assert!(camera.project(world, rect).distance(anchor) < 1e-3);
        assert_eq!(camera.zoom, 2.7);
    }

    #[test]
    fn large_map_starts_at_an_overview_scale() {
        let small = game_core::generation::generate_galaxy(0);
        assert_eq!(overview_zoom(&small), 1.0);
        let large = game_core::generation::generate_galaxy_with(
            42,
            game_core::generation::GalaxyConfig {
                systems: 150,
                radius_ly: 95,
                max_objects: 8,
            },
        )
        .unwrap();
        assert!(overview_zoom(&large) < 0.3);
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::zoom_input;

    fn zoom_after(events: Vec<egui::Event>) -> f32 {
        let ctx = egui::Context::default();
        let mut zoom = 1.0;
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| zoom = zoom_input(ui, zoom, 0.15));
            },
        );
        zoom
    }

    #[test]
    fn trackpad_pinch_zooms_and_is_clamped() {
        assert!((zoom_after(vec![egui::Event::Zoom(1.5)]) - 1.5).abs() < 1e-4);
        assert_eq!(zoom_after(vec![egui::Event::Zoom(100.0)]), 4.0);
        assert_eq!(zoom_after(vec![egui::Event::Zoom(0.001)]), 0.15);
        assert_eq!(zoom_after(Vec::new()), 1.0);
    }
}
