//! Orbit camera for the battle volume.
//!
//! Coordinate spaces:
//! - `Cell` (game-core): integer grid cells inside the battle's `GridSize`.
//! - [`ScenePoint`]: world units, one cell = 1.0, z up, cell (0, 0, 0) at the
//!   origin; +x points from the player's edge toward the raiders' edge.
//! - Screen: egui points inside the view rect.
//!
//! The camera orbits a focus point (the grid's centre plus a pan offset) and
//! stands back in proportion to the grid's bounding radius, so every grid size
//! starts framed alike. Panning keeps the focus inside the grid.
use egui::{Pos2, Rect, Vec2};
use game_core::combat::tactical::{Cell, GridSize};
use glam::{Mat4, Vec3};

/// Vertical field of view in radians.
const FOV: f32 = 0.75;
const MIN_PITCH: f32 = 0.12;
const MAX_PITCH: f32 = 1.45;
/// Camera distance per world unit of the grid's bounding radius, at zoom 1;
/// keeps the whole floor in view for every grid shape.
const FRAMING: f32 = 2.5;
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePoint(pub Vec3);

impl ScenePoint {
    pub fn of_cell(cell: Cell) -> Self {
        Self(Vec3::new(
            f32::from(cell.x),
            f32::from(cell.y),
            f32::from(cell.z),
        ))
    }

    /// The same x/y on the volume's floor (half a cell below the lowest layer).
    pub fn on_floor(self) -> Self {
        Self(Vec3::new(self.0.x, self.0.y, -0.5))
    }
}

/// The scene point at the centre of a grid.
pub fn centre(size: GridSize) -> Vec3 {
    Vec3::new(
        f32::from(size.width - 1),
        f32::from(size.depth - 1),
        f32::from(size.height - 1),
    ) / 2.0
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
    /// Above 1 moves closer than the grid's framing distance.
    pub zoom: f32,
    /// Focus offset from the grid's centre, in world units.
    pub pan: Vec3,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: -0.3,
            pitch: 0.62,
            zoom: 1.0,
            pan: Vec3::ZERO,
        }
    }
}

impl OrbitCamera {
    /// World units from the focus to the eye.
    fn distance(&self, size: GridSize) -> f32 {
        let radius = Vec3::new(
            f32::from(size.width),
            f32::from(size.depth),
            f32::from(size.height),
        )
        .length()
            / 2.0;
        FRAMING * radius / self.zoom
    }

    /// The point the camera looks at and orbits.
    pub fn focus(&self, size: GridSize) -> Vec3 {
        centre(size) + self.pan
    }

    fn eye(&self, size: GridSize) -> Vec3 {
        let distance = self.distance(size);
        let flat = distance * self.pitch.cos();
        self.focus(size)
            + Vec3::new(
                flat * self.yaw.sin(),
                -flat * self.yaw.cos(),
                distance * self.pitch.sin(),
            )
    }

    /// Unit vectors on the floor plane: (toward the view, to its right).
    fn ground_axes(&self) -> (Vec3, Vec3) {
        let (sin, cos) = self.yaw.sin_cos();
        (Vec3::new(-sin, cos, 0.0), Vec3::new(cos, sin, 0.0))
    }

    /// One-off projection (tests); frames use [`Self::projector`].
    #[cfg(test)]
    pub fn project(&self, rect: Rect, size: GridSize, point: ScenePoint) -> Option<Projected> {
        self.projector(rect, size).project(point)
    }

    /// A projection fixed for one frame, for projecting many points.
    pub fn projector(&self, rect: Rect, size: GridSize) -> Projector {
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        let eye = self.eye(size);
        Projector {
            matrix: Mat4::perspective_rh(FOV, aspect, 0.1, 400.0)
                * Mat4::look_at_rh(eye, self.focus(size), Vec3::Z),
            rect,
            eye,
        }
    }

