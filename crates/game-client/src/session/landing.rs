//! Land / Lift off controls on a fleet card, and the mining hint for landed
//! fleets. Rules and costs live in `game_core::movement::LandingRules`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN};
use game_core::{
    BodyKind, StarSystem,
    fleet::FleetId,
    travel::{Location, Ship},
};
use game_protocol::ClientCommand;

impl LocalSession {
    pub(super) fn landing_controls(
        &mut self,
        ui: &mut egui::Ui,
        fleet: FleetId,
        systems: &[StarSystem],
    ) {
        let Some(ships) = self.fleet_ships(fleet) else {
            return;
        };
        let location = ships[0].location.clone();
        let (system, body, landed) = match location {
            Location::Orbiting { system, body } => (system, body, false),
            Location::Landed { system, body } => (system, body, true),
            _ => return,
        };
        let Some(target) = systems
            .iter()
            .find(|known| known.id == system)
            .and_then(|known| known.bodies.iter().find(|known| known.id == body))
        else {
            return;
        };
        let landing = &self.state.movement_rules.landing;
        let Some(fuel) = landing.fuel(target.kind) else {
            return;
        };
        let ore = landing.ore_per_ship(target.kind);
        let busy = self.state.navigation.order(fleet).is_some()
            || self
                .state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.fleet == fleet);
        let short = ships.iter().any(|ship| ship.fuel < fuel);
        let blocked = if busy {
            Some("Cancel the course or resolve the contact first.".to_owned())
        } else if short {
            Some(format!("Each ship needs {fuel} fuel."))
        } else {
            None
        };
        let cost = match fuel {
            0 => "no fuel".to_owned(),
            fuel => format!("{fuel} fuel per ship"),
        };
        let (icon, label, hover, command) = if landed {
            (
                Icon::Orbit,
                "Lift off",
                format!("Return to orbit: {cost}. Plotting a course lifts off automatically."),
                ClientCommand::LiftOff {
                    fleet,
                    expected_revision: self.state.revision,
                },
            )
        } else {
            let mining = match (target.kind, ore) {
                (BodyKind::Moon | BodyKind::Asteroid, ore) if ore > 0 => {
                    format!(" While waiting there, each ship mines {ore} ore per run.")
                }
                _ => String::new(),
            };
            (
                Icon::Dock,
                "Land",
                format!("Set down on {}: {cost}.{mining}", target.name),
                ClientCommand::Land {
                    fleet,
                    expected_revision: self.state.revision,
                },
            )
        };
        if icons::labeled_action(ui, icon, label, blocked)
            .on_hover_text(hover)
            .clicked()
        {
            self.command(command);
        }
        if landed && ore > 0 {
            let waiting = self.state.navigation.holds.contains(&fleet);
            let (color, text) = if waiting {
                (GREEN, format!("Mining {ore} ore per ship each run."))
            } else {
                (AMBER, "Wait here to mine ore each run.".to_owned())
            };
            ui.colored_label(color, egui::RichText::new(text).small());
        }
    }

    /// Landed on a body that yields ore (whether or not holds have room).
    pub(super) fn mining(&self, fleet: FleetId, systems: &[StarSystem]) -> bool {
        let Some(ships) = self.fleet_ships(fleet) else {
            return false;
        };
        let Location::Landed { system, body } = ships[0].location else {
            return false;
        };
        systems
            .iter()
            .find(|known| known.id == system)
            .and_then(|known| known.bodies.iter().find(|known| known.id == body))
            .is_some_and(|body| self.state.movement_rules.landing.ore_per_ship(body.kind) > 0)
    }

    /// Waiting fleets landed where ore is mined that still have cargo room:
    /// Advance keeps running for them.
    pub(super) fn mining_with_room(&self) -> Vec<FleetId> {
        let systems = self.authority.systems();
        self.state
            .fleets
            .iter()
            .filter(|fleet| self.state.navigation.holds.contains(&fleet.id))
            .filter(|fleet| self.mining(fleet.id, systems))
            .filter(|fleet| {
                fleet.ships.iter().any(|id| {
                    self.state
                        .ship(*id)
                        .is_ok_and(|ship| ship.cargo_used() < u64::from(ship.cargo_capacity))
                })
            })
            .map(|fleet| fleet.id)
            .collect()
    }

    fn fleet_ships(&self, fleet: FleetId) -> Option<Vec<Ship>> {
        let record = self.state.fleets.iter().find(|record| record.id == fleet)?;
        let ships: Vec<Ship> = record
            .ships
            .iter()
            .filter_map(|id| self.state.ship(*id).ok().cloned())
            .collect();
        (!ships.is_empty()).then_some(ships)
    }
}
