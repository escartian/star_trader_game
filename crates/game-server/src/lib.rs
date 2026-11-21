//! Authoritative rules shared by offline hosting and future remote transports.
mod automation;
mod campaign;
mod commerce;
mod fleets;
mod movement;
mod navigation;
mod offline;
mod progress;
mod save;
mod setup;
pub use setup::{DangerLevel, GameSetup, InflationLevel, STARTING_CREDITS, WEALTH_SHARES};
#[cfg(any(test, feature = "dev-tools"))]
mod scenarios;
mod shipyard;
use game_core::economy::Economy;
use game_core::{
    StarSystem,
    travel::{PlayerId, TravelState},
};
use game_protocol::{ClientCommand, CommandError, PROTOCOL_VERSION, ServerMessage, SessionEvent};

pub const LOCAL_PLAYER: PlayerId = PlayerId(1);

#[derive(Clone)]
pub struct Session {
    systems: Vec<StarSystem>,
    state: TravelState,
    economy: Economy,
}

impl Default for Session {
    fn default() -> Self {
        Self::with_seed(0)
    }
}

impl Session {
    pub fn with_seed(seed: u64) -> Self {
        Self::with_economy(seed, game_core::economy::EconomyRules::default())
            .expect("embedded world/economy definitions must validate")
    }

    pub fn with_economy(
        seed: u64,
        rules: game_core::economy::EconomyRules,
    ) -> Result<Self, String> {
        rules.validate()?;
        let systems = game_core::generation::generate_galaxy(seed);
        Self::from_generated(seed, rules, systems, Default::default())
    }

    pub fn with_galaxy(
        seed: u64,
        rules: game_core::economy::EconomyRules,
        config: game_core::generation::GalaxyConfig,
    ) -> Result<Self, String> {
        rules.validate()?;
        let systems = game_core::generation::generate_galaxy_with(seed, config)?;
        Self::from_generated(seed, rules, systems, Default::default())
    }

    /// Builds the world; movement rules (contact chance, band size) apply
    /// before NPC traders and raider bands are placed.
    fn from_generated(
        seed: u64,
        rules: game_core::economy::EconomyRules,
        mut systems: Vec<StarSystem>,
        movement: game_core::movement::MovementRules,
    ) -> Result<Self, String> {
        rules.planet_catalog.populate(&mut systems);
        movement
            .validate_world(&systems)
            .map_err(|error| error.to_string())?;
        let mut economy = Economy::with_rules(&systems, rules)?;
        let mut state = TravelState {
            movement_rules: movement,
            ..TravelState::default()
        };
        state.navigation.rng_state = seed;
        state.ships[0].cargo_capacity = economy.rules.cargo_capacity;
        state.ships[0].base_cargo_capacity = economy.rules.cargo_capacity;
        economy.spawn_traders(&systems, &state.movement_rules);
        state.spawn_raider_bands(&systems);
        Ok(Self {
            systems,
            economy,
            state,
        })
    }

    /// The galaxy this session plays in (public knowledge, never hidden state).
    pub fn systems(&self) -> &[StarSystem] {
        &self.systems
    }

    /// Local identity convenience adapter. A remote adapter must resolve authenticated identity.
    pub fn handle(&mut self, command: ClientCommand) -> ServerMessage {
        self.handle_as(LOCAL_PLAYER, command)
    }

    /// `player` must come from the host/connection, never from command payloads.
    pub fn handle_as(&mut self, player: PlayerId, command: ClientCommand) -> ServerMessage {
        match self.execute(player, command) {
            Ok(message) => message,
            Err(error) => ServerMessage::CommandRejected(error),
        }
    }

    fn execute(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        // The initial world has one player; don't disclose its state to another identity.
        self.state.authorize(player, self.state.ships[0].id)?;
        self.campaign_gate(&command)?;
        // Every rule reads body positions for the current tick.
        game_core::orbits::place(&mut self.systems, self.state.tick);
        let message = self.dispatch(player, command)?;
        let message = self.record_unlocks(player, message);
        Ok(self.record_campaign_end(player, message))
    }

