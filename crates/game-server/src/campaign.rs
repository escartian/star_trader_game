//! Campaign commands (charters, the Haven assault, the run limit) and the
//! end-of-game rules: once the campaign is lost or out of time only queries
//! run, and after every command the server records the first end reached.
use super::Session;
use game_core::{
    campaign,
    fleet::FleetId,
    navigation::NavigationError,
    travel::{PlayerId, TravelError},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

/// Longest campaign length a player may set, in runs.
pub const MAX_RUN_LIMIT: u64 = 100_000;

impl Session {
    /// Rejects every command but queries once the campaign is lost or out of time.
    pub(super) fn campaign_gate(&self, command: &ClientCommand) -> Result<(), CommandError> {
        let over = self.state.campaign.ended.is_some_and(|end| !end.playable());
        if over && !command.is_query() {
            return Err(NavigationError::CampaignOver.into());
        }
        Ok(())
    }

    /// Records the first campaign end this state reaches, refreshing the
    /// snapshot `message` carries so the client sees it at once.
    pub(super) fn record_campaign_end(
        &mut self,
        player: PlayerId,
        message: ServerMessage,
    ) -> ServerMessage {
        let Some(end) = campaign::check_end(&self.state, &self.economy, player) else {
            return message;
        };
        self.state.campaign.ended = Some(end);
        match message {
            ServerMessage::SessionSnapshot { events, .. } => self.snapshot(events),
            other => other,
        }
    }

    /// Fleets whose trade route runs between two ports `player` has chartered.
    pub(super) fn lane_fleets(&self, player: PlayerId) -> Vec<FleetId> {
        self.state
            .navigation
            .routes
            .iter()
            .filter(|route| self.economy.on_lane(player, route.stops))
            .map(|route| route.fleet)
            .collect()
    }

    pub(super) fn campaign_command(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let expected = match &command {
            ClientCommand::BuyCharter {
                expected_revision, ..
            }
            | ClientCommand::AssaultHaven {
                expected_revision, ..
            }
            | ClientCommand::SetRunLimit {
                expected_revision, ..
            } => *expected_revision,
            _ => unreachable!("only campaign commands reach this handler"),
        };
        if expected != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let event = match command {
            ClientCommand::BuyCharter { ship, .. } => {
                self.state.authorize(player, ship)?;
                let cost = self.economy.buy_charter(self.state.ship(ship)?)?;
                self.state.revision = revision;
                SessionEvent::CharterBought { cost }
            }
            ClientCommand::AssaultHaven { fleet, .. } => {
                let event = self.state.assault_haven(player, fleet)?;
                SessionEvent::Navigation(event)
            }
            ClientCommand::SetRunLimit { runs, .. } => {
                let run = self.state.navigation.run;
                if runs.is_some_and(|limit| limit <= run || limit > MAX_RUN_LIMIT) {
                    return Err(NavigationError::InvalidRunLimit.into());
                }
                self.state.campaign.run_limit = runs;
                self.state.revision = revision;
                SessionEvent::RunLimitSet { runs }
            }
            _ => unreachable!("command was matched above"),
        };
        Ok(self.snapshot(vec![event]))
    }
}
