//! Offline host adapter: schedules ticks and sends commands through the authority.
mod panel;
use game_core::{
    SystemId,
    travel::{Location, TravelState},
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
use std::time::{Duration, Instant};

const TICK_INTERVAL: Duration = Duration::from_secs(1);

pub struct LocalSession {
    authority: game_server::Session,
    state: TravelState,
    clock: Instant,
    notice: String,
}

impl LocalSession {
    pub fn new(mut authority: game_server::Session) -> Self {
        let ServerMessage::SessionSnapshot { state, .. } =
            authority.handle(ClientCommand::GetSession)
        else {
            unreachable!("a fresh local session must return its own snapshot")
        };
        Self {
            authority,
            state,
            clock: Instant::now(),
            notice: String::new(),
        }
    }

    pub fn origin(&self) -> SystemId {
        match &self.state.ship.location {
            Location::Docked(system) => *system,
            Location::Travelling(journey) => journey.origin,
        }
    }

    /// Called on suspend/resume so background time never becomes simulation time.
    pub fn reset_clock(&mut self) {
        self.clock = Instant::now();
    }

    pub fn update_clock(&mut self, ctx: &egui::Context) {
        self.advance_to(Instant::now());
        // Idle simulation catches up on the next input without a continuous redraw loop.
        if matches!(self.state.ship.location, Location::Travelling(_)) {
            ctx.request_repaint_after(TICK_INTERVAL.saturating_sub(self.clock.elapsed()));
        }
    }

    fn advance_to(&mut self, now: Instant) {
        let ticks = now.saturating_duration_since(self.clock).as_secs();
        if ticks > 0 {
            self.clock += Duration::from_secs(ticks);
            let message = self.authority.advance(ticks);
            self.accept(message);
        }
    }

    fn accept(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::SessionSnapshot { state, events } => {
                self.state = state;
                for event in events {
                    self.notice = match event {
                        SessionEvent::Departed { .. } => {
                            "Departure confirmed. Travel advances automatically.".into()
                        }
                        SessionEvent::Arrived { .. } => "Arrival confirmed.".into(),
                    };
                }
            }
            ServerMessage::CommandRejected(error) => self.notice = error.to_string(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
