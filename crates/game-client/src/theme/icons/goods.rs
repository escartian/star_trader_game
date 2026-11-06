//! Commodity and danger tints; glyph shapes live in the SVG icon set.
use egui::Color32;
use game_core::{economy::Commodity, planet::Danger};

/// Identifying tint per commodity; icons keep the same shape in any color.
pub fn commodity_color(good: Commodity) -> Color32 {
    match good {
        Commodity::Food => Color32::from_rgb(170, 225, 95),
        Commodity::Ore => Color32::from_rgb(205, 160, 120),
        Commodity::Medicine => Color32::from_rgb(240, 110, 120),
        Commodity::Machinery => Color32::from_rgb(170, 190, 205),
        Commodity::Textiles => Color32::from_rgb(220, 150, 230),
        Commodity::Electronics => Color32::from_rgb(91, 225, 226),
    }
}

/// Green through red, then violet for the uniquely treacherous Insidious rating.
pub fn danger_color(danger: Danger) -> Color32 {
    match danger {
        Danger::VerySafe => Color32::from_rgb(120, 230, 110),
        Danger::Safe => Color32::from_rgb(170, 225, 100),
        Danger::Moderate => Color32::from_rgb(245, 200, 90),
        Danger::Dangerous => Color32::from_rgb(245, 140, 70),
        Danger::VeryDangerous => Color32::from_rgb(240, 80, 70),
        Danger::Insidious => Color32::from_rgb(200, 110, 240),
    }
}

/// One to six pips; used beside the shield so danger is not color-only.
pub fn danger_level(danger: Danger) -> usize {
    match danger {
        Danger::VerySafe => 1,
        Danger::Safe => 2,
        Danger::Moderate => 3,
        Danger::Dangerous => 4,
        Danger::VeryDangerous => 5,
        Danger::Insidious => 6,
    }
}