    /// Drag orbits: horizontal turns around the volume, vertical tilts.
    pub fn drag(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.008;
        self.pitch = (self.pitch + delta.y * 0.006).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// `factor` > 1 moves closer (egui's pinch convention).
    pub fn zoom(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Pans by a screen drag: the grid under the pointer follows it across the
    /// floor plane, at the focus's depth.
    pub fn drag_pan(&mut self, delta: Vec2, rect: Rect, size: GridSize) {
        let per_point = 2.0 * self.distance(size) * (FOV / 2.0).tan() / rect.height().max(1.0);
        let (forward, right) = self.ground_axes();
        // Seen from above, dragging down pulls far cells toward the viewer.
        let forward_scale = 1.0 / self.pitch.sin().max(0.2);
        self.pan += (-right * delta.x + forward * delta.y * forward_scale) * per_point;
        self.clamp_pan(size);
    }

    /// Moves the focus by cells: `forward` away from the viewer, `right`, `up`.
    pub fn nudge(&mut self, forward: f32, right: f32, up: f32, size: GridSize) {
        let (ahead, side) = self.ground_axes();
        self.pan += ahead * forward + side * right + Vec3::Z * up;
        self.clamp_pan(size);
    }

    /// Centres the view on `point` (kept inside the grid).
    pub fn look_at(&mut self, point: ScenePoint, size: GridSize) {
        self.pan = point.0 - centre(size);
        self.clamp_pan(size);
    }

    /// The focus never leaves the grid's volume.
    fn clamp_pan(&mut self, size: GridSize) {
        let half = centre(size) + Vec3::splat(0.5);
        self.pan = self.pan.clamp(-half, half);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_grid_size_is_framed_with_the_player_left_and_upper_layers_higher() {
        let camera = OrbitCamera::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        for size in [
            GridSize::default(),
            GridSize::for_ships(2),
            GridSize::for_ships(16),
        ] {
            let at = |x, z| {
                let cell = Cell {
                    x,
                    y: size.depth / 2,
                    z,
                };
                camera
                    .project(rect, size, ScenePoint::of_cell(cell))
                    .unwrap()
                    .pos
            };
            assert!(at(0, 2).x < at(size.width - 1, 2).x);
            assert!(at(size.width / 2, size.height - 1).y < at(size.width / 2, 0).y);
            let middle = camera
                .project(rect, size, ScenePoint(centre(size)))
                .unwrap()
                .pos;
            assert!((middle - rect.center()).length() < 1.0);
            // The far corners of the floor stay on screen.
            for (x, y) in [(0, 0), (size.width - 1, size.depth - 1)] {
                let corner = ScenePoint::of_cell(Cell { x, y, z: 0 }).on_floor();
                assert!(rect.contains(camera.project(rect, size, corner).unwrap().pos));
            }
        }
    }

    #[test]
    fn panning_moves_the_focus_and_stays_inside_the_grid() {
        let size = GridSize::for_ships(14);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let mut camera = OrbitCamera::default();
        let corner = ScenePoint::of_cell(Cell { x: 0, y: 0, z: 0 });
        camera.look_at(corner, size);
        let at = camera.project(rect, size, corner).unwrap().pos;
        assert!(
            (at - rect.center()).length() < 1.0,
            "focused cell is centred"
        );
        // Dragging right moves the grid right with the pointer.
        let mut camera = OrbitCamera::default();
        let middle = ScenePoint(centre(size));
        let before = camera.project(rect, size, middle).unwrap().pos;
        camera.drag_pan(Vec2::new(40.0, 0.0), rect, size);
        let after = camera.project(rect, size, middle).unwrap().pos;
        assert!(after.x > before.x + 20.0, "{before:?} -> {after:?}");
        camera.nudge(1_000.0, -1_000.0, 1_000.0, size);
        let limit = centre(size) + Vec3::splat(0.5);
        assert!(camera.pan.abs().cmple(limit).all());
        let mut fresh = OrbitCamera::default();
        fresh.drag_pan(Vec2::ZERO, rect, size);
        assert_eq!(fresh.pan, Vec3::ZERO);
    }

    #[test]
    fn orbit_and_zoom_stay_within_limits() {
        let mut camera = OrbitCamera::default();
        camera.drag(Vec2::new(0.0, 10_000.0));
        assert_eq!(camera.pitch, MAX_PITCH);
        camera.zoom(100.0);
        assert_eq!(camera.zoom, MAX_ZOOM);
        camera.zoom(0.001);
        assert_eq!(camera.zoom, MIN_ZOOM);
    }
}
