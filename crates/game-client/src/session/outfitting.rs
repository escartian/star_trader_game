//! The Market window's anti-piracy row: fit afterburners or point defense to
//! the selected ship, and hire (or renew) an escort for its fleet. Rules live
//! in `game_core::combat::gear`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN};
use game_core::{
    combat::gear::{ESCORT_RUNS, Gear},
    travel::Ship,
};
use game_protocol::ClientCommand;

impl LocalSession {
    /// Gear and escort buttons for `ship`; returns the command to send.
    pub(super) fn outfitting_row(&self, ui: &mut egui::Ui, ship: &Ship) -> Option<ClientCommand> {
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("ANTI-PIRACY").small().color(AMBER));
            for gear in Gear::ALL {
                let price = self.economy.gear_price(gear);
                let fitted = ship.gear.contains(&gear);
                let blocked = fitted.then(|| format!("{} already fitted.", gear.label()));
                let icon = match gear {
                    Gear::Afterburners => Icon::Engine,
                    Gear::PointDefense => Icon::Shield,
                };
                let label = if fitted {
                    format!("{} ✓", gear.label())
                } else {
                    format!("{} · {price} cr", gear.label())
                };
                if icons::labeled_action(ui, icon, &label, blocked)
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
            let fleet = self.state.fleet_for_ship(ship.id).ok();
            let runs = fleet.map_or(0, |fleet| self.state.navigation.escort_runs(fleet.id));
            let label = match runs {
                0 => format!("Hire escort · {} cr", self.economy.escort_price()),
                runs => format!(
                    "Renew escort ({runs} runs left) · {} cr",
                    self.economy.escort_price()
                ),
            };
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
        });
        action
    }
}
