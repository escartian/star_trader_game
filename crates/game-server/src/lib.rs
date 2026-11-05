//! Authoritative rules shared by offline hosting and future remote transports.
mod commerce;
mod fleets;
mod offline;
mod save;
use game_core::economy::Economy;
use game_core::{
    StarSystem, SystemId,
    travel::{Location, PlayerId, ShipId, TravelError, TravelQuote, TravelState},
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
        let systems = game_core::generation::generate_galaxy(seed);
        let economy = Economy::new(systems.iter().map(|system| system.id));
        let mut state = TravelState::default();
        state.ships[0].cargo_capacity = economy.rules.cargo_capacity;
        state.ships[0].base_cargo_capacity = economy.rules.cargo_capacity;
        Self {
            systems,
            economy,
            state,
        }
    }

    pub fn with_economy(
        seed: u64,
        rules: game_core::economy::EconomyRules,
    ) -> Result<Self, String> {
        rules.validate()?;
        let systems = game_core::generation::generate_galaxy(seed);
        let economy = Economy::with_rules(systems.iter().map(|system| system.id), rules)?;
        let mut state = TravelState::default();
        state.ships[0].cargo_capacity = economy.rules.cargo_capacity;
        state.ships[0].base_cargo_capacity = economy.rules.cargo_capacity;
        Ok(Self {
            systems,
            economy,
            state,
        })
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
        match command {
            ClientCommand::GetGalaxy => Ok(ServerMessage::GalaxySnapshot {
                protocol_version: PROTOCOL_VERSION,
                systems: self.systems.clone(),
            }),
            ClientCommand::ContinueTravel { .. } => Err(TravelError::OfflineMovementOnly.into()),
            command @ (ClientCommand::BuyShip { .. }
            | ClientCommand::SplitFleet { .. }
            | ClientCommand::MergeFleets { .. }
            | ClientCommand::TransferCargo { .. }) => self.fleet_command(player, command),
            ClientCommand::GetSession => Ok(self.snapshot(Vec::new())),
            ClientCommand::PreviewTravel { ship, destination } => Ok(ServerMessage::TravelPreview(
                self.quote(player, ship, destination)?,
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
            | ClientCommand::UpgradeHold { .. }) => self.commerce(player, command),
            ClientCommand::Travel {
                ship,
                destination,
                expected_revision,
            } => {
                if expected_revision != self.state.revision {
                    return Err(TravelError::StaleQuote.into());
                }
                let quotes = self.fleet_quotes(player, ship, destination)?;
                self.state.depart_many(&quotes)?;
                Ok(self.snapshot(vec![SessionEvent::Departed { destination }]))
            }
        }
    }

    fn quote(
        &self,
        player: PlayerId,
        ship: ShipId,
        destination: SystemId,
    ) -> Result<TravelQuote, CommandError> {
        self.fleet_quotes(player, ship, destination)?
            .into_iter()
            .find(|quote| quote.ship == ship)
            .ok_or_else(|| TravelError::UnknownShip.into())
    }

    fn fleet_quotes(
        &self,
        player: PlayerId,
        ship: ShipId,
        destination: SystemId,
    ) -> Result<Vec<TravelQuote>, CommandError> {
        self.state.authorize(player, ship)?;
        if matches!(self.state.ship(ship)?.location, Location::Travelling(_)) {
            return Err(TravelError::AlreadyTravelling.into());
        }
        let fleet = self.state.fleet_for_ship(ship)?;
        let origin = self.state.fleet_location(fleet.id, player)?;
        let position = |id| {
            self.systems
                .iter()
                .find(|system| system.id == id)
                .map(|system| system.position)
                .ok_or(TravelError::UnknownSystem)
        };
        let (origin, target) = (position(origin)?, position(destination)?);
        fleet
            .ships
            .iter()
            .map(|member| {
                self.state
                    .quote_for(*member, destination, origin, target)
                    .map_err(Into::into)
            })
            .collect()
    }

    /// Host-only simulation advancement. The offline adapter consumes movement ticks; a future
    /// shared-world turn resolver will call this after collecting player actions.
    /// This is deliberately absent from ClientCommand.
    pub fn advance(&mut self, ticks: u64) -> ServerMessage {
        match self.state.advance(ticks) {
            Ok(arrived) => {
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
            state: self.state.clone(),
            economy: self.economy.clone(),
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
