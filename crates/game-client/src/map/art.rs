//! Shared, preloaded bitmap artwork. Simulation never consumes rendering assets.
use egui::{Color32, Painter, Pos2, Rect, TextureHandle, Vec2};
use game_core::{BodyKind, StarKind};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtKind {
    YellowDwarf,
    RedDwarf,
    BlueGiant,
    RedGiant,
    WhiteDwarf,
    RockyPlanet,
    GasGiant,
    Moon,
    Asteroid,
    Station,
}

impl From<StarKind> for ArtKind {
    fn from(kind: StarKind) -> Self {
        match kind {
            StarKind::YellowDwarf => Self::YellowDwarf,
            StarKind::RedDwarf => Self::RedDwarf,
            StarKind::BlueGiant => Self::BlueGiant,
            StarKind::RedGiant => Self::RedGiant,
            StarKind::WhiteDwarf => Self::WhiteDwarf,
        }
    }
}

impl From<BodyKind> for ArtKind {
    fn from(kind: BodyKind) -> Self {
        match kind {
            BodyKind::RockyPlanet => Self::RockyPlanet,
            BodyKind::GasGiant => Self::GasGiant,
            BodyKind::Moon => Self::Moon,
            BodyKind::Asteroid => Self::Asteroid,
            BodyKind::Station => Self::Station,
        }
    }
}

const SOURCES: &[(ArtKind, &str, &[u8])] = &[
    (
        ArtKind::YellowDwarf,
        "yellow-dwarf",
        include_bytes!("../../../../assets/sprites/yellow-dwarf.png"),
    ),
    (
        ArtKind::RedDwarf,
        "red-dwarf",
        include_bytes!("../../../../assets/sprites/red-dwarf.png"),
    ),
    (
        ArtKind::BlueGiant,
        "blue-giant",
        include_bytes!("../../../../assets/sprites/blue-giant.png"),
    ),
    (
        ArtKind::RedGiant,
        "red-giant",
        include_bytes!("../../../../assets/sprites/red-giant.png"),
    ),
    (
        ArtKind::WhiteDwarf,
        "white-dwarf",
        include_bytes!("../../../../assets/sprites/white-dwarf.png"),
    ),
    (
        ArtKind::RockyPlanet,
        "rocky-planet",
        include_bytes!("../../../../assets/sprites/rocky-planet.png"),
    ),
    (
        ArtKind::GasGiant,
        "gas-giant",
        include_bytes!("../../../../assets/sprites/gas-giant.png"),
    ),
    (
        ArtKind::Moon,
        "moon",
        include_bytes!("../../../../assets/sprites/moon.png"),
    ),
    (
        ArtKind::Asteroid,
        "asteroid",
        include_bytes!("../../../../assets/sprites/asteroid.png"),
    ),
    (
        ArtKind::Station,
        "trade-port",
        include_bytes!("../../../../assets/sprites/trade-port.png"),
    ),
];

#[derive(Clone)]
pub struct Artwork {
    textures: BTreeMap<ArtKind, TextureHandle>,
    errors: Vec<String>,
}

impl Artwork {
    /// Cache once per egui context, and share handles across every system view.
    pub fn preload(ctx: &egui::Context) -> Self {
        let cache_key = egui::Id::new("celestial-artwork");
        if let Some(cached) = ctx.data_mut(|data| data.get_temp::<Self>(cache_key)) {
            return cached;
        }
        let mut artwork = Self {
            textures: BTreeMap::new(),
            errors: Vec::new(),
        };
        for &(kind, name, bytes) in SOURCES {
            match image::load_from_memory(bytes) {
                Ok(image) => {
                    let rgba = image.into_rgba8();
                    let pixels = egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    );
                    artwork.textures.insert(
                        kind,
                        ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR),
                    );
                }
                Err(error) => artwork
                    .errors
                    .push(format!("Artwork {name} could not load: {error}")),
            }
        }
        ctx.data_mut(|data| data.insert_temp(cache_key, artwork.clone()));
        artwork
    }

    /// Returns false when a procedural fallback should be drawn.
    pub fn paint(&self, kind: ArtKind, painter: &Painter, center: Pos2, size: f32) -> bool {
        let Some(texture) = self.textures.get(&kind) else {
            return false;
        };
        painter.image(
            texture.id(),
            Rect::from_center_size(center, Vec2::splat(size)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        true
    }

    pub fn preview(&self, kind: ArtKind, ui: &mut egui::Ui) {
        if let Some(texture) = self.textures.get(&kind) {
            ui.image((texture.id(), Vec2::splat(160.0)));
        }
    }

    pub fn warnings(&self, ui: &mut egui::Ui) {
        for error in &self.errors {
            ui.label(error);
        }
    }
}

#[cfg(test)]
mod tests;
