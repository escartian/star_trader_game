//! Shared native command-console styling; no runtime art or animation dependency.
use egui::{Color32, Context, FontFamily, FontId, Stroke, TextStyle, Vec2};
pub const CYAN: Color32 = Color32::from_rgb(91, 225, 226);
pub const AMBER: Color32 = Color32::from_rgb(245, 190, 91);
pub const PANEL: Color32 = Color32::from_rgb(10, 21, 36);

pub fn apply(ctx: &Context) {
    let key = egui::Id::new("command-console-theme");
    if ctx.data_mut(|data| data.get_temp::<bool>(key)).is_some() {
        return;
    }
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = Color32::from_rgb(12, 25, 42);
    style.visuals.extreme_bg_color = Color32::from_rgb(5, 13, 24);
    style.visuals.faint_bg_color = Color32::from_rgb(17, 37, 54);
    style.visuals.override_text_color = Some(Color32::from_rgb(201, 221, 234));
    style.visuals.selection.bg_fill = Color32::from_rgb(26, 79, 94);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, CYAN);
    style.visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(47, 105, 124));
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(33, 65, 85));
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(19, 43, 62);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(19, 43, 62);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(26, 72, 88);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, CYAN);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(31, 99, 109);
    style.visuals.hyperlink_color = CYAN;
    style.spacing.item_spacing = Vec2::new(9.0, 7.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(14);
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(19.0, FontFamily::Monospace));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Monospace));
    ctx.set_style(style);
    ctx.data_mut(|data| data.insert_temp(key, true));
}

pub fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.colored_label(CYAN, egui::RichText::new(title).monospace().size(19.0));
    ui.small(subtitle);
    ui.separator();
}
