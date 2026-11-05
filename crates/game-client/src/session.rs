//! Offline host adapter: schedules ticks and sends commands through the authority.
mod guide;
mod market;
mod panel;
mod saves;
use game_core::economy::Economy;
use game_core::{
    SystemId,
    travel::{Location, TravelState},
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct LocalSession {
    authority: game_server::Session,
    state: TravelState,
    economy: Economy,
    market: market::MarketPanel,
    saves: saves::SavePanel,
    log: VecDeque<String>,
    loaded_galaxy: Option<Vec<game_core::StarSystem>>,
    clock: Instant,
    notice: String,
}

impl LocalSession {
    pub fn new(mut authority: game_server::Session) -> Self {
        let ServerMessage::SessionSnapshot { state, economy, .. } =
            authority.handle(ClientCommand::GetSession)
        else {
            unreachable!("a fresh local session must return its own snapshot")
        };
        Self {
            authority,
            state,
            economy,
            market: market::MarketPanel::default(),
            saves: saves::SavePanel::default(),
            log: VecDeque::new(),
            loaded_galaxy: None,
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

    pub fn display_system(&self) -> SystemId {
        match &self.state.ship.location {
            Location::Docked(system) => *system,
            Location::Travelling(journey) => journey.destination,
        }
    }

    /// Called on suspend/resume so background time never becomes simulation time.
    pub fn reset_clock(&mut self) {
        self.clock = Instant::now();
    }

    pub fn update_clock(&mut self, ctx: &egui::Context) {
        if self.saves.paused {
            self.reset_clock();
        } else {
            self.advance_to(Instant::now());
        }
        // Idle simulation catches up on the next input without a continuous redraw loop.
        if !self.saves.paused && matches!(self.state.ship.location, Location::Travelling(_)) {
            ctx.request_repaint_after(self.tick_interval().saturating_sub(self.clock.elapsed()));
        }
    }

    fn advance_to(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.clock);
        let interval = self.tick_interval();
        let ticks = (elapsed.as_nanos() / interval.as_nanos()).min(u128::from(u64::MAX)) as u64;
        if ticks > 0 {
            let remainder = (elapsed.as_nanos() % interval.as_nanos()) as u64;
            self.clock = now - Duration::from_nanos(remainder);
            let message = self.authority.advance(ticks);
            self.accept(message);
        }
    }

    fn tick_interval(&self) -> Duration {
        Duration::from_millis(1000 / u64::from(self.saves.speed))
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
        let message = self.authority.handle(command);
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
                self.economy = economy;
                for event in events {
                    let message = match event {
                        SessionEvent::Departed { .. } => {
                            "Departure confirmed. Travel advances automatically.".into()
                        }
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
