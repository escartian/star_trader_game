//! Native session adapter. Course planning is separate from shared navigation runs.
mod fleets;
mod guide;
mod map_controls;
mod market;
mod navigation;
mod panel;
mod runs;
mod saves;
mod settings;
mod shipyard;
use game_core::economy::Economy;
use game_core::{SystemId, travel::TravelState};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
use std::collections::VecDeque;

pub struct LocalSession {
    map_target: Option<map_controls::MapTarget>,
    focus_requested: bool,
    authority: game_server::Session,
    state: TravelState,
    active_ship: game_core::travel::ShipId,
    economy: Economy,
    market: market::MarketPanel,
    navigation: navigation::NavigationPanel,
    fleet_panel: fleets::FleetPanel,
    saves: saves::SavePanel,
    log: VecDeque<String>,
    loaded_galaxy: Option<Vec<game_core::StarSystem>>,
    notice: String,
    preferences_path: Option<std::path::PathBuf>,
}

impl LocalSession {
    pub fn new(mut authority: game_server::Session) -> Self {
        let ServerMessage::SessionSnapshot { state, economy, .. } =
            authority.handle(ClientCommand::GetSession)
        else {
            unreachable!("a fresh local session must return its own snapshot")
        };
        let active_ship = state.ships[0].id;
        Self {
            map_target: None,
            focus_requested: false,
            authority,
            active_ship,
            state,
            economy: *economy,
            market: market::MarketPanel::default(),
            navigation: navigation::NavigationPanel::default(),
            fleet_panel: fleets::FleetPanel::default(),
            saves: saves::SavePanel::default(),
            log: VecDeque::new(),
            loaded_galaxy: None,
            notice: String::new(),
            preferences_path: None,
        }
    }

    pub fn system_half_extent_km(&self) -> i64 {
        self.state.movement_rules.system_half_extent_km
    }

    pub fn selected_ship(&self) -> &game_core::travel::Ship {
        self.state
            .ship(self.active_ship)
            .unwrap_or(&self.state.ships[0])
    }

    pub fn ships(&self) -> &[game_core::travel::Ship] {
        &self.state.ships
    }

    pub fn select_ship(&mut self, id: game_core::travel::ShipId) {
        if self.state.ship(id).is_ok() {
            self.active_ship = id;
        }
    }

    pub fn galaxy_position(&self, systems: &[game_core::StarSystem]) -> Option<glam::DVec3> {
        self.ship_position(self.active_ship, systems)
    }

    pub fn ship_position(
        &self,
        ship: game_core::travel::ShipId,
        systems: &[game_core::StarSystem],
    ) -> Option<glam::DVec3> {
        self.state
            .navigation_position(ship, systems)
            .map(|position| {
                glam::DVec3::new(
                    position.x_km as f64,
                    position.y_km as f64,
                    position.z_km as f64,
                ) / game_core::movement::KM_PER_LIGHT_YEAR as f64
            })
    }

    pub fn ships_for_map(
        &self,
        system: &game_core::StarSystem,
        systems: &[game_core::StarSystem],
    ) -> Vec<game_core::travel::Ship> {
        let mut ships = self.state.ships.clone();
        let Ok(center) = game_core::movement::SpacePosition::from_galaxy(system.position) else {
            return ships;
        };
        for ship in &mut ships {
            if matches!(ship.location, game_core::travel::Location::InTransit(_))
                && let Some(position) = self.state.navigation_position(ship.id, systems)
            {
                let local = game_core::LocalPosition {
                    x_km: position.x_km - center.x_km,
                    y_km: position.y_km - center.y_km,
                    z_km: position.z_km - center.z_km,
                };
                if [local.x_km, local.y_km, local.z_km].iter().all(|value| {
                    value.unsigned_abs() <= self.state.movement_rules.system_half_extent_km as u64
                }) {
                    ship.location = game_core::travel::Location::InSystem {
                        system: system.id,
                        position: local,
                    };
                }
            }
        }
        ships
    }

    pub fn display_system(&self) -> Option<SystemId> {
        self.current_system()
    }

    pub fn take_loaded_galaxy(&mut self) -> Option<Vec<game_core::StarSystem>> {
        self.loaded_galaxy.take()
    }

    pub fn open_market(&mut self) {
        self.market.open = true;
    }

    pub fn open_saves(&mut self) {
        self.saves.open = true;
    }

    pub fn notify(&mut self, message: String) {
        self.record(message);
    }

    fn record(&mut self, message: String) {
        self.notice = message.clone();
        self.log.push_back(message);
        if self.log.len() > 50 {
            self.log.pop_front();
        }
    }

    fn command(&mut self, command: ClientCommand) {
        let message = self.authority.handle_offline(command);
        self.accept(message);
    }

    fn accept(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::SessionSnapshot {
                state,
                economy,
                events,
            } => {
                self.state = state;
                if self.state.ship(self.active_ship).is_err() {
                    self.active_ship = self.state.ships[0].id;
                }
                self.economy = *economy;
                for event in events {
                    let message = match event {
                        SessionEvent::Navigation(event) => runs::event_label(event),
                        SessionEvent::ShipSold { ship, price } => {
                            format!("Sold ship {} for {price} cr.", ship.0)
                        }
                        SessionEvent::FleetRenamed { fleet } => {
                            format!("Renamed fleet {}.", fleet.0)
                        }
                        SessionEvent::ShipPurchased { ship } => {
                            self.active_ship = ship;
                            format!("Purchased {}.", self.selected_ship().name)
                        }
                        SessionEvent::FleetSplit { new_fleet, .. } => {
                            format!("Created Fleet {} from selected ships.", new_fleet.0)
                        }
                        SessionEvent::FleetsMerged { source, target } => {
                            format!("Merged Fleet {} into Fleet {}.", source.0, target.0)
                        }
                        SessionEvent::CargoTransferred {
                            source,
                            target,
                            quantity,
                        } => format!(
                            "Transferred {quantity} cargo from ship {} to ship {}.",
                            source.0, target.0
                        ),
                        SessionEvent::Arrived { .. } => "Arrival confirmed.".into(),
                        SessionEvent::Traded(quote) => format!(
                            "{:?}: {} {} for {} credits.",
                            quote.side,
                            quote.quantity,
                            quote.commodity.label(),
                            quote.total
                        ),
                        SessionEvent::Refuelled { units } => format!("Purchased {units} fuel."),
                        SessionEvent::HoldUpgraded => {
                            "Cargo hold upgraded. First earnings goal completed!".into()
                        }
                    };
                    self.record(message);
                }
            }
            ServerMessage::CommandRejected(error) => self.record(error.to_string()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
