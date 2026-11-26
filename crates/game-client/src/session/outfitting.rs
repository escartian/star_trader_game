//! The Market window's ship upgrades: fit universal, model and size gear to
//! any of the fleet's ships at this port (each row lists what fits that
//! ship), and hire (or renew) an escort for the fleet. Rules live in
//! `game_core::combat::gear`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN};
use game_core::{
    combat::gear::{ESCORT_RUNS, Gear},
    fleet_trade::{self, LoadOrder},
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
    /// Upgrade rows for every fleet ship at `ship`'s port (`ship` first), then
    /// the escort; returns the command to send.
    pub(super) fn outfitting_row(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let mut action = None;
        ui.label(egui::RichText::new("SHIP UPGRADES").small().color(AMBER))
            .on_hover_text(
                "Upgrades are fitted to one ship and stay with it. Each row lists what fits \
                 that ship's model and size.",
            );
        let mut here = fleet_trade::partners(&self.state, &self.economy, ship.id, LoadOrder::Even)
            .unwrap_or_else(|_| vec![ship.id]);
        here.sort_by_key(|id| *id != ship.id);
        for id in here {
            let Ok(member) = self.state.ship(id) else {
                continue;
            };
            if let Some(command) = self.ship_upgrades(ui, member) {
                action = Some(command);
            }
        }
        ui.horizontal_wrapped(|ui| {
            if let Some(command) = self.escort_button(ui, ship) {
                action = Some(command);
            }
        });
        action
    }

    /// One ship's row: its name, then a button per fitting upgrade.
    fn ship_upgrades(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let mut action = None;
        let size = self
            .economy
            .rules
            .ship_catalog
            .spec(ship.kind)
            .map(|spec| spec.combat.size);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(&ship.name).strong())
                .on_hover_text(ship.kind.label());
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
        });
        action
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
