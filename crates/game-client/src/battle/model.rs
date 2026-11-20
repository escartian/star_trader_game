//! Low-poly ship models for the battle view, exported from the Blender sources by
//! `tools/art/export_battle_models.py`. They are drawn flat-shaded as egui
//! triangles, far faces first; a hull whose model fails to load keeps its sprite.
//!
//! Model space is Blender's: +Z up, nose along +Y, in Blender units. Loading
//! recentres each model on its bounding box.
use super::camera::{Projector, ScenePoint};
use egui::{Color32, Mesh, Painter, Pos2, Rgba, Shape, Vec2 as ScreenVec2};
use game_core::fleet::ShipKind;
use glam::{Mat3, Mat4, Vec2, Vec3};
use std::sync::OnceLock;

/// Light direction in scene space (toward the light): above, behind the player's edge.
const LIGHT: Vec3 = Vec3::new(-0.45, -0.35, 0.82);
/// Share of a face's colour it keeps when facing away from the light.
const AMBIENT: f32 = 0.32;

struct Triangle {
    corners: [Vec3; 3],
    normal: Vec3,
    /// Linear base colour.
    color: Rgba,
    /// Glows (engine nozzles): drawn at full brightness regardless of light.
    emissive: bool,
}

pub struct ShipModel {
    triangles: Vec<Triangle>,
    /// Longest bounding-box edge in model units.
    length: f32,
}

/// The embedded model for a hull, if it has one that loaded.
pub fn for_kind(kind: ShipKind) -> Option<&'static ShipModel> {
    static FREIGHTER: OnceLock<Option<ShipModel>> = OnceLock::new();
    static PATROL: OnceLock<Option<ShipModel>> = OnceLock::new();
    static SCOUT: OnceLock<Option<ShipModel>> = OnceLock::new();
    static PROSPECTOR: OnceLock<Option<ShipModel>> = OnceLock::new();
    let (cell, bytes): (_, &[u8]) = match kind {
        ShipKind::Freighter => (
            &FREIGHTER,
            include_bytes!("../../../../assets/models/freighter.glb"),
        ),
        ShipKind::Patrol => (
            &PATROL,
            include_bytes!("../../../../assets/models/patrol.glb"),
        ),
        ShipKind::Scout => (
            &SCOUT,
            include_bytes!("../../../../assets/models/scout.glb"),
        ),
        ShipKind::Prospector => (
            &PROSPECTOR,
            include_bytes!("../../../../assets/models/prospector.glb"),
        ),
    };
    cell.get_or_init(|| ShipModel::load(bytes).ok()).as_ref()
}

