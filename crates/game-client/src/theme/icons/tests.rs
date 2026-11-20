use super::{Icon, sources::SOURCES, svg};
use game_core::{
    economy::Commodity,
    planet::{Biome, EconomyType},
};

fn all_icons() -> Vec<Icon> {
    let mut icons = vec![
        Icon::Fleets,
        Icon::Ship,
        Icon::Shipyard,
        Icon::Save,
        Icon::Load,
        Icon::New,
        Icon::Navigation,
        Icon::Market,
        Icon::Execute,
        Icon::Back,
        Icon::Focus,
        Icon::Overview,
        Icon::Credits,
        Icon::Funds,
        Icon::Cargo,
        Icon::Fuel,
        Icon::Refuel,
        Icon::Clock,
        Icon::Dock,
        Icon::Orbit,
        Icon::Pin,
        Icon::DeepSpace,
        Icon::Underway,
        Icon::Buy,
        Icon::Sell,
        Icon::Upgrade,
        Icon::Hold,
        Icon::Cancel,
        Icon::Confirm,
        Icon::Plot,
        Icon::Split,
        Icon::Merge,
        Icon::Rename,
        Icon::Transfer,
        Icon::Log,
        Icon::Reset,
        Icon::Star,
        Icon::Warning,
        Icon::Info,
        Icon::Distance,
        Icon::Legs,
        Icon::Bypass,
        Icon::Contact,
        Icon::Enter,
        Icon::Seed,
        Icon::Help,
        Icon::Settings,
        Icon::Danger,
        Icon::Hull,
        Icon::Shield,
        Icon::Armor,
        Icon::Engine,
        Icon::Weapon,
    ];
    icons.extend(Commodity::ALL.map(Icon::Good));
    icons.extend(
        [
            EconomyType::Agricultural,
            EconomyType::Extraction,
            EconomyType::Industrial,
        ]
        .map(Icon::Economy),
    );
    icons.extend([Biome::Temperate, Biome::Arid, Biome::Frozen, Biome::Gas].map(Icon::Biome));
    icons
}

#[test]
fn every_icon_has_an_svg_that_renders_visible_pixels_at_any_size() {
    let icons = all_icons();
    assert_eq!(
        icons.len(),
        SOURCES.len(),
        "an exported SVG has no Icon variant"
    );
    for icon in icons {
        for pixels in [16, 40] {
            let image = svg::rasterize(icon, pixels)
                .unwrap_or_else(|| panic!("{} did not parse", svg::name(icon)));
            assert_eq!(image.size, [pixels as usize; 2]);
            let lit = image.pixels.iter().filter(|pixel| pixel.a() > 64).count();
            assert!(
                lit > pixels as usize,
                "{} is blank at {pixels}px",
                svg::name(icon)
            );
        }
    }
}
