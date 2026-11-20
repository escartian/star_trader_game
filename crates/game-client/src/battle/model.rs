//! Low-poly ship models for the battle view, exported from the Blender sources by
//! `tools/art/export_battle_models.py`. They are drawn flat-shaded as egui
//! triangles, far faces first; hulls without a model keep their sprite.
//!
//! Model space is Blender's: +Z up, nose along +Y, in Blender units. Loading
//! recentres each model on its bounding box.
use super::camera::{Projector, ScenePoint};
use egui::{Color32, Mesh, Painter, Rgba, Shape};
use game_core::fleet::ShipKind;
use glam::{Mat3, Mat4, Vec3};
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
    match kind {
        ShipKind::Freighter => FREIGHTER
            .get_or_init(|| {
                ShipModel::load(include_bytes!("../../../../assets/models/freighter.glb")).ok()
            })
            .as_ref(),
        ShipKind::Scout | ShipKind::Patrol => None,
    }
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

    /// Draws the model centred on `at`, `size` world units long, nose turned to
    /// face +x (or −x when `facing_left`).
    pub fn paint(
        &self,
        painter: &Painter,
        projector: &Projector,
        at: ScenePoint,
        size: f32,
        facing_left: bool,
    ) {
        let turn = if facing_left {
            std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2
        };
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
            let material = primitive.material();
            let [red, green, blue, _] = material.pbr_metallic_roughness().base_color_factor();
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
                    triangles.push(Triangle {
                        corners: [a, b, c],
                        normal,
                        color: Rgba::from_rgb(red, green, blue),
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
    fn the_embedded_freighter_loads_centred_with_coloured_parts() {
        let model = for_kind(ShipKind::Freighter).expect("freighter model loads");
        assert!((500..5_000).contains(&model.triangles.len()));
        let colors: std::collections::HashSet<_> = model
            .triangles
            .iter()
            .map(|triangle| Color32::from(triangle.color))
            .collect();
        assert!(colors.len() >= 4, "hull, canopy, cargo pods and engines");
        assert!(model.triangles.iter().any(|triangle| triangle.emissive));
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
        assert!(ShipModel::load(b"not a model").is_err());
    }
}
