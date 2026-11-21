//! Rasterize embedded SVG icons on demand. Each (icon, pixel size) pair is
//! rendered once per egui context at the display's pixels-per-point, so icons
//! stay sharp at any DPI; the white artwork is tinted when painted.
use super::{Icon, sources::SOURCES};
use egui::{Color32, ColorImage, Painter, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use game_core::{
    economy::Commodity,
    planet::{Biome, EconomyType},
};
use std::collections::HashMap;

/// SVG artboard size in grid units (viewBox -10..10).
const GRID: f32 = 20.0;

pub fn name(icon: Icon) -> &'static str {
    match icon {
        Icon::Fleets => "fleets",
        Icon::Ship => "ship",
        Icon::Shipyard => "shipyard",
        Icon::Save => "save",
        Icon::Load => "load",
        Icon::New => "new",
        Icon::Navigation => "navigation",
        Icon::Market => "market",
        Icon::Execute => "execute",
        Icon::Back => "back",
        Icon::Focus => "focus",
        Icon::Overview => "overview",
        Icon::Credits => "credits",
        Icon::Funds => "funds",
        Icon::Cargo => "cargo",
        Icon::Fuel => "fuel",
        Icon::Refuel => "refuel",
        Icon::Clock => "clock",
        Icon::Dock => "dock",
        Icon::Orbit => "orbit",
        Icon::Pin => "pin",
        Icon::DeepSpace => "deep-space",
        Icon::Underway => "underway",
        Icon::Buy => "buy",
        Icon::Sell => "sell",
        Icon::Upgrade => "upgrade",
        Icon::Hold => "hold",
        Icon::Cancel => "cancel",
        Icon::Confirm => "confirm",
        Icon::Plot => "plot",
        Icon::Split => "split",
        Icon::Merge => "merge",
        Icon::Rename => "rename",
        Icon::Transfer => "transfer",
        Icon::Log => "log",
        Icon::Reset => "reset",
        Icon::Star => "star",
        Icon::Warning => "warning",
        Icon::Info => "info",
        Icon::Distance => "distance",
        Icon::Legs => "legs",
        Icon::Bypass => "bypass",
        Icon::Contact => "contact",
        Icon::Enter => "enter",
        Icon::Seed => "seed",
        Icon::Help => "help",
        Icon::Settings => "settings",
        Icon::Danger => "danger",
        Icon::Hull => "combat-hull",
        Icon::Shield => "combat-shield",
        Icon::Armor => "combat-armor",
        Icon::Engine => "combat-engine",
        Icon::Weapon => "combat-weapon",
        Icon::Good(good) => match good {
            Commodity::Food => "good-food",
            Commodity::Ore => "good-ore",
            Commodity::Medicine => "good-medicine",
            Commodity::Machinery => "good-machinery",
            Commodity::Textiles => "good-textiles",
            Commodity::Electronics => "good-electronics",
        },
        Icon::Economy(economy) => match economy {
            EconomyType::Agricultural => "economy-agricultural",
            EconomyType::Extraction => "economy-extraction",
            EconomyType::Industrial => "economy-industrial",
        },
        Icon::Biome(biome) => match biome {
            Biome::Temperate => "biome-temperate",
            Biome::Arid => "biome-arid",
            Biome::Frozen => "biome-frozen",
            Biome::Gas => "biome-gas",
            Biome::Ocean => "biome-ocean",
            Biome::Jungle => "biome-jungle",
            Biome::Volcanic => "biome-volcanic",
            Biome::Barren => "biome-barren",
            Biome::Metallic => "biome-metallic",
        },
    }
}

/// Premultiplied RGBA pixels for a square `pixels`-wide rendering, if the icon exists.
pub fn rasterize(icon: Icon, pixels: u32) -> Option<ColorImage> {
    let source = SOURCES
        .iter()
        .find(|(file, _)| *file == name(icon))
        .map(|(_, svg)| *svg)?;
    let tree = resvg::usvg::Tree::from_str(source, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels)?;
    let scale = pixels as f32 / GRID;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(ColorImage::from_rgba_premultiplied(
        [pixels as usize, pixels as usize],
        pixmap.data(),
    ))
}

#[derive(Clone, Default)]
struct Cache(HashMap<(&'static str, u32), TextureHandle>);

pub fn paint(painter: &Painter, icon: Icon, center: Pos2, size: f32, color: Color32) {
    let pixels = (size * painter.pixels_per_point()).ceil().max(1.0) as u32;
    let key = (name(icon), pixels);
    let ctx = painter.ctx();
    let id = egui::Id::new("svg-icon-cache");
    let cached = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Cache>(id)
            .0
            .get(&key)
            .cloned()
    });
    // Textures load outside `data_mut`: both take the context lock.
    let texture = match cached {
        Some(texture) => texture,
        None => {
            let Some(image) = rasterize(icon, pixels) else {
                return;
            };
            let texture = ctx.load_texture(
                format!("icon-{}-{pixels}", key.0),
                image,
                TextureOptions::LINEAR,
            );
            ctx.data_mut(|data| {
                data.get_temp_mut_or_default::<Cache>(id)
                    .0
                    .insert(key, texture.clone())
            });
            texture
        }
    };
    painter.image(
        texture.id(),
        Rect::from_center_size(center, Vec2::splat(size)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        color,
    );
}
