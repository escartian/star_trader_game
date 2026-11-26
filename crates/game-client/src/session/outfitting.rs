//! Port service controls on Trade and fitting controls on Shipyard. Rules
//! live in simulation; these rows only submit commands for docked ships.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN};
use game_core::{
    combat::gear::{ESCORT_RUNS, Gear},
    fleet::software::FleetSoftware,
    travel::Ship,
};
use game_protocol::ClientCommand;

/// The glyph for a fitting, in the Market and the Fleets roster.
pub(super) fn gear_icon(gear: Gear) -> Icon {
    match gear {
        Gear::Afterburners | Gear::ScoutDrive | Gear::EvasionJets => Icon::Engine,
        Gear::PointDefense | Gear::ShieldArray | Gear::ArmoredBulkheads => Icon::Shield,
        Gear::FreighterRacks | Gear::ProspectorDrill => Icon::Cargo,
        Gear::PatrolTargeting => Icon::Weapon,
    }
}

/// Fitted gear as small glyphs (hover for what each does), for the roster.
pub(super) fn fitted_gear(ui: &mut egui::Ui, ship: &Ship) {
    for gear in &ship.gear {
        icons::glyph(
            ui,
            gear_icon(*gear),
            GREEN,
            &format!("{}: {}", gear.label(), gear.summary()),
        );
    }
}

impl LocalSession {
    /// Refuel and repair each ship here, plus the fleet's escort contract.
    pub(super) fn trade_services_ui(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
    ) -> Option<ClientCommand> {
        let mut action = None;
        ui.label(egui::RichText::new("SHIP SERVICES").small().color(AMBER));
        for id in self.ships_here(ship.id) {
            let Ok(member) = self.state.ship(id) else {
                continue;
            };
            ui.label(egui::RichText::new(&member.name).strong())
                .on_hover_text(member.kind.label());
            if let Some(command) = ui
                .horizontal_wrapped(|ui| self.ship_service_buttons(ui, member))
                .inner
            {
                action = Some(command);
            }
        }
        ui.label(egui::RichText::new("FLEET SERVICE").small().color(AMBER));
        if let Some(command) = self.escort_button(ui, ship) {
            action = Some(command);
        }
        action
    }

    /// Buy fittings for every ship at this port and software for its fleet.
    pub(super) fn shipyard_upgrades_ui(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
    ) -> Option<ClientCommand> {
        let mut action = None;
        ui.separator();
        ui.label(egui::RichText::new("OUTFIT SHIPS").small().color(AMBER))
            .on_hover_text("Fittings stay with each ship and must fit its model and size.");
        for id in self.ships_here(ship.id) {
            let Ok(member) = self.state.ship(id) else {
                continue;
            };
            if let Some(command) = self.ship_upgrades(ui, member) {
                action = Some(command);
            }
        }
        ui.label(egui::RichText::new("FLEET SOFTWARE").small().color(AMBER))
            .on_hover_text("Bought once for the whole fleet; the lead ship pays here.");
        if let Some(command) = self.software_buttons(ui, ship) {
            action = Some(command);
        }
        action
    }

    /// A button per fleet software item, for the fleet `ship` belongs to.
    fn software_buttons(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let installed = self
            .state
            .fleet_for_ship(ship.id)
            .map(|fleet| fleet.software.clone())
            .unwrap_or_default();
        let mut action = None;
        for software in FleetSoftware::ALL {
            let price = self.economy.software_price(software);
            let (label, blocked) = if installed.contains(&software) {
                (
                    format!("{} ✓", software.label()),
                    Some(format!("{} already installed.", software.label())),
                )
            } else {
                let short = (price > self.economy.credits)
                    .then(|| format!("Costs {price} cr; you have {} cr.", self.economy.credits));
                (format!("{} · {price} cr", software.label()), short)
            };
            if icons::labeled_action(ui, Icon::Overview, &label, blocked)
                .on_hover_text(format!("Fleet software: {}", software.summary()))
                .clicked()
            {
                action = Some(ClientCommand::InstallSoftware {
                    ship: ship.id,
                    software,
                    expected_revision: self.state.revision,
                });
            }
        }
        action
    }

    /// One ship's upgrades, one per line to keep the card's width fixed.
    fn ship_upgrades(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let mut action = None;
        let size = self
            .economy
            .rules
            .ship_catalog
            .spec(ship.kind)
            .map(|spec| spec.combat.size);
        ui.label(egui::RichText::new(&ship.name).strong())
            .on_hover_text(ship.kind.label());
        if let Some(command) = self.hold_upgrade_button(ui, ship) {
            action = Some(command);
        }
        for gear in Gear::ALL {
            if !size.is_some_and(|size| gear.fits(ship.kind, size)) {
                continue;
            }
            let price = self.economy.gear_price(gear);
            let fitted = ship.gear.contains(&gear);
            let blocked = if fitted {
                Some(format!("{} already fitted.", gear.label()))
            } else {
                (price > self.economy.credits)
                    .then(|| format!("Costs {price} cr; you have {} cr.", self.economy.credits))
            };
            let label = if fitted {
                format!("{} ✓", gear.label())
            } else {
                format!("{} · {price} cr", gear.label())
            };
            if icons::labeled_action(ui, gear_icon(gear), &label, blocked)
                .on_hover_text(gear.summary())
                .clicked()
            {
                action = Some(ClientCommand::FitGear {
                    ship: ship.id,
                    gear,
                    expected_revision: self.state.revision,
                });
            }
        }
        action
    }

    /// The one-time cargo hold upgrade for this ship.
    fn hold_upgrade_button(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let capacity = self.economy.rules.hold_upgrade_capacity;
        let price = self.economy.hold_upgrade_price();
        let (label, blocked) = if ship.upgraded {
            (
                format!("Hold +{capacity} ✓"),
                Some("Hold already upgraded.".to_owned()),
            )
        } else {
            (format!("Hold +{capacity} · {price} cr"), None)
        };
        icons::labeled_action(ui, Icon::Upgrade, &label, blocked)
            .on_hover_text(format!(
                "Upgrade this ship's hold: +{capacity} cargo for {price} cr"
            ))
            .clicked()
            .then_some(ClientCommand::UpgradeHold {
                ship: ship.id,
                expected_revision: self.state.revision,
            })
    }

    fn escort_button(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let fleet = self.state.fleet_for_ship(ship.id).ok();
        let runs = fleet.map_or(0, |fleet| self.state.navigation.escort_runs(fleet.id));
        let label = match runs {
            0 => format!("Hire escort · {} cr", self.economy.escort_price()),
            runs => format!(
                "Renew escort ({runs} runs left) · {} cr",
                self.economy.escort_price()
            ),
        };
        let mut action = None;
        if icons::labeled_action(ui, Icon::Weapon, &label, None)
            .on_hover_text(format!(
                "A patrol ship joins this fleet's battles for {ESCORT_RUNS} runs; \
                 the contract ends if it is destroyed."
            ))
            .clicked()
        {
            action = Some(ClientCommand::HireEscort {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
        if runs > 0 {
            icons::glyph(
                ui,
                Icon::Weapon,
                GREEN,
                &format!("Escorted: {runs} runs left"),
            );
        }
        action
    }
}
