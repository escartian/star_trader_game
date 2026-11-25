//! Jobs on the client: the port's Jobs tab (its board's offers, taken with
//! Accept while a ship is there), the Jobs window (accepted jobs, where to go,
//! deadlines, Abandon) and an event card for each job that ends. Display
//! only; the host owns the job rules (`game_core::jobs`).
use super::LocalSession;
use super::market::market_name;
use crate::theme::event_art::{Illustration, Mood};
use crate::theme::event_card::{self, Card};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN, metal};
use egui::RichText;
use game_core::StarSystem;
use game_core::economy::MarketId;
use game_core::jobs::{Job, JobKind, JobOutcome, Task};
use game_protocol::ClientCommand;
use std::collections::VecDeque;

#[derive(Default)]
pub(super) struct JobsPanel {
    pub(super) open: bool,
    /// Jobs that ended and have not been acknowledged yet, oldest first.
    pub(super) finished: VecDeque<JobOutcome>,
    /// Quest offers whose event card has not been answered, oldest first.
    pub(super) quest_cards: VecDeque<u64>,
}

fn kind_icon(kind: JobKind) -> Icon {
    match kind {
        JobKind::Deliver => Icon::Cargo,
        JobKind::Hunt => Icon::Weapon,
        JobKind::Escort => Icon::Fleets,
        JobKind::Survey => Icon::Orbit,
    }
}

fn system_name(systems: &[StarSystem], id: game_core::SystemId) -> &str {
    systems
        .iter()
        .find(|system| system.id == id)
        .map_or("an unknown system", |system| system.name.as_str())
}

/// Where the job's task is done, for the Jobs window.
fn destination(systems: &[StarSystem], task: &Task) -> String {
    let place = |system: game_core::SystemId, body: &str| match systems
        .iter()
        .find(|known| known.id == system)
    {
        Some(known) if !body.starts_with(&known.name) => format!("{body}, {}", known.name),
        _ => body.to_owned(),
    };
    let port = |market: MarketId| place(market.system, market_name(systems, market));
    match task {
        Task::Deliver {
            commodity,
            quantity,
            market,
        } => format!(
            "Bring {quantity} {} to {}",
            commodity.label(),
            port(*market)
        ),
        Task::Hunt { system, .. } => {
            format!("Raider band last seen in {}", system_name(systems, *system))
        }
        Task::Escort { market, .. } => format!(
            "Be in {} when the trader docks",
            system_name(systems, market.system)
        ),
        Task::Survey { system, body } => {
            let body = systems
                .iter()
                .find(|known| known.id == *system)
                .and_then(|known| known.bodies.iter().find(|candidate| candidate.id == *body))
                .map_or("the body", |body| body.name.as_str());
            format!("Orbit {}", place(*system, body))
        }
    }
}

impl LocalSession {
    /// The reward line: credits and standing with the issuing faction.
    fn reward_line(&self, job: &Job) -> String {
        let mut line = format!("{} cr", job.reward);
        if let Some(faction) = job.faction.filter(|_| self.factions_active()) {
            line.push_str(&format!(
                " · +{} {} standing (−{} if failed)",
                job.standing,
                faction.label(),
                job.penalty
            ));
        }
        line
    }

