//! Map markers show each ship model as a small pre-rendered image. A model's
//! marker view never changes, so its few thousand sorted triangles are
//! rasterised once per egui context instead of being re-sent every frame for
//! every trader on the map (which made the galaxy map stutter while zooming).
use super::model::ShipModel;
use egui::{Color32, ColorImage, Mesh, Painter, Pos2, Rect, TextureHandle, Vec2};

/// How finely a sprite is rasterised: (edge in texels, supersamples per
/// texel along each axis, averaged for smooth edges).
#[derive(Clone, Copy, Hash)]
enum Detail {
    /// Map markers, about 22 points (44 pixels on a 2× display).
    Marker,
    /// Event-card illustrations, up to about 120 points.
    Portrait,
}

impl Detail {
    fn texels_and_samples(self) -> (usize, usize) {
        match self {
            Self::Marker => (64, 4),
            Self::Portrait => (256, 2),
        }
    }
}

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
        self.paint_sprite(painter, Detail::Marker, at, size, false, Color32::WHITE);
    }

    /// The same view at illustration size, optionally mirrored left-to-right
    /// and multiplied by `tint` (premultiplied: dims and fades).
    pub fn paint_portrait(
        &self,
        painter: &Painter,
        at: Pos2,
        size: f32,
        mirrored: bool,
        tint: Color32,
    ) {
        self.paint_sprite(painter, Detail::Portrait, at, size, mirrored, tint);
    }

    fn paint_sprite(
        &self,
        painter: &Painter,
        detail: Detail,
        at: Pos2,
        size: f32,
        mirrored: bool,
        tint: Color32,
    ) {
        let sprite = self.sprite(painter.ctx(), detail);
        let (left, right) = if mirrored { (1.0, 0.0) } else { (0.0, 1.0) };
        painter.image(
            sprite.texture.id(),
            Rect::from_center_size(at, Vec2::splat(2.0 * sprite.half_extent * size)),
            Rect::from_min_max(Pos2::new(left, 0.0), Pos2::new(right, 1.0)),
            tint,
        );
    }

    fn sprite(&self, ctx: &egui::Context, detail: Detail) -> Sprite {
        // Models are process-wide statics (`model::for_kind`), so the address
        // identifies one for as long as the context lives.
        let key = egui::Id::new(("ship-sprite", self as *const Self as usize, detail));
        if let Some(cached) = ctx.data_mut(|data| data.get_temp::<Sprite>(key)) {
            return cached;
        }
        let mesh = self.marker_mesh();
        let half_extent = half_extent(&mesh);
        let image = rasterise(&mesh, half_extent, detail);
        let sprite = Sprite {
            texture: ctx.load_texture("ship-sprite", image, egui::TextureOptions::LINEAR),
            half_extent,
        };
        ctx.data_mut(|data| data.insert_temp(key, sprite.clone()));
        sprite
    }
}

/// The largest coordinate of `mesh` along either axis.
fn half_extent(mesh: &Mesh) -> f32 {
    mesh.vertices
        .iter()
        .map(|vertex| vertex.pos.x.abs().max(vertex.pos.y.abs()))
        .fold(f32::EPSILON, f32::max)
}

/// Paint `mesh` (already far-to-near, positions within ±`half_extent`) into
/// a premultiplied image, later triangles over earlier ones.
fn rasterise(mesh: &Mesh, half_extent: f32, detail: Detail) -> ColorImage {
    let (texels, samples_per_axis) = detail.texels_and_samples();
    let fine = texels * samples_per_axis;
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
    let mut image = ColorImage::new([texels, texels], Color32::TRANSPARENT);
    for y in 0..texels {
        for x in 0..texels {
            let mut sum = [0_u32; 4];
            for sy in 0..samples_per_axis {
                for sx in 0..samples_per_axis {
                    let sample =
                        samples[(y * samples_per_axis + sy) * fine + x * samples_per_axis + sx];
                    for (total, channel) in sum.iter_mut().zip(sample.to_array()) {
                        *total += u32::from(channel);
                    }
                }
            }
            let count = (samples_per_axis * samples_per_axis) as u32;
            let [r, g, b, a] = sum.map(|total| (total / count) as u8);
            image.pixels[y * texels + x] = Color32::from_rgba_premultiplied(r, g, b, a);
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
        let image = rasterise(&mesh, half_extent(&mesh), Detail::Marker);
        let texels = image.size[0];
        let centre = image.pixels[texels / 2 * texels + texels / 2];
        assert_eq!(centre.a(), 255);
        assert_eq!(image.pixels[0], Color32::TRANSPARENT);
    }
}
