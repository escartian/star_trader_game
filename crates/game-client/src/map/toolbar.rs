//! Icon toolbar shared by galaxy and system views. View presets and zoom apply
//! directly to the camera; focus requests are returned for the view to resolve.
use crate::theme::icons::{Icon, icon_button, toggle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraAction {
    Reset,
    FocusSelection,
    FocusFleet,
    Overview,
}

impl CameraAction {
    fn icon(self) -> (Icon, &'static str) {
        match self {
            Self::Reset => (Icon::Reset, "Reset view"),
            Self::FocusSelection => (Icon::Focus, "Focus selection (F)"),
            Self::FocusFleet => (Icon::Ship, "Focus selected fleet"),
            Self::Overview => (Icon::Overview, "System overview"),
        }
    }
}

/// A small column of icon buttons floating at the map's top-right corner (the
/// phone-first replacement for the camera toolbar): the given camera actions,
/// then the pan tool for touch screens, which cannot middle-drag.
pub fn map_buttons(
    ctx: &egui::Context,
    map: egui::Rect,
    id: &str,
    pan_tool: &mut bool,
    actions: &[CameraAction],
) -> Option<CameraAction> {
    let mut chosen = None;
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .fixed_pos(map.right_top() + egui::vec2(-46.0, 8.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.vertical(|ui| {
                for &action in actions {
                    let (icon, label) = action.icon();
                    if icon_button(ui, icon, label).clicked() {
                        chosen = Some(action);
                    }
                }
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
            });
        });
    chosen
}
