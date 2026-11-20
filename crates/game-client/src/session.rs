//! Native session adapter. Course planning is separate from shared navigation runs.
mod advance;
mod battle;
mod battle_cards;
mod battle_controls;
#[cfg(any(test, feature = "dev-tools"))]
mod capture;
mod combat_report;
mod fleets;
pub use fleets::TradeOption;
mod guide;
mod landing;
mod map_controls;
mod market;
mod market_table;
mod motion;
mod navigation;
mod panel;
mod routes;
mod runs;
mod saves;
mod settings;
mod shell;
pub use shell::SheetTab;
mod shipyard;
use game_core::economy::Economy;
use game_core::{
    SystemId,
    movement::{KM_PER_LIGHT_YEAR, SpacePosition},
    navigation::NavigationEvent,
    travel::TravelState,
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
use std::collections::VecDeque;

/// Galaxy-space kilometers to the light-year coordinates the galaxy map draws in.
pub(crate) fn light_years(position: SpacePosition) -> glam::DVec3 {
    glam::DVec3::new(
        position.x_km as f64,
        position.y_km as f64,
        position.z_km as f64,
    ) / KM_PER_LIGHT_YEAR as f64
}

pub struct LocalSession {
    map_target: Option<map_controls::MapTarget>,
    focus_requested: bool,
    /// Sheet page the map should switch to (e.g. after Advance stops).
    sheet_request: Option<SheetTab>,
    /// Present while "Advance to next event" is repeating navigation runs.
    advance: Option<advance::Advance>,
    /// The latest run's display glide, until it has played out.
    motion: Option<motion::Motion>,
    /// Latest UI clock reading, in seconds.
    clock: f64,
    /// Glide length in seconds; 0 when the UI has animations off.
    glide_seconds: f64,
    authority: game_server::Session,
    state: TravelState,
    active_ship: game_core::travel::ShipId,
    economy: Economy,
    market: market::MarketPanel,
    navigation: navigation::NavigationPanel,
    fleet_panel: fleets::FleetPanel,
    combat_report: combat_report::CombatReportPanel,
    battle: battle::BattlePanel,
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
        let combat_report = combat_report::CombatReportPanel::seen(&state);
        Self {
            map_target: None,
            focus_requested: false,
            sheet_request: None,
            advance: None,
            motion: None,
            clock: 0.0,
            glide_seconds: motion::GLIDE_SECONDS,
            authority,
            active_ship,
            state: *state,
            economy: *economy,
            market: market::MarketPanel::default(),
            navigation: navigation::NavigationPanel::default(),
            fleet_panel: fleets::FleetPanel::default(),
            combat_report,
            battle: battle::BattlePanel::default(),
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

    /// Ships that the authoritative economy lets trade at their current location.
    pub fn trade_ready_ships(&self) -> Vec<game_core::travel::ShipId> {
        self.state
            .ships
            .iter()
            .filter(|ship| self.economy.current_market(ship).is_ok())
            .map(|ship| ship.id)
            .collect()
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
            .map(light_years)
    }

    /// Where the galaxy map draws an own ship: like `ship_position`, but gliding
    /// after each run. Camera-follow logic keeps using `ship_position`.
    pub fn display_ship_position(
        &self,
        ship: game_core::travel::ShipId,
        systems: &[game_core::StarSystem],
    ) -> Option<glam::DVec3> {
        self.display_position(ship, systems).map(light_years)
    }

    /// Own ships as the system map draws them: in-transit ships, and resting ships
    /// still gliding in from the latest run, become free-flying local positions.
    pub fn ships_for_map(
        &self,
        system: &game_core::StarSystem,
        systems: &[game_core::StarSystem],
    ) -> Vec<game_core::travel::Ship> {
        let mut ships = self.state.ships.clone();
        let Ok(center) = SpacePosition::from_galaxy(system.position) else {
            return ships;
        };
        let gliding = self.gliding();
        for ship in &mut ships {
            let in_transit = matches!(ship.location, game_core::travel::Location::InTransit(_));
            if !in_transit && !gliding {
                continue;
            }
            let Some(position) = self.display_position(ship.id, systems) else {
                continue;
            };
            // A resting ship keeps its berth marker unless it is still gliding in.
            if in_transit || Some(position) != self.state.navigation_position(ship.id, systems) {
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

    /// The latest message shown to the player.
    #[cfg(test)]
    pub fn notice(&self) -> &str {
        &self.notice
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
                let before = std::mem::replace(&mut self.state, *state);
                let economy_before = std::mem::replace(&mut self.economy, *economy);
                if events.iter().any(|event| {
                    matches!(
                        event,
                        SessionEvent::Navigation(NavigationEvent::RunExecuted(_))
                    )
                }) {
                    self.motion = Some(motion::Motion::new(before, economy_before.traders));
                }
                if self.state.ship(self.active_ship).is_err() {
                    self.active_ship = self.state.ships[0].id;
                }
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
                        SessionEvent::Repaired { ship, cost } => {
                            format!("Repaired ship {} for {cost} cr.", ship.0)
                        }
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
mod advance_tests;
#[cfg(test)]
mod motion_tests;
#[cfg(test)]
mod tests;