    fn dispatch(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        match command {
            ClientCommand::GetGalaxy => Ok(ServerMessage::GalaxySnapshot {
                protocol_version: PROTOCOL_VERSION,
                systems: self.systems.clone(),
            }),
            command @ (ClientCommand::Move { .. }
            | ClientCommand::PlotDestination { .. }
            | ClientCommand::ExecuteNavigationRun { .. }
            | ClientCommand::HoldFleet { .. }
            | ClientCommand::CancelCourse { .. }
            | ClientCommand::ResolveEncounter { .. }
            | ClientCommand::SetTradeRoute { .. }
            | ClientCommand::CancelTradeRoute { .. }
            | ClientCommand::BattleAction { .. }
            | ClientCommand::Land { .. }
            | ClientCommand::LiftOff { .. }
            | ClientCommand::SetFlagship { .. }
            | ClientCommand::SetPatrol { .. }
            | ClientCommand::HailTrader { .. }) => self.navigation_command(player, command),
            command @ (ClientCommand::BuyShip { .. }
            | ClientCommand::SellShip { .. }
            | ClientCommand::RenameFleet { .. }
            | ClientCommand::SplitFleet { .. }
            | ClientCommand::MergeFleets { .. }
            | ClientCommand::TransferCargo { .. }) => self.fleet_command(player, command),
            ClientCommand::GetSession => Ok(self.snapshot(Vec::new())),
            ClientCommand::PreviewDestination { ship, system, body } => {
                Ok(ServerMessage::DestinationPreview(
                    self.state
                        .plan_destination(player, ship, system, body, &self.systems)?,
                ))
            }
            ClientCommand::PreviewMove { ship, target } => Ok(ServerMessage::MovementPreview(
                self.preview_movement(player, ship, &target)?,
            )),
            ClientCommand::PreviewTrade {
                ship,
                commodity,
                quantity,
                side,
            } => {
                self.state.authorize(player, ship)?;
                Ok(ServerMessage::TradePreview(self.economy.quote(
                    self.state.ship(ship)?,
                    commodity,
                    quantity,
                    side,
                    self.state.revision,
                )?))
            }
            command @ (ClientCommand::Trade { .. }
            | ClientCommand::Refuel { .. }
            | ClientCommand::UpgradeHold { .. }
            | ClientCommand::RepairShip { .. }) => self.commerce(player, command),
            command @ (ClientCommand::BuyCharter { .. }
            | ClientCommand::AssaultHaven { .. }
            | ClientCommand::SetRunLimit { .. }) => self.campaign_command(player, command),
        }
    }

    /// Host-only simulation advancement. The offline adapter consumes movement ticks; a future
    /// shared-world turn resolver will call this after collecting player actions.
    /// This is deliberately absent from ClientCommand.
    pub fn advance(&mut self, ticks: u64) -> ServerMessage {
        match self.state.advance(ticks) {
            Ok(arrived) => {
                game_core::orbits::place(&mut self.systems, self.state.tick);
                self.economy.advance(ticks);
                self.snapshot(
                    arrived
                        .into_iter()
                        .map(|(ship, system)| SessionEvent::Arrived { system, ship })
                        .collect(),
                )
            }
            Err(error) => ServerMessage::CommandRejected(error.into()),
        }
    }

    fn snapshot(&self, events: Vec<SessionEvent>) -> ServerMessage {
        ServerMessage::SessionSnapshot {
            state: Box::new(self.state.clone()),
            economy: Box::new(self.economy.clone()),
            events,
        }
    }
}

#[cfg(test)]
mod commerce_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod fleet_tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod movement_tests;

#[cfg(test)]
mod planet_tests;

#[cfg(test)]
mod band_tests;
#[cfg(test)]
mod battle_tests;
#[cfg(test)]
mod campaign_tests;
#[cfg(test)]
mod combat_tests;
#[cfg(test)]
mod contact_tests;
#[cfg(test)]
mod encounter_tests;
#[cfg(test)]
mod flagship_tests;
#[cfg(test)]
mod landing_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod orbit_tests;
#[cfg(test)]
mod raid_tests;
#[cfg(test)]
mod setup_tests;

#[cfg(test)]
mod npc_tests;

#[cfg(test)]
mod route_tests;

#[cfg(test)]
mod trade_route_tests;

#[cfg(test)]
mod piracy_tests;
