//! Offline action adapter. Only successful movement advances simulation ticks.
mod fleets;
mod guide;
mod market;
mod panel;
mod saves;
mod settings;
use game_core::economy::Economy;
use game_core::{
    SystemId,
    travel::{Location, TravelState},
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
use std::collections::VecDeque;

pub struct LocalSession {
    authority: game_server::Session,
    state: TravelState,
    active_ship: game_core::travel::ShipId,
    economy: Economy,
    market: market::MarketPanel,
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
            authority,
            active_ship,
            state,
            economy,
            market: market::MarketPanel::default(),
            fleet_panel: fleets::FleetPanel::default(),
            saves: saves::SavePanel::default(),
            log: VecDeque::new(),
            loaded_galaxy: None,
            notice: String::new(),
            preferences_path: None,
        }
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

    pub fn origin(&self) -> SystemId {
        match &self.selected_ship().location {
            Location::Docked(system) => *system,
            Location::Travelling(journey) => journey.origin,
        }
    }

    pub fn display_system(&self) -> SystemId {
        match &self.selected_ship().location {
            Location::Docked(system) => *system,
            Location::Travelling(journey) => journey.destination,
        }
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
                self.economy = economy;
                for event in events {
                    let message = match event {
                        SessionEvent::Departed { .. } => "Departure confirmed.".into(),
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
