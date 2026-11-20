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
    OrangeDwarf,
    BlueWhiteStar,
    BrownDwarf,
    NeutronStar,
    RockyPlanet,
    AridPlanet,
    FrozenPlanet,
    OceanPlanet,
    JunglePlanet,
    VolcanicPlanet,
    BarrenPlanet,
    GasGiant,
    StormGiant,
    MethaneGiant,
    Moon,
    IceMoon,
    Asteroid,
    MetallicAsteroid,
    Station,
    Freighter,
    Scout,
    Patrol,
    Prospector,
}

impl From<StarKind> for ArtKind {
    fn from(kind: StarKind) -> Self {
        match kind {
            StarKind::YellowDwarf => Self::YellowDwarf,
            StarKind::RedDwarf => Self::RedDwarf,
            StarKind::BlueGiant => Self::BlueGiant,
            StarKind::RedGiant => Self::RedGiant,
            StarKind::WhiteDwarf => Self::WhiteDwarf,
            StarKind::OrangeDwarf => Self::OrangeDwarf,
            StarKind::BlueWhiteStar => Self::BlueWhiteStar,
            StarKind::BrownDwarf => Self::BrownDwarf,
            StarKind::NeutronStar => Self::NeutronStar,
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

impl ArtKind {
    /// Planet art follows biome or hazard; small bodies use stable visual variants.
    pub fn for_body(body: &game_core::LocalBody) -> Self {
        match (body.kind, body.planet.as_ref().map(|profile| profile.biome)) {
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Arid)) => Self::AridPlanet,
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Frozen)) => Self::FrozenPlanet,
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Ocean)) => Self::OceanPlanet,
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Jungle)) => Self::JunglePlanet,
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Volcanic)) => {
                Self::VolcanicPlanet
            }
            (BodyKind::RockyPlanet, Some(game_core::planet::Biome::Barren)) => Self::BarrenPlanet,
            (BodyKind::GasGiant, _)
                if body.planet.as_ref().is_some_and(|profile| {
                    profile.danger == game_core::planet::Danger::Insidious
                }) =>
            {
                Self::StormGiant
            }
            (BodyKind::GasGiant, _)
                if body.planet.as_ref().is_some_and(|profile| {
                    profile.danger == game_core::planet::Danger::Dangerous
                }) =>
            {
                Self::MethaneGiant
            }
            (BodyKind::Moon, _) if body.name.bytes().fold(0_u8, u8::wrapping_add) % 2 == 1 => {
                Self::IceMoon
            }
            (BodyKind::Asteroid, _) if body.id.0.is_multiple_of(3) => Self::MetallicAsteroid,
            (kind, _) => Self::from(kind),
        }
    }
}

impl From<game_core::fleet::ShipKind> for ArtKind {
    fn from(kind: game_core::fleet::ShipKind) -> Self {
        match kind {
            game_core::fleet::ShipKind::Freighter => Self::Freighter,
            game_core::fleet::ShipKind::Scout => Self::Scout,
            game_core::fleet::ShipKind::Patrol => Self::Patrol,
            game_core::fleet::ShipKind::Prospector => Self::Prospector,
        }
    }
}

