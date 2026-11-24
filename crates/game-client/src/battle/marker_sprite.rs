//! Map markers show each ship model as a small pre-rendered image. A model's
//! marker view never changes, so its few thousand sorted triangles are
//! rasterised once per egui context instead of being re-sent every frame for
//! every trader on the map (which made the galaxy map stutter while zooming).
use super::model::ShipModel;
use egui::{Color32, ColorImage, Mesh, Painter, Pos2, Rect, TextureHandle, Vec2};

/// Sprite edge in texels: comfortably above a marker's on-screen size
/// (about 22 points, so 44 pixels on a 2× display).
const TEXELS: usize = 64;
/// Supersamples per texel along each axis, averaged for smooth edges.
const SAMPLES: usize = 4;

#[derive(Clone)]
struct Sprite {
    texture: TextureHandle,
    /// Half the sprite's square edge in marker units (1 = model length).
    half_extent: f32,
}

impl ShipModel {
    /// Draw the marker view `size` points long (the model's longest edge),
    /// centred on `at`.
    pub fn paint_marker(&self, painter: &Painter, at: Pos2, size: f32) {
        let sprite = self.sprite(painter.ctx());
        painter.image(
            sprite.texture.id(),
            Rect::from_center_size(at, Vec2::splat(2.0 * sprite.half_extent * size)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn sprite(&self, ctx: &egui::Context) -> Sprite {
        // Models are process-wide statics (`model::for_kind`), so the address
        // identifies one for as long as the context lives.
        let key = egui::Id::new(("ship-marker-sprite", self as *const Self as usize));
        if let Some(cached) = ctx.data_mut(|data| data.get_temp::<Sprite>(key)) {
            return cached;
        }
        let mesh = self.marker_mesh();
        let half_extent = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x.abs().max(vertex.pos.y.abs()))
            .fold(f32::EPSILON, f32::max);
        let image = rasterise(&mesh, half_extent);
        let sprite = Sprite {
            texture: ctx.load_texture("ship-marker", image, egui::TextureOptions::LINEAR),
            half_extent,
        };
        ctx.data_mut(|data| data.insert_temp(key, sprite.clone()));
        sprite
    }
}

/// Paint `mesh` (already far-to-near, positions within ±`half_extent`) into
/// a premultiplied image, later triangles over earlier ones.
fn rasterise(mesh: &Mesh, half_extent: f32) -> ColorImage {
    let fine = TEXELS * SAMPLES;
    let to_fine = |pos: Pos2| {
        let scale = fine as f32 / (2.0 * half_extent);
        Pos2::new((pos.x + half_extent) * scale, (pos.y + half_extent) * scale)
    };
    let mut samples = vec![Color32::TRANSPARENT; fine * fine];
    for corners in mesh.indices.as_chunks::<3>().0 {
        let vertex = |index: u32| &mesh.vertices[index as usize];
        let points = corners.map(|index| to_fine(vertex(index).pos));
        fill_triangle(&mut samples, fine, points, vertex(corners[0]).color);
    }
    let mut image = ColorImage::new([TEXELS, TEXELS], Color32::TRANSPARENT);
    for y in 0..TEXELS {
        for x in 0..TEXELS {
            let mut sum = [0_u32; 4];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let sample = samples[(y * SAMPLES + sy) * fine + x * SAMPLES + sx];
                    for (total, channel) in sum.iter_mut().zip(sample.to_array()) {
                        *total += u32::from(channel);
                    }
                }
            }
            let count = (SAMPLES * SAMPLES) as u32;
            let [r, g, b, a] = sum.map(|total| (total / count) as u8);
            image.pixels[y * TEXELS + x] = Color32::from_rgba_premultiplied(r, g, b, a);
        }
    }
    image
}

/// Fill the samples whose centres lie inside the triangle, either winding.
fn fill_triangle(samples: &mut [Color32], edge: usize, [a, b, c]: [Pos2; 3], color: Color32) {
    let cross = |p: Pos2, q: Pos2, r: Pos2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let area = cross(a, b, c);
    if area.abs() < f32::EPSILON {
        return;
    }
    let span = |values: [f32; 3]| {
        let low = values.iter().copied().fold(f32::INFINITY, f32::min);
        let high = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let low = low.floor().max(0.0) as usize;
        let high = (high.ceil().max(0.0) as usize).min(edge);
        low..high
    };
    for y in span([a.y, b.y, c.y]) {
        for x in span([a.x, b.x, c.x]) {
            let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
            let inside = [cross(a, b, p), cross(b, c, p), cross(c, a, p)]
                .iter()
                .all(|side| side * area.signum() >= 0.0);
            if inside {
                samples[y * edge + x] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_sprite_is_opaque_at_its_centre_and_clear_at_its_corners() {
        let model = super::super::model::for_kind(game_core::fleet::ShipKind::Freighter)
            .expect("the freighter model loads");
        let mesh = model.marker_mesh();
        let half_extent = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x.abs().max(vertex.pos.y.abs()))
            .fold(f32::EPSILON, f32::max);
        let image = rasterise(&mesh, half_extent);
        let centre = image.pixels[TEXELS / 2 * TEXELS + TEXELS / 2];
        assert_eq!(centre.a(), 255);
        assert_eq!(image.pixels[0], Color32::TRANSPARENT);
    }
}
