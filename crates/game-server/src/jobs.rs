//! Job commands (accept, abandon) and the after-command job pass: boards
//! refresh every few runs, every active job is settled, paying or failing
//! each once with a `JobFinished` event, and factions offer story quest
//! steps your standing has opened (`QuestOffered`; see `game_core::jobs`).
use super::Session;
use game_core::{
    jobs,
    travel::{PlayerId, TravelError},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn job_command(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let expected = match &command {
            ClientCommand::AcceptJob {
                expected_revision, ..
            }
            | ClientCommand::AbandonJob {
                expected_revision, ..
            } => *expected_revision,
            _ => unreachable!("only job commands reach this handler"),
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
            ClientCommand::AcceptJob { job, ship, .. } => {
                self.state.authorize(player, ship)?;
                let at = self
                    .economy
                    .current_market(self.state.ship(ship)?)
                    .ok()
                    .map(|market| market.id);
                let max = self.economy.rules.job_catalog.max_active as usize;
                let run = self.state.navigation.run;
                let taken = self.state.jobs.accept(job, at, max, run)?;
                SessionEvent::JobAccepted {
                    job,
                    title: taken.title.clone(),
                }
            }
            ClientCommand::AbandonJob { job, .. } => {
                let outcome = jobs::abandon(&mut self.state, &mut self.economy, job)
                    .ok_or(game_core::jobs::JobError::UnknownJob)?;
                SessionEvent::JobFinished(outcome)
            }
            _ => unreachable!("command was matched above"),
        };
        self.state.revision = revision;
        Ok(self.snapshot(vec![event]))
    }

    /// Refreshes the boards when due and settles active jobs, adding a
    /// `JobFinished` event for each that ended. Skipped once the game is over.
    pub(super) fn record_jobs(
        &mut self,
        player: PlayerId,
        message: ServerMessage,
    ) -> ServerMessage {
        if self.state.campaign.ended.is_some_and(|end| !end.playable()) {
            return message;
        }
        let refreshed = jobs::refresh_offers(&mut self.state, &self.economy, &self.systems);
        let outcomes = jobs::settle(&mut self.state, &mut self.economy, player);
        // Standing just earned may open the next story step at once.
        let offered = jobs::quests::offer(&mut self.state, &self.economy, &self.systems);
        if !refreshed && outcomes.is_empty() && offered.is_empty() {
            return message;
        }
        let ServerMessage::SessionSnapshot { mut events, .. } = message else {
            return message;
        };
        events.extend(outcomes.into_iter().map(SessionEvent::JobFinished));
        events.extend(
            offered
                .into_iter()
                .map(|job| SessionEvent::QuestOffered { job }),
        );
        self.snapshot(events)
    }
}
