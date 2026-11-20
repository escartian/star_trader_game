//! Small session controls shared by the shell's sheets.
use super::LocalSession;
use crate::theme::icons::{Icon, icon_button};
use game_core::{SystemId, travel::Location};

impl LocalSession {
    /// In deep space with a system selected on the galaxy map: preview entering it.
    pub fn enter_button(&mut self, ui: &mut egui::Ui, target: Option<SystemId>) {
        if matches!(self.selected_ship().location, Location::DeepSpace(_))
            && let Some(system) = target
            && icon_button(ui, Icon::Enter, "Preview entry into the selected system").clicked()
        {
            self.request_movement(game_core::movement::MoveTarget::EnterSystem(system));
        }
    }
}
