//! Focus controls shared by galaxy and system views.
use crate::theme::icons::{Icon, icon_button, toggle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraAction {
    Reset,
    FocusSelection,
    FocusFleet,
    Overview,
    /// Opens or closes the galaxy's name search.
    Search,
    /// Shows or hides NPC traffic on the system map.
    ToggleTraffic,
    /// Shows or hides faction territory on the galaxy map.
    ToggleTerritory,
}

impl CameraAction {
    fn icon(self) -> (Icon, &'static str) {
        match self {
            Self::Reset => (Icon::Reset, "Reset view"),
            Self::FocusSelection => (Icon::Focus, "Focus selection (F)"),
            Self::FocusFleet => (Icon::Ship, "Focus selected fleet"),
            Self::Overview => (Icon::Overview, "System overview"),
            Self::Search => (Icon::Search, "Search stars & planets (Ctrl+F)"),
            Self::ToggleTraffic => (
                Icon::Contact,
                "Show other fleets (traders, police, raiders)",
            ),
            Self::ToggleTerritory => (Icon::Shield, "Show faction territory"),
        }
    }
}

/// A small column of focus buttons at the map's top-right corner. Each
/// action comes with `Some(on)` when it is a layer toggle (drawn lit while on).
pub fn map_buttons(
    ctx: &egui::Context,
    map: egui::Rect,
    id: &str,
    actions: &[(CameraAction, Option<bool>)],
) -> Option<CameraAction> {
    let mut chosen = None;
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .fixed_pos(map.right_top() + egui::vec2(-46.0, 8.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.vertical(|ui| {
                for &(action, on) in actions {
                    let (icon, label) = action.icon();
                    let response = match on {
                        Some(on) => toggle(ui, icon, label, on),
                        None => icon_button(ui, icon, label),
                    };
                    if response.clicked() {
                        chosen = Some(action);
                    }
                }
            });
        });
    chosen
}
