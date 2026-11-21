//! Battle terrain on the board: debris as a tinted hex strewn with flat
//! wreckage, asteroids as raised rocks. Rock outlines are jittered from the hex
//! itself, so a rock keeps its shape from frame to frame.
use super::backdrop;
use super::camera::{HEX_RADIUS, ScenePoint};
use super::canvas::Canvas;
use egui::{Color32, Pos2, Shape, Stroke};
use game_core::combat::tactical::{Hex, Terrain, TerrainHex};
use glam::Vec3;

/// Height of an asteroid's top above the board, in world units.
pub const ROCK_HEIGHT: f32 = 0.55;
const ROCK_TOP: Color32 = Color32::from_rgb(118, 108, 98);
const ROCK_SIDE: Color32 = Color32::from_rgb(64, 58, 54);
const DEBRIS: Color32 = Color32::from_rgb(196, 160, 96);

/// A small stable hash of a hex and an index, in 0..1.
fn jitter(hex: Hex, index: u32) -> f32 {
    // (q, r) packed into distinct bits, so mirrored hexes differ too.
    let key = (u32::from(hex.q as u8) << 8) | u32::from(hex.r as u8);
    let mut value = key.wrapping_mul(0x9e37_79b1) ^ index.wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2c1b_3c6d);
    value ^= value >> 12;
    (value % 1000) as f32 / 1000.0
}

/// Debris hexes, painted flat on the board under everything else.
pub fn paint_debris(canvas: &Canvas, terrain: &[TerrainHex]) {
    for cell in terrain.iter().filter(|cell| cell.kind == Terrain::Debris) {
        backdrop::hex(
            canvas,
            cell.hex,
            0.08,
            DEBRIS.gamma_multiply(0.1),
            Stroke::new(1.0_f32, DEBRIS.gamma_multiply(0.45)),
        );
        let centre = ScenePoint::of_hex(cell.hex).0;
        for shard in 0..7 {
            let angle = jitter(cell.hex, shard) * std::f32::consts::TAU;
            let reach = 0.12 + jitter(cell.hex, shard + 20) * 0.3;
            let at = centre + Vec3::new(angle.cos(), angle.sin(), 0.0) * reach;
            let size = 0.05 + jitter(cell.hex, shard + 40) * 0.06;
            let turn = jitter(cell.hex, shard + 60) * std::f32::consts::TAU;
            let points: Option<Vec<Pos2>> = (0..3)
                .map(|corner| {
                    let a = turn + corner as f32 * 2.1;
                    let offset = Vec3::new(a.cos(), a.sin(), 0.02) * size;
                    canvas.at(ScenePoint(at + offset))
                })
                .collect();
            if let Some(points) = points {
                canvas.painter.add(Shape::convex_polygon(
                    points,
                    DEBRIS.gamma_multiply(0.75),
                    Stroke::NONE,
                ));
            }
        }
    }
}

/// The scene point a rock is depth-sorted by: the middle of its top.
pub fn rock_anchor(hex: Hex) -> ScenePoint {
    let mut point = ScenePoint::of_hex(hex).0;
    point.z = ROCK_HEIGHT / 2.0;
    ScenePoint(point)
}

/// One asteroid: a jagged outline lifted into a prism, sides darker than the
/// top. Sides are drawn back to front so near faces cover far ones.
pub fn paint_rock(canvas: &Canvas, hex: Hex) {
    const CORNERS: u32 = 9;
    let centre = ScenePoint::of_hex(hex).0;
    let outline: Vec<Vec3> = (0..CORNERS)
        .map(|corner| {
            let angle = (corner as f32 + jitter(hex, corner) * 0.5) / CORNERS as f32
                * std::f32::consts::TAU;
            let radius = HEX_RADIUS * (0.62 + jitter(hex, corner + 10) * 0.3);
            centre + Vec3::new(angle.cos(), angle.sin(), 0.0) * radius
        })
        .collect();
    let height = |corner: usize| ROCK_HEIGHT * (0.8 + jitter(hex, corner as u32 + 30) * 0.4);
    let top: Vec<Vec3> = outline
        .iter()
        .enumerate()
        .map(|(corner, point)| *point + Vec3::Z * height(corner))
        .collect();
    let mut sides: Vec<(f32, [Vec3; 4])> = (0..outline.len())
        .filter_map(|corner| {
            let next = (corner + 1) % outline.len();
            let quad = [outline[corner], outline[next], top[next], top[corner]];
            let middle = (quad[0] + quad[1] + quad[2] + quad[3]) / 4.0;
            // Only sides facing the camera: one seen edge-on projects to a
            // sliver, which egui's anti-aliasing draws as a long spike.
            let outward = (middle - centre).with_z(0.0);
            if outward.dot(canvas.projector.eye - middle) <= 0.0 {
                return None;
            }
            let depth = canvas.projector.project(ScenePoint(middle))?.depth;
            Some((depth, quad))
        })
        .collect();
    sides.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, quad) in sides {
        let points: Option<Vec<Pos2>> = quad.iter().map(|p| canvas.at(ScenePoint(*p))).collect();
        if let Some(points) = points.filter(|points| area(points) > 2.0) {
            canvas.painter.add(Shape::convex_polygon(
                points,
                ROCK_SIDE,
                Stroke::new(0.8_f32, Color32::from_black_alpha(120)),
            ));
        }
    }
    let points: Option<Vec<Pos2>> = top.iter().map(|p| canvas.at(ScenePoint(*p))).collect();
    let peak = top.iter().copied().sum::<Vec3>() / top.len() as f32;
    if let (Some(points), Some(peak)) = (points, canvas.at(ScenePoint(peak))) {
        // A jagged top is not always convex, and egui fills only convex
        // shapes cleanly: fill it as a fan of triangles from its middle.
        for (corner, point) in points.iter().enumerate() {
            let next = points[(corner + 1) % points.len()];
            if area(&[peak, *point, next]) <= 0.5 {
                continue;
            }
            canvas.painter.add(Shape::convex_polygon(
                vec![peak, *point, next],
                ROCK_TOP,
                Stroke::new(0.6_f32, ROCK_TOP),
            ));
        }
        canvas.painter.add(Shape::closed_line(
            points,
            Stroke::new(1.0_f32, Color32::from_rgb(150, 140, 128)),
        ));
    }
}

/// Unsigned screen area of a polygon, in square points.
fn area(points: &[Pos2]) -> f32 {
    let twice: f32 = (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    twice.abs() / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_shapes_are_stable_and_vary_between_hexes() {
        let a = Hex::new(1, -1);
        assert_eq!(jitter(a, 3), jitter(a, 3));
        assert!((0..9).any(|corner| jitter(a, corner) != jitter(Hex::new(-1, 1), corner)));
        assert!((0..50).all(|i| (0.0..1.0).contains(&jitter(a, i))));
    }
}
