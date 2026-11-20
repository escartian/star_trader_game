//! Unlocks (see `game_core::progress`): a new game starts as a captain, and
//! after every command the host records the features play has reached, as
//! `Unlocked` events the client turns into guide cards.
use super::Session;
use game_core::{
    campaign,
    progress::{self, Progress},
    travel::PlayerId,
};
use game_protocol::{ServerMessage, SessionEvent};

impl Session {
    /// Starts unlocks from the beginning: only trading and travel at first.
    /// New player games call this; tests and captures keep everything open.
    pub fn begin_progression(&mut self) {
        let start = campaign::net_worth(&self.state, &self.economy, super::LOCAL_PLAYER);
        self.state.progress = Progress::captain(start);
    }

    /// Records newly reached features and adds their events to `message`.
    pub(super) fn record_unlocks(
        &mut self,
        player: PlayerId,
        message: ServerMessage,
    ) -> ServerMessage {
        let found = progress::newly_unlocked(&self.state, &self.economy, &self.systems, player);
        if found.is_empty() {
            return message;
        }
        self.state.progress.unlocked.extend(&found);
        let ServerMessage::SessionSnapshot { mut events, .. } = message else {
            return message;
        };
        events.extend(
            found
                .into_iter()
                .map(|feature| SessionEvent::Unlocked { feature }),
        );
        self.snapshot(events)
    }
}