    fn job_summary(&self, ui: &mut egui::Ui, job: &Job) {
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, kind_icon(job.kind), CYAN, job.kind.label());
            ui.label(RichText::new(&job.title).strong());
        });
        ui.label(RichText::new(&job.brief).small());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(self.reward_line(job)).color(GREEN).small());
            let left = job.deadline.saturating_sub(self.state.navigation.run);
            ui.label(
                RichText::new(format!("· due run {} ({left} runs left)", job.deadline))
                    .color(if left <= 3 {
                        AMBER
                    } else {
                        egui::Color32::GRAY
                    })
                    .small(),
            );
        });
    }

    /// The market window's Jobs page: this port's offers.
    pub(super) fn jobs_board_ui(&mut self, ui: &mut egui::Ui, market: MarketId) {
        let offers: Vec<Job> = self
            .state
            .jobs
            .offers
            .iter()
            .filter(|job| job.issuer == market)
            .cloned()
            .collect();
        let active = self.state.jobs.active.len();
        let max = self.economy.rules.job_catalog.max_active as usize;
        ui.label(
            RichText::new(format!(
                "Jobs taken: {active} of {max}. The board changes every {} runs.",
                self.economy.rules.job_catalog.refresh_runs
            ))
            .small(),
        );
        if offers.is_empty() {
            ui.weak("Nothing on the board right now.");
        }
        let mut accept = None;
        for job in &offers {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                self.job_summary(ui, job);
                let blocked =
                    (active >= max).then(|| "You have as many jobs as you can handle.".to_owned());
                if icons::labeled_action(ui, Icon::Confirm, "Accept", blocked).clicked() {
                    accept = Some(job.id);
                }
            });
        }
        if let Some(job) = accept {
            self.command(ClientCommand::AcceptJob {
                job,
                ship: self.active_ship,
                expected_revision: self.state.revision,
            });
        }
    }

    /// The Jobs window (Menu → Jobs): accepted jobs and the record so far.
    pub fn show_jobs(&mut self, ctx: &egui::Context, systems: &[StarSystem]) {
        let mut open = self.jobs.open;
        metal::window(ctx, "Jobs", &mut open, 520.0, |ui| {
            self.jobs_ui(ui, systems)
        });
        self.jobs.open = open;
        self.job_outcome_card(ctx);
        self.quest_offer_card(ctx);
    }

    fn jobs_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        self.quests_ui(ui);
        let jobs = self.state.jobs.clone();
        ui.label(
            RichText::new(format!(
                "Completed {} · failed {}",
                jobs.completed, jobs.failed
            ))
            .small(),
        );
        if jobs.active.is_empty() {
            ui.weak("No jobs taken. Ports post them on the market window's Jobs tab.");
        }
        let mut abandon = None;
        for job in &jobs.active {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                self.job_summary(ui, job);
                ui.horizontal_wrapped(|ui| {
                    icons::glyph(ui, Icon::Pin, CYAN, "Where");
                    ui.label(RichText::new(destination(systems, &job.task)).small());
                });
                if icons::button(ui, Icon::Cancel, "Abandon")
                    .on_hover_text("Give it up now; it counts as failed.")
                    .clicked()
                {
                    abandon = Some(job.id);
                }
            });
        }
        if let Some(job) = abandon {
            self.command(ClientCommand::AbandonJob {
                job,
                expected_revision: self.state.revision,
            });
        }
        if !jobs.history.is_empty() {
            ui.separator();
            ui.label(RichText::new("RECENT").small().color(egui::Color32::GRAY));
            for done in jobs.history.iter().rev().take(6) {
                let (mark, color) = if done.success {
                    ("✔", GREEN)
                } else {
                    ("✖", AMBER)
                };
                ui.label(
                    RichText::new(format!("{mark} {} · run {}", done.title, done.run))
                        .color(color)
                        .small(),
                );
            }
        }
    }

    /// One event card per ended job, oldest first (none during a battle).
    fn job_outcome_card(&mut self, ctx: &egui::Context) {
        if self.battle_open() {
            return;
        }
        let Some(outcome) = self.jobs.finished.front().cloned() else {
            return;
        };
        let (title, mood) = if outcome.success {
            ("Job complete", Mood::Triumph)
        } else {
            ("Job failed", Mood::Distress)
        };
        let mut art = Illustration::new(mood, (outcome.title.len() as u64).wrapping_mul(977));
        art.station = outcome.success;
        art.ours = vec![self.selected_ship().kind];
        let mut done = false;
        let card = Card {
            id: egui::Id::new("job-outcome"),
            kicker: &outcome.title,
            title,
            art: &art,
            can_hide: false,
        };
        event_card::show(ctx, card, |ui| {
            ui.label(&outcome.reason);
            if outcome.success {
                ui.label(RichText::new(format!("Paid {} cr.", outcome.reward)).color(GREEN));
            }
            event_card::choices(ui, |ui| {
                done = event_card::choice(ui, Icon::Confirm, "Understood").clicked();
            });
        });
        if done {
            self.jobs.finished.pop_front();
        }
    }
}

#[cfg(any(test, feature = "dev-tools"))]
impl LocalSession {
    /// Captures: "jobs" the home port's board, "jobs-window" with its first
    /// offer taken, "job-done" with that job's completion card.
    pub fn capture_jobs(&mut self, view: &str) -> Result<(), String> {
        // A loaded fixture's board posts with its first order.
        let fleet = self
            .state
            .fleet_for_ship(self.active_ship)
            .map_err(|error| error.to_string())?
            .id;
        self.command(ClientCommand::HoldFleet {
            fleet,
            expected_revision: self.state.revision,
        });
        if view == "jobs" {
            self.open_market_page_jobs();
            return Ok(());
        }
        let job = self
            .state
            .jobs
            .offers
            .first()
            .ok_or("No job on the board")?
            .clone();
        self.command(ClientCommand::AcceptJob {
            job: job.id,
            ship: self.active_ship,
            expected_revision: self.state.revision,
        });
        if view == "job-done" {
            self.jobs.finished.push_back(JobOutcome {
                title: job.title,
                success: true,
                reward: job.reward,
                reason: "Delivered 12 Medicine.".into(),
            });
        } else {
            self.jobs.open = true;
        }
        Ok(())
    }
}