impl ShipModel {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let gltf = gltf::Gltf::from_slice(bytes).map_err(|error| error.to_string())?;
        let blob = gltf
            .blob
            .as_deref()
            .ok_or("The model has no binary chunk")?;
        let scene = gltf
            .default_scene()
            .or_else(|| gltf.scenes().next())
            .ok_or("The model has no scene")?;
        let mut triangles = Vec::new();
        for node in scene.nodes() {
            collect(&node, Mat4::IDENTITY, blob, &mut triangles)?;
        }
        let (low, high) = triangles.iter().flat_map(|triangle| triangle.corners).fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(low, high), corner| (low.min(corner), high.max(corner)),
        );
        if triangles.is_empty() || triangles.len() > 20_000 {
            return Err("The model has no triangles or too many".into());
        }
        let centre = (low + high) / 2.0;
        for triangle in &mut triangles {
            for corner in &mut triangle.corners {
                *corner -= centre;
            }
        }
        Ok(Self {
            triangles,
            length: (high - low).max_element().max(f32::EPSILON),
        })
    }

    /// Draws the model centred on `at`, `size` world units long, with its nose
    /// turned from Blender's +Y toward the horizontal `heading`.
    pub fn paint(
        &self,
        painter: &Painter,
        projector: &Projector,
        at: ScenePoint,
        size: f32,
        heading: Vec2,
    ) {
        let turn = -heading.x.atan2(heading.y);
        let rotation = Mat3::from_rotation_z(turn);
        let scale = size / self.length;
        let light = LIGHT.normalize();
        let mut faces = Vec::with_capacity(self.triangles.len());
        for triangle in &self.triangles {
            let corners = triangle
                .corners
                .map(|corner| at.0 + rotation * (corner * scale));
            let normal = rotation * triangle.normal;
            let centroid = (corners[0] + corners[1] + corners[2]) / 3.0;
            if normal.dot(projector.eye - centroid) <= 0.0 {
                continue;
            }
            let projected = corners.map(|corner| projector.project(ScenePoint(corner)));
            let [Some(a), Some(b), Some(c)] = projected else {
                continue;
            };
            let shade = if triangle.emissive {
                1.0
            } else {
                AMBIENT + (1.0 - AMBIENT) * normal.dot(light).max(0.0)
            };
            let color = Color32::from(Rgba::from_rgb(
                triangle.color.r() * shade,
                triangle.color.g() * shade,
                triangle.color.b() * shade,
            ));
            faces.push((a.depth + b.depth + c.depth, [a.pos, b.pos, c.pos], color));
        }
        faces.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut mesh = Mesh::default();
        for (_, points, color) in faces {
            let base = mesh.vertices.len() as u32;
            for point in points {
                mesh.colored_vertex(point, color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(Shape::mesh(mesh));
    }

    /// A tiny orthographic view of the same GLB for moving map traffic. The
    /// marker follows the trader's projected position; mesh depth is sorted
    /// locally, while the map remains responsible for world positioning.
    pub fn paint_marker(&self, painter: &Painter, at: Pos2, size: f32) {
        let eye = Vec3::new(0.8, -1.2, 2.5).normalize();
        let right = Vec3::Z.cross(eye).normalize();
        let up = eye.cross(right);
        let scale = size / self.length;
        let light = LIGHT.normalize();
        let mut faces = Vec::with_capacity(self.triangles.len());
        for triangle in &self.triangles {
            if triangle.normal.dot(eye) <= 0.0 {
                continue;
            }
            let centroid = triangle.corners.into_iter().sum::<Vec3>() / 3.0;
            let points = triangle
                .corners
                .map(|corner| at + ScreenVec2::new(corner.dot(right), -corner.dot(up)) * scale);
            let shade = if triangle.emissive {
                1.0
            } else {
                AMBIENT + (1.0 - AMBIENT) * triangle.normal.dot(light).max(0.0)
            };
            let color = Color32::from(Rgba::from_rgb(
                triangle.color.r() * shade,
                triangle.color.g() * shade,
                triangle.color.b() * shade,
            ));
            faces.push((-centroid.dot(eye), points, color));
        }
        faces.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut mesh = Mesh::default();
        for (_, points, color) in faces {
            let base = mesh.vertices.len() as u32;
            for point in points {
                mesh.colored_vertex(point, color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(Shape::mesh(mesh));
    }
}

fn collect(
    node: &gltf::Node,
    parent: Mat4,
    blob: &[u8],
    triangles: &mut Vec<Triangle>,
) -> Result<(), String> {
    let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|_| Some(blob));
            let positions: Vec<Vec3> = reader
                .read_positions()
                .ok_or("A model part has no positions")?
                .map(|position| transform.transform_point3(Vec3::from(position)))
                .collect();
            let indices: Vec<usize> = match reader.read_indices() {
                Some(indices) => indices.into_u32().map(|index| index as usize).collect(),
                None => (0..positions.len()).collect(),
            };
            // Scanned hulls carry linear per-vertex colours; others are white here.
            let colors: Vec<[f32; 4]> = match reader.read_colors(0) {
                Some(colors) => colors.into_rgba_f32().collect(),
                None => vec![[1.0; 4]; positions.len()],
            };
            if colors.len() != positions.len() {
                return Err("A model part has the wrong number of vertex colours".into());
            }
            let material = primitive.material();
            let factor = material.pbr_metallic_roughness().base_color_factor();
            let emissive = material
                .emissive_factor()
                .iter()
                .any(|channel| *channel > 0.0);
            for corner in indices.as_chunks::<3>().0 {
                let corners = [0, 1, 2].map(|i| positions.get(corner[i]).copied());
                let [Some(a), Some(b), Some(c)] = corners else {
                    return Err("A model triangle refers to a missing vertex".into());
                };
                let normal = (b - a).cross(c - a).normalize_or_zero();
                if normal != Vec3::ZERO {
                    // Flat shading: one colour per face, the mean of its corners.
                    let channel = |index: usize| {
                        let sum: f32 = corner.iter().map(|&vertex| colors[vertex][index]).sum();
                        factor[index] * sum / 3.0
                    };
                    triangles.push(Triangle {
                        corners: [a, b, c],
                        normal,
                        color: Rgba::from_rgb(channel(0), channel(1), channel(2)),
                        emissive,
                    });
                }
            }
        }
    }
    for child in node.children() {
        collect(&child, transform, blob, triangles)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_models_load_centred_with_coloured_parts() {
        for kind in ShipKind::ALL {
            check(for_kind(kind).unwrap_or_else(|| panic!("{kind:?} model loads")));
        }
        assert!(ShipModel::load(b"not a model").is_err());
        let scout = for_kind(ShipKind::Scout).unwrap();
        assert!(
            scout.triangles.iter().any(|face| {
                face.color.g() > face.color.r() * 1.3 && face.color.b() > face.color.r() * 1.2
            }),
            "the scout has teal paint, not only a grey scan"
        );
    }

    fn check(model: &ShipModel) {
        // Every face is sorted on the CPU each frame; 7,000 is the per-hull budget.
        assert!((500..7_000).contains(&model.triangles.len()));
        let colors: std::collections::HashSet<_> = model
            .triangles
            .iter()
            .map(|triangle| Color32::from(triangle.color))
            .collect();
        assert!(colors.len() >= 4, "several painted parts");
        let (low, high) = model
            .triangles
            .iter()
            .flat_map(|triangle| triangle.corners)
            .fold((Vec3::MAX, Vec3::MIN), |(low, high), corner| {
                (low.min(corner), high.max(corner))
            });
        assert!(((low + high) / 2.0).length() < 1e-3);
        // Nose along +Y: the model is longer than it is wide.
        assert_eq!((high - low).max_element(), high.y - low.y);
    }

    #[test]
    fn scout_mesh_follows_its_battle_hex() {
        use super::super::camera::BoardCamera;
        use game_core::combat::tactical::Board;

        let model = for_kind(ShipKind::Scout).unwrap();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 560.0));
        let camera = BoardCamera::default();
        let projector = camera.projector(rect, Board::for_ships(2));
        let painted_centre = |point: Vec3| {
            let ctx = egui::Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        egui::Id::new("scout-position-test"),
                    ));
                    model.paint(&painter, &projector, ScenePoint(point), 1.6, Vec2::X);
                },
            );
            let positions: Vec<_> = output
                .shapes
                .into_iter()
                .filter_map(|shape| match shape.shape {
                    Shape::Mesh(mesh) => Some(mesh.vertices.clone()),
                    _ => None,
                })
                .flatten()
                .map(|vertex| vertex.pos)
                .collect();
            assert!(!positions.is_empty());
            positions.iter().map(|pos| pos.x).sum::<f32>() / positions.len() as f32
        };
        let left = painted_centre(Vec3::new(-2.0, 0.0, 1.0));
        let right = painted_centre(Vec3::new(2.0, 0.0, 1.0));
        assert!(right > left + 20.0, "the scout follows its new hex");
    }

    #[test]
    fn scout_hull_is_level_along_its_bow_axis() {
        let model = for_kind(ShipKind::Scout).unwrap();
        let points: Vec<_> = model
            .triangles
            .iter()
            .flat_map(|face| face.corners)
            .collect();
        let mean = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        let (covariance, variance) = points.iter().fold((0.0, 0.0), |(cov, var), point| {
            let y = point.y - mean.y;
            (cov + y * (point.z - mean.z), var + y * y)
        });
        assert!(
            (covariance / variance).abs() < 0.12,
            "scout points nose-down"
        );
    }
}
