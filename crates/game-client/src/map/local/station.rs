//! Blender-authored map art. PNG decoding is isolated from map behavior.
use egui::{Color32, Painter, Pos2, Rect, TextureHandle, Vec2};

const PNG: &[u8] = include_bytes!("../../../../../assets/sprites/trade-port.png");

#[derive(Default)]
pub(super) struct StationArt {
    texture: Option<TextureHandle>,
    error: Option<String>,
}

impl StationArt {
    pub(super) fn prepare(&mut self, ctx: &egui::Context) {
        if self.texture.is_some() || self.error.is_some() {
            return;
        }
        match image::load_from_memory(PNG) {
            Ok(image) => {
                let rgba = image.into_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.texture =
                    Some(ctx.load_texture("trade-port", pixels, egui::TextureOptions::LINEAR));
            }
            Err(error) => self.error = Some(format!("Trade-port artwork could not load: {error}")),
        }
    }

    pub(super) fn paint(&self, painter: &Painter, center: Pos2) {
        if let Some(texture) = &self.texture {
            painter.image(
                texture.id(),
                Rect::from_center_size(center, Vec2::splat(88.0)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            // Preserve a recognizable destination if artwork fails to decode.
            painter.circle_stroke(
                center,
                10.0,
                egui::Stroke::new(2.0_f32, Color32::LIGHT_GREEN),
            );
        }
    }

    pub(super) fn inspector(&self, ui: &mut egui::Ui) {
        if let Some(texture) = &self.texture {
            ui.horizontal_wrapped(|ui| {
                ui.image((texture.id(), Vec2::splat(160.0)));
                ui.label("Trade port\nHabitat ring • cargo bays • solar arrays\nCyan docking guidance / amber approach beacons");
            });
        }
        if let Some(error) = &self.error {
            ui.label(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_station_sprite_decodes_with_transparent_margin() {
        let rgba = image::load_from_memory(PNG).unwrap().into_rgba8();
        assert_eq!(rgba.dimensions(), (512, 512));
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        let visible = rgba.pixels().filter(|pixel| pixel[3] > 128).count();
        assert!(visible > 10_000 && visible < 512 * 512 / 2);
    }
}
