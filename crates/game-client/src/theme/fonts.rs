//! Bundled SIL OFL fonts (see assets/fonts/*-OFL.txt). Rajdhani is the single
//! text face for body, controls and numbers; Orbitron is reserved for display
//! headings. egui's default fonts stay last in each family as glyph fallbacks.
use egui::{FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const DISPLAY: &str = "display";

const RAJDHANI: &[u8] = include_bytes!("../../../../assets/fonts/Rajdhani-Medium.ttf");
const RAJDHANI_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/Rajdhani-SemiBold.ttf");
const ORBITRON: &[u8] = include_bytes!("../../../../assets/fonts/Orbitron.ttf");

pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        ("rajdhani", RAJDHANI),
        ("rajdhani-bold", RAJDHANI_BOLD),
        ("orbitron", ORBITRON),
    ] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.insert(0, "rajdhani".into());
    }
    let mut display = vec!["orbitron".to_string(), "rajdhani-bold".to_string()];
    display.extend(fallback);
    fonts
        .families
        .insert(FontFamily::Name(DISPLAY.into()), display);
    fonts
}

/// Orbitron at `size` points, for titles and the few large headings.
/// New font definitions bind on the frame after `set_fonts`, so the first frame
/// of a fresh context falls back to the body face instead of an unbound family.
pub fn display(ctx: &egui::Context, size: f32) -> FontId {
    let family = FontFamily::Name(DISPLAY.into());
    if ctx.fonts(|fonts| fonts.families().contains(&family)) {
        FontId::new(size, family)
    } else {
        FontId::proportional(size)
    }
}

/// The standard text face at `size` points.
pub fn text(size: f32) -> FontId {
    FontId::proportional(size)
}
