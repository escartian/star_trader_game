//! Shared native command-console styling; no runtime art or animation dependency.
use egui::{Color32, Context, FontFamily, FontId, Stroke, TextStyle, Vec2};
pub mod icons;
pub const GREEN: Color32 = Color32::from_rgb(148, 255, 85);
pub const CYAN: Color32 = Color32::from_rgb(91, 225, 226);
pub const AMBER: Color32 = Color32::from_rgb(245, 190, 91);
pub const PANEL: Color32 = Color32::from_rgb(21, 27, 29);

pub fn apply(ctx: &Context) {
    let key = egui::Id::new("command-console-theme");
    if ctx.data_mut(|data| data.get_temp::<bool>(key)).is_some() {
        return;
    }
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = Color32::from_rgb(25, 32, 34);
    style.visuals.extreme_bg_color = Color32::from_rgb(5, 13, 24);
    style.visuals.faint_bg_color = Color32::from_rgb(37, 46, 47);
    style.visuals.override_text_color = Some(Color32::from_rgb(201, 221, 234));
    style.visuals.selection.bg_fill = Color32::from_rgb(42, 70, 38);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, GREEN);
    style.visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(85, 107, 94));
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(63, 80, 76));
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(39, 51, 48);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(39, 51, 48);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(51, 76, 45);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, GREEN);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(56, 92, 43);
    style.visuals.hyperlink_color = GREEN;
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
    ui.colored_label(GREEN, egui::RichText::new(title).monospace().size(19.0));
    ui.small(subtitle);
    ui.separator();
}

/// Static brushed gunmetal with a soft highlight and machined edges.
/// Painted before content so controls and text remain crisp.
pub fn metal_panel(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    let mut mesh = egui::Mesh::default();
    for (point, color) in [
        (rect.left_top(), Color32::from_rgb(48, 58, 59)),
        (rect.right_top(), Color32::from_rgb(32, 40, 42)),
        (rect.left_bottom(), PANEL),
        (rect.right_bottom(), Color32::from_rgb(15, 20, 22)),
    ] {
        mesh.colored_vertex(point, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
    let mut y = rect.top() + 3.0;
    let mut row = 0_u32;
    while y < rect.bottom() {
        let x = (rect.left() + ((row * 67) % 211) as f32).min(rect.right());
        painter.line_segment(
            [
                egui::pos2(x, y),
                egui::pos2((x + 90.0).min(rect.right()), y),
            ],
            Stroke::new(0.5_f32, Color32::from_white_alpha(2)),
        );
        y += 7.0;
        row += 1;
    }
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0_f32, Color32::from_white_alpha(40)),
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::BLACK),
    );
    painter.line_segment(
        [rect.left_top(), rect.left_top() + Vec2::new(42.0, 0.0)],
        Stroke::new(2.0_f32, GREEN),
    );
}
