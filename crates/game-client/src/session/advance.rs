//! Advance to next event: repeats ordinary navigation runs, one per short interval so
//! movement stays visible on the map, until something needs the player. Every step
//! is the same `ExecuteNavigationRun` command a manual run sends, so the authority,
//! saves and determinism are unchanged. Future multiplayer can treat an advance as
//! standing readiness that lapses on the same stop conditions.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use game_core::{fleet::FleetId, travel::Location};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

/// Pause between automatic runs, in seconds of UI time, so each step is visible.
pub(super) const RUN_INTERVAL_SECONDS: f64 = 0.35;
/// Safety net far above any single itinerary; reaching it pauses and reports.
const MAX_RUNS: u32 = 64;

/// Progress of the advance in progress.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Advance {
    runs: u32,
    /// UI clock (seconds) at which the latest automatic run executed.
    last_run_at: Option<f64>,
}

/// Why an advance stopped.
enum Stop {
    Contact(FleetId),
    NeedsOrders(FleetId),
    NothingUnderway,
    Rejected,
    Limit,
}

impl LocalSession {
    pub fn is_advancing(&self) -> bool {
        self.advance.is_some()
    }

    /// Starts advancing, or stops the advance in progress after its latest run.
    pub fn toggle_advance(&mut self) {
        if let Some(advance) = self.advance.take() {
            self.record(format!(
                "Advance stopped after {}.",
                runs_label(advance.runs)
            ));
        } else if self.state.navigation_attention(LOCAL_PLAYER).is_some() {
            // Nothing runs until every fleet has orders; this points at the one that lacks them.
            self.request_execute_run();
        } else {
            self.advance = Some(Advance::default());
        }
    }

    /// Call once per frame with the UI clock in seconds. Executes at most one run when
    /// one is due; returns the seconds until the next run while still advancing.
    pub fn drive_advance(&mut self, now: f64) -> Option<f64> {
        let advance = self.advance?;
        if let Some(last) = advance.last_run_at {
            let due = last + RUN_INTERVAL_SECONDS;
            if now < due {
                return Some(due - now);
            }
        }
        let runs = advance.runs.saturating_add(1);
        if let Some(stop) = self.advance_step(runs) {
            self.advance = None;
            self.report_stop(stop, runs);
            return None;
        }
        self.advance = Some(Advance {
            runs,
            last_run_at: Some(now),
        });
        Some(RUN_INTERVAL_SECONDS)
    }

    /// Selects a fleet and asks the map to focus it.
    pub(super) fn focus_fleet(&mut self, fleet: FleetId) {
        if let Some(ship) = self
            .state
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .and_then(|record| record.ships.first())
            .copied()
        {
            self.select_ship(ship);
        }
        self.focus_requested = true;
    }

    /// Advance (primary), single-run step and Stop, shared by the HUD and Navigation.
    pub(super) fn run_controls(&mut self, ui: &mut egui::Ui) {
        if self.is_advancing() {
            if icons::button(ui, Icon::Cancel, "STOP")
                .on_hover_text("Stop advancing; completed runs stay completed")
                .clicked()
            {
                self.toggle_advance();
            }
            ui.small(format!("Advancing · run {}", self.state.navigation.run));
            return;
        }
        if icons::button(ui, Icon::Execute, "ADVANCE")
            .on_hover_text(
                "Run until a fleet arrives, meets a contact or needs orders.\nEvery fleet needs a course or a Wait order first.",
            )
            .clicked()
        {
            self.toggle_advance();
        }
        let run = self.state.navigation.run.saturating_add(1);
        if icons::icon_button(ui, Icon::Clock, &format!("Execute run {run} only")).clicked() {
            self.request_execute_run();
        }
    }

    fn advance_step(&mut self, runs: u32) -> Option<Stop> {
        let before = self.state.navigation.run;
        self.command(ClientCommand::ExecuteNavigationRun {
            expected_revision: self.state.revision,
        });
        if self.state.navigation.run == before {
            return Some(Stop::Rejected);
        }
        if let Some(fleet) = self.state.navigation_attention(LOCAL_PLAYER) {
            return Some(if self.contact_fleets().contains(&fleet) {
                Stop::Contact(fleet)
            } else {
                Stop::NeedsOrders(fleet)
            });
        }
        if !self.anything_underway() {
            return Some(Stop::NothingUnderway);
        }
        (runs >= MAX_RUNS).then_some(Stop::Limit)
    }

    /// True while any own fleet has a course, including older saved journeys.
    fn anything_underway(&self) -> bool {
        self.state.fleets.iter().any(|fleet| {
            fleet.owner == LOCAL_PLAYER && self.state.navigation.order(fleet.id).is_some()
        }) || self.state.ships.iter().any(|ship| {
            ship.owner == LOCAL_PLAYER && matches!(ship.location, Location::Travelling(_))
        })
    }

    fn report_stop(&mut self, stop: Stop, runs: u32) {
        let advanced = runs_label(runs);
        let message = match stop {
            Stop::Contact(fleet) => {
                self.focus_fleet(fleet);
                let kind = self
                    .state
                    .navigation
                    .encounters
                    .iter()
                    .find(|encounter| encounter.fleet == fleet)
                    .map_or("Contact", |encounter| encounter.kind.label());
                format!(
                    "Advanced {advanced} · {}: {kind}, decision needed.",
                    self.fleet_name(fleet)
                )
            }
            Stop::NeedsOrders(fleet) => {
                self.focus_fleet(fleet);
                format!(
                    "Advanced {advanced} · {} needs orders.",
                    self.fleet_name(fleet)
                )
            }
            Stop::NothingUnderway => format!("Advanced {advanced} · no fleet is underway."),
            Stop::Rejected => format!("Advance stopped: {}", self.notice),
            Stop::Limit => format!("Advanced {advanced} without an event; paused."),
        };
        self.record(message);
    }

    fn fleet_name(&self, fleet: FleetId) -> String {
        self.state
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .map_or_else(
                || format!("Fleet {}", fleet.0),
                |record| record.name.clone(),
            )
    }
}

fn runs_label(runs: u32) -> String {
    if runs == 1 {
        "1 run".into()
    } else {
        format!("{runs} runs")
    }
}
