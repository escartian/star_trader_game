//! Pursuit in the Fleets page's traffic list: Pursue on traders and raider
//! bands in the flagship's system, the raider bands there, and the running
//! chase with a Stop button. Rules live in `game_core::pursuit`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::iff;
use game_core::{progress::Feature, pursuit::Quarry};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

impl LocalSession {
    /// Whether the flagship can be ordered to chase things (battles unlocked).
    pub(super) fn can_pursue(&self) -> bool {
        self.unlocked(Feature::Battles) && self.state.flagship(LOCAL_PLAYER).is_some()
    }

    /// A Pursue button; returns whether it was clicked.
    pub(super) fn pursue_button(ui: &mut egui::Ui, what: &str) -> bool {
        icons::button(ui, Icon::Engine, "Pursue")
            .on_hover_text(format!(
                "Chase {what} with your flagship, run after run. Catching it opens the \
                 contact; it ends if the quarry leaves the system or you give new orders."
            ))
            .clicked()
    }

    /// Sends the flagship after `quarry`.
    pub(super) fn pursue(&mut self, quarry: Quarry) {
        if let Some(fleet) = self.state.flagship(LOCAL_PLAYER) {
            self.command(ClientCommand::Pursue {
                fleet,
                quarry,
                expected_revision: self.state.revision,
            });
        }
    }

    /// What the flagship is chasing, with Stop (a Wait order).
    pub(super) fn pursuit_status(&mut self, ui: &mut egui::Ui) {
        let Some(pursuit) = self.state.navigation.pursuit else {
            return;
        };
        let name = match pursuit.quarry {
            Quarry::Trader(id) => self
                .economy
                .traders
                .iter()
                .find(|npc| npc.id == id)
                .map_or_else(|| "a trader".to_owned(), |npc| npc.name.clone()),
            Quarry::Band(id) => self
                .state
                .navigation
                .bands
                .iter()
                .find(|band| band.id == id)
                .map_or_else(
                    || "a raider band".to_owned(),
                    |band| format!("raiders · {}", crew(&band.ships)),
                ),
        };
        let mut stop = false;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Engine, iff::HOSTILE, "Pursuit");
            ui.colored_label(iff::HOSTILE, format!("Pursuing {name}"));
            stop = icons::button(ui, Icon::Hold, "Stop")
                .on_hover_text("Give up the chase and wait here")
                .clicked();
        });
        if stop {
            self.command(ClientCommand::HoldFleet {
                fleet: pursuit.fleet,
                expected_revision: self.state.revision,
            });
        }
    }

    /// Raider bands in the flagship's system, each with Pursue; returns the
    /// band whose Pursue was clicked.
    pub(super) fn raider_rows(
        &self,
        ui: &mut egui::Ui,
        here: Option<game_core::SystemId>,
    ) -> Option<u64> {
        let bands: Vec<_> = self
            .state
            .navigation
            .bands
            .iter()
            .filter(|band| Some(band.system) == here && band.transit.is_none())
            .cloned()
            .collect();
        if bands.is_empty() {
            return None;
        }
        let mut chase = None;
        for band in bands {
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Danger, iff::HOSTILE, "Raider band");
                ui.colored_label(iff::HOSTILE, format!("Raiders · {}", crew(&band.ships)));
                if self.can_pursue() && Self::pursue_button(ui, "this raider band") {
                    chase = Some(band.id);
                }
            });
        }
        chase
    }
}

/// "2 scouts", "patrol + scout": a group's ships in words.
pub fn crew(ships: &[game_core::fleet::ShipKind]) -> String {
    use game_core::fleet::ShipKind;
    let count = |kind| ships.iter().filter(|ship| **ship == kind).count();
    let parts: Vec<String> = [
        (ShipKind::Patrol, "patrol", "patrols"),
        (ShipKind::Scout, "scout", "scouts"),
        (ShipKind::Freighter, "freighter", "freighters"),
        (ShipKind::Prospector, "prospector", "prospectors"),
    ]
    .into_iter()
    .filter_map(|(kind, one, many)| match count(kind) {
        0 => None,
        1 => Some(one.to_owned()),
        n => Some(format!("{n} {many}")),
    })
    .collect();
    parts.join(" + ")
}

#[cfg(test)]
mod tests {
    use super::crew;
    use game_core::fleet::ShipKind;

    #[test]
    fn crews_read_naturally() {
        assert_eq!(crew(&[ShipKind::Scout, ShipKind::Scout]), "2 scouts");
        assert_eq!(crew(&[ShipKind::Patrol, ShipKind::Scout]), "patrol + scout");
    }
}
