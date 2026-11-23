//! Kenney's transparent source sprites, tinted at draw time for combat hits.
use egui::{Color32, Painter, Pos2, Rect, TextureHandle, Vec2};

#[derive(Clone, Copy)]
pub enum Kind {
    Ring,
    Spark,
    Smoke,
}

#[derive(Clone)]
pub struct Particles {
    ring: TextureHandle,
    spark: TextureHandle,
    smoke: TextureHandle,
}

impl Particles {
    pub fn preload(ctx: &egui::Context) -> Option<Self> {
        let key = egui::Id::new("battle-particles");
        if let Some(cached) = ctx.data(|data| data.get_temp::<Self>(key)) {
            return Some(cached);
        }
        let load = |name, bytes: &[u8]| {
            let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            );
            Some(ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR))
        };
        let particles = Self {
            ring: load(
                "battle-ring",
                include_bytes!("../../../../assets/effects/particles/circle_03.png"),
            )?,
            spark: load(
                "battle-spark",
                include_bytes!("../../../../assets/effects/particles/spark_01.png"),
            )?,
            smoke: load(
                "battle-smoke",
                include_bytes!("../../../../assets/effects/particles/smoke_03.png"),
            )?,
        };
        ctx.data_mut(|data| data.insert_temp(key, particles.clone()));
        Some(particles)
    }

    pub fn paint(&self, painter: &Painter, kind: Kind, center: Pos2, diameter: f32, tint: Color32) {
        let texture = match kind {
            Kind::Ring => &self.ring,
            Kind::Spark => &self.spark,
            Kind::Smoke => &self.smoke,
        };
        painter.image(
            texture.id(),
            Rect::from_center_size(center, Vec2::splat(diameter)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_sprites_have_transparent_and_visible_pixels() {
        for bytes in [
            include_bytes!("../../../../assets/effects/particles/circle_03.png").as_slice(),
            include_bytes!("../../../../assets/effects/particles/spark_01.png").as_slice(),
            include_bytes!("../../../../assets/effects/particles/smoke_03.png").as_slice(),
        ] {
            let image = image::load_from_memory(bytes).unwrap().into_rgba8();
            assert!(image.pixels().any(|pixel| pixel[3] == 0));
            assert!(image.pixels().any(|pixel| pixel[3] > 0));
        }
    }
}
