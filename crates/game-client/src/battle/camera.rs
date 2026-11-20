//! Orbit camera for the battle volume.
//!
//! Coordinate spaces:
//! - `Cell` (game-core): integer grid cells.
//! - [`ScenePoint`]: world units, one cell = 1.0, z up, origin at the volume's
//!   centre; +x points from the player's edge toward the raiders' edge.
//! - Screen: egui points inside the view rect.
use egui::{Pos2, Rect, Vec2};
use game_core::combat::tactical::{Cell, DEPTH, HEIGHT, WIDTH};
use glam::{Mat4, Vec3};

/// Vertical field of view in radians.
const FOV: f32 = 0.75;
const MIN_PITCH: f32 = 0.12;
const MAX_PITCH: f32 = 1.45;
const MIN_DISTANCE: f32 = 9.0;
const MAX_DISTANCE: f32 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePoint(pub Vec3);

impl ScenePoint {
    pub fn of_cell(cell: Cell) -> Self {
        let centre = |value: i8, size: i8| f32::from(value) - f32::from(size - 1) / 2.0;
        Self(Vec3::new(
            centre(cell.x, WIDTH),
            centre(cell.y, DEPTH),
            centre(cell.z, HEIGHT),
        ))
    }

    /// The same x/y on the volume's floor (half a cell below the lowest layer).
    pub fn on_floor(self) -> Self {
        Self(Vec3::new(self.0.x, self.0.y, -f32::from(HEIGHT) / 2.0))
    }
}

/// Scene-to-screen mapping for one view rect and camera pose.
pub struct Projector {
    matrix: Mat4,
    rect: Rect,
    /// Camera position in scene space.
    pub eye: Vec3,
}

impl Projector {
    /// `None` for points behind the camera.
    pub fn project(&self, point: ScenePoint) -> Option<Projected> {
        let clip = self.matrix * point.0.extend(1.0);
        if clip.w <= 0.1 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let half = self.rect.size() / 2.0;
        Some(Projected {
            pos: Pos2::new(
                self.rect.center().x + ndc.x * half.x,
                self.rect.center().y - ndc.y * half.y,
            ),
            depth: clip.w,
            scale: half.y / (clip.w * (FOV / 2.0).tan()),
        })
    }
}

/// A scene point on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projected {
    pub pos: Pos2,
    /// Distance along the view axis; larger is farther.
    pub depth: f32,
    /// Screen points per world unit at this depth.
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    /// Radians around the vertical axis; 0 looks along +y with the player on the left.
    pub yaw: f32,
    /// Radians above the horizon.
    pub pitch: f32,
    /// World units from the volume's centre.
    pub distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: -0.3,
            pitch: 0.62,
            distance: 13.5,
        }
    }
}

impl OrbitCamera {
    fn eye(&self) -> Vec3 {
        let flat = self.distance * self.pitch.cos();
        Vec3::new(
            flat * self.yaw.sin(),
            -flat * self.yaw.cos(),
            self.distance * self.pitch.sin(),
        )
    }

    fn view_projection(&self, rect: Rect) -> Mat4 {
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        Mat4::perspective_rh(FOV, aspect, 0.1, 200.0)
            * Mat4::look_at_rh(self.eye(), Vec3::ZERO, Vec3::Z)
    }

    /// One-off projection (tests); frames use [`Self::projector`].
    #[cfg(test)]
    pub fn project(&self, rect: Rect, point: ScenePoint) -> Option<Projected> {
        self.projector(rect).project(point)
    }

    /// A projection fixed for one frame, for projecting many points.
    pub fn projector(&self, rect: Rect) -> Projector {
        Projector {
            matrix: self.view_projection(rect),
            rect,
            eye: self.eye(),
        }
    }

    /// Drag orbits: horizontal turns around the volume, vertical tilts.
    pub fn drag(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.008;
        self.pitch = (self.pitch + delta.y * 0.006).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// `factor` > 1 moves closer (egui's pinch convention).
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance / factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_view_puts_the_player_edge_left_and_upper_layers_higher() {
        let camera = OrbitCamera::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let at = |x, z| {
            camera
                .project(rect, ScenePoint::of_cell(Cell { x, y: 4, z }))
                .unwrap()
                .pos
        };
        assert!(at(0, 2).x < at(WIDTH - 1, 2).x);
        assert!(at(4, HEIGHT - 1).y < at(4, 0).y);
        let centre = camera.project(rect, ScenePoint(Vec3::ZERO)).unwrap().pos;
        assert!((centre - rect.center()).length() < 1.0);
    }

    #[test]
    fn orbit_and_zoom_stay_within_limits() {
        let mut camera = OrbitCamera::default();
        camera.drag(Vec2::new(0.0, 10_000.0));
        assert_eq!(camera.pitch, MAX_PITCH);
        camera.zoom(100.0);
        assert_eq!(camera.distance, MIN_DISTANCE);
        camera.zoom(0.001);
        assert_eq!(camera.distance, MAX_DISTANCE);
    }
}
