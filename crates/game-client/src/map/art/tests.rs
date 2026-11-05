use super::{Artwork, SOURCES};

#[test]
fn all_embedded_art_decodes_with_transparency_and_is_reused_across_views() {
    for &(_, name, bytes) in SOURCES {
        let rgba = image::load_from_memory(bytes).unwrap().into_rgba8();
        assert!(rgba.width() >= 256, "{name}");
        assert_eq!(rgba.get_pixel(0, 0)[3], 0, "{name}");
        assert!(
            rgba.pixels().filter(|pixel| pixel[3] > 128).count() > 1000,
            "{name}"
        );
    }
    let ctx = egui::Context::default();
    let initial = Artwork::preload(&ctx);
    let cached = Artwork::preload(&ctx);
    assert!(initial.errors.is_empty());
    assert_eq!(initial.textures.len(), SOURCES.len());
    for (kind, texture) in &initial.textures {
        assert_eq!(texture.id(), cached.textures[kind].id());
    }
}
