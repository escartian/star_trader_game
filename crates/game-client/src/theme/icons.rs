//! Original Blender-authored SVG console symbols (assets/icons), rasterized per DPI.
//! Every icon-only control keeps its text as hover help and accessible name.
pub mod goods;
mod sources;
mod svg;
#[cfg(test)]
mod tests;

use egui::{Color32, Painter, Pos2, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use game_core::{
    economy::Commodity,
    planet::{Biome, Danger, EconomyType, PlanetProfile},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Icon {
    Fleets,
    Ship,
    Shipyard,
    Save,
    Load,
    New,
    Navigation,
    Market,
    Execute,
    Back,
    Focus,
    Overview,
    Credits,
    Funds,
    Cargo,
    Fuel,
    Refuel,
    Clock,
    Dock,
    Orbit,
    Pin,
    DeepSpace,
    Underway,
    Buy,
    Sell,
    Upgrade,
    Hold,
    Cancel,
    Confirm,
    Plot,
    Split,
    Merge,
    Rename,
    Transfer,
    Log,
    Reset,
    Pan,
    Star,
    Warning,
    Info,
    Distance,
    Legs,
    Bypass,
    Contact,
    Enter,
    Seed,
    Help,
    Settings,
    Danger,
    Hull,
    Shield,
    Armor,
    Engine,
    Weapon,
    Good(Commodity),
    Economy(EconomyType),
    Biome(Biome),
}

/// Paint at the standard 20-point size.
pub fn paint(painter: &Painter, icon: Icon, center: Pos2, color: Color32) {
    paint_sized(painter, icon, center, 20.0, color);
}

pub fn paint_sized(painter: &Painter, icon: Icon, center: Pos2, size: f32, color: Color32) {
    svg::paint(painter, icon, center, size, color);
}

fn accent(ui: &Ui) -> Color32 {
    if ui.is_enabled() {
        super::GREEN
    } else {
        Color32::from_rgb(90, 104, 98)
    }
}

fn frame(ui: &Ui, rect: egui::Rect, response: &Response, selected: bool) {
    let visual = ui.style().interact_selectable(response, selected);
    ui.painter().rect(
        rect,
        3.0,
        visual.weak_bg_fill,
        visual.bg_stroke,
        StrokeKind::Inside,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            3.0,
            Stroke::new(1.0_f32, super::GREEN),
            StrokeKind::Outside,
        );
    }
    ui.painter().line_segment(
        [
            rect.left_top() + Vec2::splat(1.0),
            rect.right_top() + Vec2::new(-1.0, 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
    );
}

/// Icon plus a short label, for primary actions where words aid discovery.
pub fn button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    labeled_toggle(ui, icon, label, false)
}

/// [`button`] drawn as selected when `on` is true (tabs, modes).
pub fn labeled_toggle(ui: &mut Ui, icon: Icon, label: &str, on: bool) -> Response {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        super::fonts::text(15.0),
        ui.visuals().text_color(),
    );
    let size = Vec2::new(text.size().x + 42.0, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), on, label)
    });
    frame(ui, rect, &response, on);
    paint(
        ui.painter(),
        icon,
        rect.left_center() + Vec2::new(15.0, 0.0),
        accent(ui),
    );
    ui.painter().galley(
        rect.left_center() + Vec2::new(29.0, -text.size().y * 0.5),
        text,
        ui.visuals().text_color(),
    );
    response
}

/// Square icon-only control; `tooltip` names the action for hover and screen readers.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    toggle(ui, icon, tooltip, false)
}

/// Icon-only control drawn as selected when `on` is true.
pub fn toggle(ui: &mut Ui, icon: Icon, tooltip: &str, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), on, tooltip)
    });
    frame(ui, rect, &response, on);
    paint(ui.painter(), icon, rect.center(), accent(ui));
    response
        .on_hover_text(tooltip)
        .on_disabled_hover_text(tooltip)
}

/// Icon-only control that can be disabled with an explanation.
pub fn action(ui: &mut Ui, icon: Icon, tooltip: &str, blocked: Option<String>) -> Response {
    let enabled = blocked.is_none();
    let response = ui
        .add_enabled_ui(enabled, |ui| icon_button(ui, icon, tooltip))
        .inner;
    match blocked {
        Some(reason) => response.on_disabled_hover_text(reason),
        None => response,
    }
}

/// Labelled variant of [`action`] for decisions where words matter.
pub fn labeled_action(ui: &mut Ui, icon: Icon, label: &str, blocked: Option<String>) -> Response {
    let enabled = blocked.is_none();
    let response = ui
        .add_enabled_ui(enabled, |ui| button(ui, icon, label))
        .inner;
    match blocked {
        Some(reason) => response.on_disabled_hover_text(reason),
        None => response,
    }
}

/// Small read-only glyph with a value; hover explains what the value means.
pub fn stat(ui: &mut Ui, icon: Icon, color: Color32, value: &str, tooltip: &str) -> Response {
    let text = ui.painter().layout_no_wrap(
        value.into(),
        super::fonts::text(15.0),
        ui.visuals().text_color(),
    );
    let width = if value.is_empty() {
        18.0
    } else {
        text.size().x + 24.0
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, format!("{tooltip}: {value}"))
    });
    paint_sized(
        ui.painter(),
        icon,
        rect.left_center() + Vec2::new(9.0, 0.0),
        16.0,
        color,
    );
    ui.painter().galley(
        rect.left_center() + Vec2::new(21.0, -text.size().y * 0.5),
        text,
        ui.visuals().text_color(),
    );
    response.on_hover_text(tooltip)
}

/// A glyph without a value, e.g. a status marker.
pub fn glyph(ui: &mut Ui, icon: Icon, color: Color32, tooltip: &str) -> Response {
    stat(ui, icon, color, "", tooltip)
}

pub fn commodity(ui: &mut Ui, good: Commodity) -> Response {
    glyph(
        ui,
        Icon::Good(good),
        goods::commodity_color(good),
        good.label(),
    )
}

/// Shield plus filled pips: readable without relying on color alone.
pub fn danger(ui: &mut Ui, danger: Danger) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(44.0, 20.0), Sense::hover());
    let color = goods::danger_color(danger);
    let tooltip = format!("Danger: {}", danger.label());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &tooltip));
    paint_sized(
        ui.painter(),
        Icon::Danger,
        rect.left_center() + Vec2::new(9.0, 0.0),
        16.0,
        color,
    );
    for pip in 0..6 {
        let x = rect.left() + 20.0 + pip as f32 * 4.0;
        let bar =
            egui::Rect::from_min_size(egui::pos2(x, rect.center().y - 5.0), Vec2::new(2.5, 10.0));
        let fill = if pip < goods::danger_level(danger) {
            color
        } else {
            Color32::from_white_alpha(25)
        };
        ui.painter().rect_filled(bar, 0.5, fill);
    }
    response.on_hover_text(tooltip)
}

/// Economy, specialization, biome and danger as one compact row.
pub fn planet_traits(ui: &mut Ui, profile: &PlanetProfile) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        glyph(
            ui,
            Icon::Economy(profile.economy),
            super::AMBER,
            &format!("{} economy", profile.economy.label()),
        );
        let good = profile.specialization;
        glyph(
            ui,
            Icon::Good(good),
            goods::commodity_color(good),
            &format!("Produces {}", good.label()),
        );
        glyph(
            ui,
            Icon::Biome(profile.biome),
            super::CYAN,
            &format!("Biome: {}", profile.biome.label()),
        );
        danger(ui, profile.danger);
    });
}
