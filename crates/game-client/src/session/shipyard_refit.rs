//! Shipyard removal and same-ship swap controls. The host quotes and applies
//! each change; this panel only previews the result against a cloned snapshot.
use super::LocalSession;
use crate::theme::AMBER;
use crate::theme::icons::{self, Icon};
use game_core::{combat::gear::Gear, economy::ShipUpgrade, travel::Ship};
use game_protocol::ClientCommand;

impl LocalSession {
    pub(super) fn shipyard_refit_ui(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
    ) -> Option<ClientCommand> {
        ui.separator();
        ui.label(
            egui::RichText::new(format!("REFIT {}", ship.name.to_uppercase()))
                .small()
                .color(AMBER),
        );
        ui.small("Remove an upgrade for its current resale value, or swap it in one transaction. Cargo must fit after a capacity change.");
        let mut fitted = Vec::new();
        if ship.upgraded {
            fitted.push(ShipUpgrade::Hold);
        }
        fitted.extend(ship.gear.iter().copied().map(ShipUpgrade::Gear));
        if fitted.is_empty() {
            ui.weak("No upgrades fitted to this ship.");
        }
        let mut action = None;
        for old in fitted {
            ui.horizontal_wrapped(|ui| {
                ui.strong(old.label());
                let (refund, blocked) = self.refit_quote(ship, old, None);
                let label = format!("Remove · {refund} cr back");
                if icons::labeled_action(ui, Icon::Sell, &label, blocked).clicked() {
                    action = Some(ClientCommand::RefitShip {
                        ship: ship.id,
                        remove: old,
                        install: None,
                        expected_revision: self.state.revision,
                    });
                }
                ui.menu_button("Swap for…", |ui| {
                    let size = self
                        .economy
                        .rules
                        .ship_catalog
                        .spec(ship.kind)
                        .map(|spec| spec.combat.size);
                    let choices = std::iter::once(ShipUpgrade::Hold).chain(
                        Gear::ALL
                            .into_iter()
                            .filter(|gear| size.is_some_and(|size| gear.fits(ship.kind, size)))
                            .map(ShipUpgrade::Gear),
                    );
                    for new in choices {
                        if new == old || self.has_upgrade(ship, new) {
                            continue;
                        }
                        let (_, blocked) = self.refit_quote(ship, old, Some(new));
                        let label = format!("{} · {} cr", new.label(), self.upgrade_price(new));
                        if icons::labeled_action(ui, Icon::Upgrade, &label, blocked).clicked() {
                            action = Some(ClientCommand::RefitShip {
                                ship: ship.id,
                                remove: old,
                                install: Some(new),
                                expected_revision: self.state.revision,
                            });
                            ui.close_menu();
                        }
                    }
                })
                .response
                .on_hover_text("Refund the old fitting and pay for the replacement.");
            });
        }
        action
    }

    fn has_upgrade(&self, ship: &Ship, upgrade: ShipUpgrade) -> bool {
        match upgrade {
            ShipUpgrade::Hold => ship.upgraded,
            ShipUpgrade::Gear(gear) => ship.gear.contains(&gear),
        }
    }

    fn upgrade_price(&self, upgrade: ShipUpgrade) -> game_core::economy::Credits {
        match upgrade {
            ShipUpgrade::Hold => self.economy.hold_upgrade_price(),
            ShipUpgrade::Gear(gear) => self.economy.gear_price(gear),
        }
    }

    fn refit_quote(
        &self,
        ship: &Ship,
        remove: ShipUpgrade,
        install: Option<ShipUpgrade>,
    ) -> (game_core::economy::Credits, Option<String>) {
        let refund = self.economy.upgrade_refund(remove);
        let result = self.economy.quote_refit(ship, remove, install);
        (refund, result.err().map(|error| error.to_string()))
    }
}
