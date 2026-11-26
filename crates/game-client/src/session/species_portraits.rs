//! High-resolution portraits embedded for the species chooser.
use egui::{ColorImage, Context, Rect, TextureHandle, TextureOptions, Ui, Vec2};
use game_core::species::Species;

const HUMAN: &[u8] = include_bytes!("../../../../assets/portraits/human.png");
const MIRI: &[u8] = include_bytes!("../../../../assets/portraits/miri.png");
const AURALITH: &[u8] = include_bytes!("../../../../assets/portraits/auralith.png");

fn bytes(species: Species) -> &'static [u8] {
    match species {
        Species::Human => HUMAN,
        Species::Miri => MIRI,
        Species::Auralith => AURALITH,
    }
}

fn texture(ctx: &Context, species: Species) -> TextureHandle {
    let id = egui::Id::new(("species-portrait", species));
    if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return texture;
    }
    let rgba = image::load_from_memory(bytes(species))
        .expect("embedded species portrait must decode")
        .into_rgba8();
    let pixels = ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    );
    let texture = ctx.load_texture(
        format!("species-portrait-{}", species.label()),
        pixels,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

pub(super) fn show(ui: &mut Ui, species: Species, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let image = texture(ui.ctx(), species);
    ui.painter().image(
        image.id(),
        rect,
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_embedded_portrait_decodes_and_is_cached() {
        let context = Context::default();
        for species in Species::ALL {
            let rgba = image::load_from_memory(bytes(species))
                .unwrap()
                .into_rgba8();
            assert_eq!(rgba.dimensions(), (512, 512));
            let first = texture(&context, species);
            assert_eq!(first.size(), [512, 512]);
            assert_eq!(texture(&context, species).id(), first.id());
        }
    }
}
