//! Solo navigation-run commands. Multiplayer readiness will wrap this same resolver.
use super::Session;
use game_core::{
    navigation::NavigationEvent,
    travel::{PlayerId, TravelError},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn navigation_command(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let revision = match &command {
            ClientCommand::Move {
                expected_revision, ..
            }
            | ClientCommand::ExecuteNavigationRun { expected_revision }
            | ClientCommand::HoldFleet {
                expected_revision, ..
            }
            | ClientCommand::CancelCourse {
                expected_revision, ..
            }
            | ClientCommand::ResolveEncounter {
                expected_revision, ..
            } => *expected_revision,
            _ => unreachable!("only navigation commands reach this handler"),
        };
        if revision != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let mut candidate = self.clone();
        let events = match command {
            ClientCommand::Move { ship, target, .. } => {
                let fleet =
                    candidate
                        .state
                        .plot_course(player, ship, target, &candidate.systems)?;
                vec![NavigationEvent::CoursePlotted(fleet)]
            }
            ClientCommand::HoldFleet { fleet, .. } => {
                candidate.state.hold_fleet(player, fleet)?;
                vec![NavigationEvent::Holding(fleet)]
            }
            ClientCommand::CancelCourse { fleet, .. } => {
                candidate.state.cancel_course(player, fleet)?;
                vec![NavigationEvent::CourseCancelled(fleet)]
            }
            ClientCommand::ResolveEncounter {
                encounter, choice, ..
            } => vec![
                candidate
                    .state
                    .resolve_encounter(player, encounter, choice)?,
            ],
            ClientCommand::ExecuteNavigationRun { .. } => {
                let events = candidate
                    .state
                    .execute_navigation_run(player, &candidate.systems)?;
                candidate
                    .economy
                    .advance(candidate.state.movement_rules.ticks_per_navigation_run);
                events
            }
            _ => unreachable!(),
        };
        *self = candidate;
        Ok(self.snapshot(events.into_iter().map(SessionEvent::Navigation).collect()))
    }
}