const SOURCES: &[(ArtKind, &str, &[u8])] = &[
    (
        ArtKind::Freighter,
        "freighter",
        include_bytes!("../../../../assets/sprites/freighter.png"),
    ),
    (
        ArtKind::Scout,
        "scout",
        include_bytes!("../../../../assets/sprites/scout.png"),
    ),
    (
        ArtKind::Patrol,
        "patrol",
        include_bytes!("../../../../assets/sprites/patrol.png"),
    ),
    (
        ArtKind::Prospector,
        "prospector",
        include_bytes!("../../../../assets/sprites/prospector.png"),
    ),
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
        ArtKind::OrangeDwarf,
        "orange-dwarf",
        include_bytes!("../../../../assets/sprites/orange-dwarf.png"),
    ),
    (
        ArtKind::BlueWhiteStar,
        "blue-white-star",
        include_bytes!("../../../../assets/sprites/blue-white-star.png"),
    ),
    (
        ArtKind::BrownDwarf,
        "brown-dwarf",
        include_bytes!("../../../../assets/sprites/brown-dwarf.png"),
    ),
    (
        ArtKind::NeutronStar,
        "neutron-star",
        include_bytes!("../../../../assets/sprites/neutron-star.png"),
    ),
    (
        ArtKind::RockyPlanet,
        "rocky-planet",
        include_bytes!("../../../../assets/sprites/rocky-planet.png"),
    ),
    (
        ArtKind::AridPlanet,
        "arid-planet",
        include_bytes!("../../../../assets/sprites/arid-planet.png"),
    ),
    (
        ArtKind::FrozenPlanet,
        "frozen-planet",
        include_bytes!("../../../../assets/sprites/frozen-planet.png"),
    ),
    (
        ArtKind::OceanPlanet,
        "ocean-planet",
        include_bytes!("../../../../assets/sprites/ocean-planet.png"),
    ),
    (
        ArtKind::JunglePlanet,
        "jungle-planet",
        include_bytes!("../../../../assets/sprites/jungle-planet.png"),
    ),
    (
        ArtKind::VolcanicPlanet,
        "volcanic-planet",
        include_bytes!("../../../../assets/sprites/volcanic-planet.png"),
    ),
    (
        ArtKind::BarrenPlanet,
        "barren-planet",
        include_bytes!("../../../../assets/sprites/barren-planet.png"),
    ),
    (
        ArtKind::GasGiant,
        "gas-giant",
        include_bytes!("../../../../assets/sprites/gas-giant.png"),
    ),
    (
        ArtKind::StormGiant,
        "storm-giant",
        include_bytes!("../../../../assets/sprites/storm-giant.png"),
    ),
    (
        ArtKind::MethaneGiant,
        "methane-giant",
        include_bytes!("../../../../assets/sprites/methane-giant.png"),
    ),
    (
        ArtKind::Moon,
        "moon",
        include_bytes!("../../../../assets/sprites/moon.png"),
    ),
    (
        ArtKind::IceMoon,
        "ice-moon",
        include_bytes!("../../../../assets/sprites/ice-moon.png"),
    ),
    (
        ArtKind::Asteroid,
        "asteroid",
        include_bytes!("../../../../assets/sprites/asteroid.png"),
    ),
    (
        ArtKind::MetallicAsteroid,
        "metallic-asteroid",
        include_bytes!("../../../../assets/sprites/metallic-asteroid.png"),
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

    /// Like `paint`, mirrored when `mirrored` (ship art faces right by default).
    pub fn paint_facing(
        &self,
        kind: ArtKind,
        painter: &Painter,
        center: Pos2,
        size: f32,
        mirrored: bool,
    ) -> bool {
        let Some(texture) = self.textures.get(&kind) else {
            return false;
        };
        let (left, right) = if mirrored { (1.0, 0.0) } else { (0.0, 1.0) };
        painter.image(
            texture.id(),
            Rect::from_center_size(center, Vec2::splat(size)),
            Rect::from_min_max(Pos2::new(left, 0.0), Pos2::new(right, 1.0)),
            Color32::WHITE,
        );
        true
    }

    pub fn preview(&self, kind: ArtKind, ui: &mut egui::Ui) {
        if let Some(texture) = self.textures.get(&kind) {
            ui.image((texture.id(), Vec2::splat(160.0)));
        }
    }

    /// Image-first destination control; the name also remains available to assistive UI.
    pub fn tile(
        &self,
        ui: &mut egui::Ui,
        kind: ArtKind,
        name: &str,
        selected: bool,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(80.0, 66.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name)
        });
        let color = if selected || response.hovered() {
            crate::theme::GREEN
        } else {
            Color32::from_rgb(72, 91, 85)
        };
        ui.painter().rect(
            rect,
            4.0,
            Color32::from_rgb(11, 20, 23),
            egui::Stroke::new(1.0_f32, color),
            egui::StrokeKind::Inside,
        );
        self.paint(
            kind,
            ui.painter(),
            rect.center_top() + Vec2::new(0.0, 23.0),
            38.0,
        );
        // Wrap long names inside their own tile instead of letting them obscure neighbors.
        let text = ui.painter().layout(
            name.into(),
            egui::FontId::proportional(12.0),
            if selected {
                crate::theme::GREEN
            } else {
                Color32::from_rgb(190, 209, 200)
            },
            rect.width() - 6.0,
        );
        ui.painter().galley(
            egui::pos2(rect.center().x - text.size().x * 0.5, rect.bottom() - 22.0),
            text,
            color,
        );
        response.on_hover_text(name)
    }

    pub fn warnings(&self, ui: &mut egui::Ui) {
        for error in &self.errors {
            ui.label(error);
        }
    }
}

#[cfg(test)]
mod tests;
