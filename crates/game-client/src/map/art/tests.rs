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

#[test]
fn rocky_planets_use_biome_artwork() {
    use super::ArtKind;
    use game_core::planet::Biome;
    let mut body = game_core::LocalBody {
        id: game_core::BodyId(1),
        name: "Test".into(),
        kind: game_core::BodyKind::RockyPlanet,
        orbit: None,
        position: game_core::LocalPosition {
            x_km: 0,
            y_km: 0,
            z_km: 0,
        },
        planet: None,
    };
    assert_eq!(ArtKind::for_body(&body), ArtKind::RockyPlanet);
    for (biome, art) in [
        (Biome::Temperate, ArtKind::RockyPlanet),
        (Biome::Arid, ArtKind::AridPlanet),
        (Biome::Frozen, ArtKind::FrozenPlanet),
    ] {
        body.planet = Some(game_core::planet::PlanetProfile {
            economy: game_core::planet::EconomyType::Agricultural,
            specialization: game_core::economy::Commodity::Food,
            biome,
            danger: game_core::planet::Danger::Safe,
        });
        assert_eq!(ArtKind::for_body(&body), art);
    }
}
