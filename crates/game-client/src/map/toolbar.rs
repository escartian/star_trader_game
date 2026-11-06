//! Icon toolbar shared by galaxy and system views. View presets and zoom apply
//! directly to the camera; focus requests are returned for the view to resolve.
use super::camera::Camera;
use crate::theme::icons::{Icon, icon_button, toggle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraAction {
    Reset,
    FocusSelection,
    FocusFleet,
    FocusStar,
    Overview,
}

impl CameraAction {
    fn icon(self) -> (Icon, &'static str) {
        match self {
            Self::Reset => (Icon::Reset, "Reset view"),
            Self::FocusSelection => (Icon::Focus, "Focus selection (F)"),
            Self::FocusFleet => (Icon::Ship, "Focus selected fleet"),
            Self::FocusStar => (Icon::Star, "Focus star"),
            Self::Overview => (Icon::Overview, "System overview"),
        }
    }
}

pub fn camera_toolbar(
    ui: &mut egui::Ui,
    camera: &mut Camera,
    pan_tool: &mut bool,
    actions: &[CameraAction],
    help: &str,
) -> Option<CameraAction> {
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for &action in actions {
            let (icon, label) = action.icon();
            if icon_button(ui, icon, label).clicked() {
                chosen = Some(action);
            }
        }
        ui.separator();
        if toggle(
            ui,
            Icon::Pan,
            "Pan tool: drag to move instead of orbit",
            *pan_tool,
        )
        .clicked()
        {
            *pan_tool = !*pan_tool;
        }
        let half_pi = std::f64::consts::FRAC_PI_2;
        for (icon, label, yaw, pitch) in [
            (Icon::ViewTop, "Top view (XZ)", 0.0, half_pi),
            (Icon::ViewFront, "Front view (XY)", 0.0, 0.0),
            (Icon::ViewSide, "Side view (ZY)", half_pi, 0.0),
        ] {
            if icon_button(ui, icon, label).clicked() {
                camera.yaw = yaw;
                camera.pitch = pitch;
            }
        }
        ui.separator();
        crate::theme::icons::glyph(ui, Icon::Zoom, crate::theme::GREEN, "Zoom (mouse wheel)");
        ui.style_mut().visuals.slider_trailing_fill = true;
        ui.add(egui::Slider::new(&mut camera.zoom, 0.3..=4.0).show_value(false))
            .on_hover_text("Zoom (mouse wheel)");
        crate::theme::icons::glyph(ui, Icon::Help, crate::theme::CYAN, help);
    });
    chosen
}
