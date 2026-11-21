//! Fixed-tilt camera over the hex board.
//!
//! Coordinate spaces:
//! - `Hex` (game-core): axial hexes on a single-layer board.
//! - [`ScenePoint`]: world units, z up, the board's centre hex at the origin
//!   on the board plane (z = 0). Hex centres are 1.0 apart: a hex sits at
//!   x = (√3 / 2) q, y = r + q / 2, so +x points from the player's edge toward
//!   the raiders'. Ships hover just above their hex at [`SHIP_HEIGHT`].
//! - Screen: egui points inside the view rect.
//!
//! The camera looks at the board from the player's side at a fixed tilt; it
//! only zooms and pans (flat play: no orbit). At zoom 1 it stands back just
//! far enough for the whole board, and the ships over it, to fit the
//! view rect, whatever its shape.
use egui::{Pos2, Rect, Vec2};
use game_core::combat::tactical::{Board, Hex};
use glam::{Mat4, Vec3};

/// Vertical field of view in radians.
const FOV: f32 = 0.75;
/// Fixed tilt above the board plane, in radians (about 55°).
const PITCH: f32 = 0.96;
/// Share of the view rect the board may fill at zoom 1; the rest is margin.
const FILL: f32 = 0.94;
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 4.0;
/// x spacing of hex columns, in world units (√3 / 2).
const COLUMN: f32 = 0.866_025_4;
/// A hex's circumradius in world units (centres are 1.0 apart).
pub const HEX_RADIUS: f32 = 0.577_350_3;
/// Height a ship hovers above the board plane, in world units.
pub const SHIP_HEIGHT: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePoint(pub Vec3);

impl ScenePoint {
    /// The centre of `hex` on the board plane.
    pub fn of_hex(hex: Hex) -> Self {
        let (q, r) = (f32::from(hex.q), f32::from(hex.r));
        Self(Vec3::new(COLUMN * q, r + q / 2.0, 0.0))
    }

    /// Where a ship on `hex` hovers.
    pub fn of(hex: Hex) -> Self {
        let mut point = Self::of_hex(hex).0;
        point.z = SHIP_HEIGHT;
        Self(point)
    }

    /// The same x/y on the board plane (a ship's shadow).
    pub fn on_board(self) -> Self {
        Self(Vec3::new(self.0.x, self.0.y, 0.0))
    }
}

/// Half the board's extent along x and y, in world units, hex corners included.
fn half_extent(board: Board) -> glam::Vec2 {
    let radius = f32::from(board.radius);
    glam::Vec2::new(COLUMN * radius + HEX_RADIUS, radius + 0.5)
}

/// The board's outline for framing: the corners of its six corner hexes, on
/// the board and at ship height.
fn outline(board: Board) -> Vec<Vec3> {
    let radius = board.radius;
    let corner_hexes = [
        (radius, 0),
        (radius, -radius),
        (0, -radius),
        (-radius, 0),
        (-radius, radius),
        (0, radius),
    ];
    let top = SHIP_HEIGHT;
    corner_hexes
        .into_iter()
        .flat_map(|(q, r)| {
            let centre = ScenePoint::of_hex(Hex::new(q, r)).0;
            (0..6).flat_map(move |corner| {
                let angle = corner as f32 * std::f32::consts::FRAC_PI_3;
                let point = centre + Vec3::new(angle.cos(), angle.sin(), 0.0) * HEX_RADIUS;
                [point, point + Vec3::Z * top]
            })
        })
        .collect()
}

/// The nearest distance from the board's centre at which its whole outline
/// fits in [`FILL`] of `rect`: the zoom-1 framing for any board and any view
/// shape. Found by bisection; the fit only improves with distance.
fn fit_distance(rect: Rect, board: Board) -> f32 {
    let points = outline(board);
    let inner = Rect::from_center_size(rect.center(), rect.size() * FILL);
    let centre = Vec3::new(0.0, 0.0, SHIP_HEIGHT);
    let fits = |distance| {
        let projector = look(centre, distance, rect);
        points.iter().all(|point| {
            projector
                .project(ScenePoint(*point))
                .is_some_and(|at| inner.contains(at.pos))
        })
    };
    let (mut near, mut far) = (1.0_f32, 400.0_f32);
    for _ in 0..24 {
        let middle = (near + far) / 2.0;
        if fits(middle) {
            far = middle;
        } else {
            near = middle;
        }
    }
    far
}

/// A projection looking at `focus` from `distance` away, from the player's
/// side at the fixed tilt.
fn look(focus: Vec3, distance: f32, rect: Rect) -> Projector {
    let eye = focus + Vec3::new(0.0, -PITCH.cos(), PITCH.sin()) * distance;
    Projector::looking_at(eye, focus, rect)
}

/// Scene-to-screen mapping for one view rect and camera pose.
pub struct Projector {
    matrix: Mat4,
    rect: Rect,
    /// Camera position in scene space (for back-face culling).
    pub eye: Vec3,
}

impl Projector {
    /// A perspective camera at `eye` looking at `focus`, z up, drawing into
    /// `rect`. Scene points farther than 400 units are clipped.
    pub fn looking_at(eye: Vec3, focus: Vec3, rect: Rect) -> Self {
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        Self {
            matrix: Mat4::perspective_rh(FOV, aspect, 0.1, 400.0)
                * Mat4::look_at_rh(eye, focus, Vec3::Z),
            rect,
            eye,
        }
    }

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
pub struct BoardCamera {
    /// 1 fits the whole board; larger moves closer.
    pub zoom: f32,
    /// Focus offset from the board's centre on the board plane, in world units.
    pub pan: glam::Vec2,
}

impl Default for BoardCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: glam::Vec2::ZERO,
        }
    }
}

impl BoardCamera {
    /// The point the camera looks at: the pan offset, at ship height.
    pub fn focus(&self) -> Vec3 {
        self.pan.extend(SHIP_HEIGHT)
    }

    /// World units from the focus to the eye (see [`fit_distance`]).
    fn distance(&self, rect: Rect, board: Board) -> f32 {
        fit_distance(rect, board) / self.zoom
    }

    /// A projection fixed for one frame, for projecting many points.
    pub fn projector(&self, rect: Rect, board: Board) -> Projector {
        look(self.focus(), self.distance(rect, board), rect)
    }

    /// One-off projection (tests); frames use [`Self::projector`].
    #[cfg(test)]
    pub fn project(&self, rect: Rect, board: Board, point: ScenePoint) -> Option<Projected> {
        self.projector(rect, board).project(point)
    }

    /// `factor` > 1 moves closer (egui's pinch convention).
    pub fn zoom(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Pans by a screen drag: the board under the pointer follows it.
    pub fn drag_pan(&mut self, delta: Vec2, rect: Rect, board: Board) {
        let per_point =
            2.0 * self.distance(rect, board) * (FOV / 2.0).tan() / rect.height().max(1.0);
        // Seen at a tilt, a vertical drag covers more board than a horizontal one.
        let forward_scale = 1.0 / PITCH.sin();
        self.pan += glam::Vec2::new(-delta.x, delta.y * forward_scale) * per_point;
        self.clamp_pan(board);
    }

    /// Moves the focus by world units: `forward` away from the viewer, `right`.
    pub fn nudge(&mut self, forward: f32, right: f32, board: Board) {
        self.pan += glam::Vec2::new(right, forward);
        self.clamp_pan(board);
    }

    /// Centres the view on `point` (kept over the board).
    pub fn look_at(&mut self, point: ScenePoint, board: Board) {
        self.pan = point.0.truncate();
        self.clamp_pan(board);
    }

    /// The focus never leaves the board.
    fn clamp_pan(&mut self, board: Board) {
        let half = half_extent(board);
        self.pan = self.pan.clamp(-half, half);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat::Side;

    fn screens() -> [Rect; 2] {
        [
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0)),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(370.0, 420.0)),
        ]
    }

    #[test]
    fn every_board_fits_wide_and_phone_views_with_the_player_left() {
        let camera = BoardCamera::default();
        for rect in screens() {
            for ships in [2, 4, 16] {
                let board = Board::for_ships(ships);
                let at = |position| camera.project(rect, board, position).unwrap().pos;
                let player = Hex::new(board.home_column(Side::Player), 0);
                let raider = Hex::new(board.home_column(Side::Hostile), 0);
                assert!(at(ScenePoint::of_hex(player)).x < at(ScenePoint::of_hex(raider)).x);
                let centre = Hex::new(0, 0);
                let ship = at(ScenePoint::of(centre));
                let hex = at(ScenePoint::of_hex(centre));
                assert!(ship.y < hex.y, "ships hover above their hex");
                // Every hex, and a ship on it, stays on screen.
                for hex in board.hexes() {
                    assert!(
                        rect.contains(at(ScenePoint::of_hex(hex))),
                        "{ships}: {hex:?}"
                    );
                    let ship = ScenePoint::of(hex);
                    assert!(rect.contains(at(ship)), "{ships}: ship {hex:?}");
                }
            }
        }
    }

    #[test]
    fn panning_moves_the_focus_and_stays_over_the_board() {
        let board = Board::for_ships(14);
        let rect = screens()[0];
        let mut camera = BoardCamera::default();
        let corner = ScenePoint::of(Hex::new(-6, 0));
        camera.look_at(corner, board);
        let at = camera.project(rect, board, corner).unwrap().pos;
        assert!(
            (at - rect.center()).length() < 1.0,
            "focused hex is centred"
        );
        // Dragging right moves the board right with the pointer.
        let mut camera = BoardCamera::default();
        let middle = ScenePoint(camera.focus());
        let before = camera.project(rect, board, middle).unwrap().pos;
        camera.drag_pan(Vec2::new(40.0, 0.0), rect, board);
        let after = camera.project(rect, board, middle).unwrap().pos;
        assert!(after.x > before.x + 20.0, "{before:?} -> {after:?}");
        camera.nudge(1_000.0, -1_000.0, board);
        assert!(camera.pan.abs().cmple(half_extent(board)).all());
    }

    #[test]
    fn zoom_stays_within_limits_and_never_shrinks_the_board_below_fitting() {
        let mut camera = BoardCamera::default();
        camera.zoom(100.0);
        assert_eq!(camera.zoom, MAX_ZOOM);
        camera.zoom(0.001);
        assert_eq!(camera.zoom, MIN_ZOOM);
    }
}
