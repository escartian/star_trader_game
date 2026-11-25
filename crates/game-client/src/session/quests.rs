//! Story quests on the client: a faction's offer arrives as an event card
//! telling its story (Accept, or Later: it waits in Menu → Jobs under
//! Requests), and the Jobs window shows each faction's story progress.
//! Display only; the host offers and settles quests (`game_core::jobs::quests`).
use super::LocalSession;
use crate::theme::event_art::{Illustration, Mood};
use crate::theme::event_card::{self, Card};
use crate::theme::icons::{self, Icon};
use crate::theme::{CYAN, GREEN};
use egui::RichText;
use game_core::jobs::Job;
use game_protocol::ClientCommand;

impl LocalSession {
    fn quest_offer(&self, id: u64) -> Option<Job> {
        self.state
            .jobs
            .quest_offers
            .iter()
            .find(|job| job.id == id)
            .cloned()
    }

    /// The oldest unseen quest offer, as an event card (none during a battle).
    pub(super) fn quest_offer_card(&mut self, ctx: &egui::Context) {
        if self.battle_open() {
            return;
        }
        // Offers accepted or gone meanwhile need no card.
        while let Some(id) = self.jobs.quest_cards.front().copied() {
            if self.quest_offer(id).is_some() {
                break;
            }
            self.jobs.quest_cards.pop_front();
        }
        let Some(job) = self
            .jobs
            .quest_cards
            .front()
            .and_then(|id| self.quest_offer(*id))
        else {
            return;
        };
        let Some(quest) = job.quest.clone() else {
            return;
        };
        let name = &self.economy.rules.faction_catalog.get(quest.faction).name;
        let kicker = format!(
            "{} · {}",
            name,
            if quest.treaty { "Treaty" } else { "Request" }
        );
        let mut art = Illustration::new(
            if quest.treaty {
                Mood::Triumph
            } else {
                Mood::Lawful
            },
            job.id.wrapping_mul(131),
        );
        art.station = true;
        art.ours = vec![self.selected_ship().kind];
        let (mut accept, mut later) = (false, false);
        let card = Card {
            id: egui::Id::new(("quest-offer", job.id)),
            kicker: &kicker,
            title: &job.title,
            art: &art,
            can_hide: false,
        };
        event_card::show(ctx, card, |ui| {
            ui.label(&quest.intro);
            ui.add_space(4.0);
            ui.label(RichText::new(&job.brief).strong());
            ui.label(
                RichText::new(format!(
                    "Reward {} cr · +{} {} standing (−{} if it fails)",
                    job.reward,
                    job.standing,
                    quest.faction.label(),
                    job.penalty
                ))
                .color(GREEN)
                .small(),
            );
            event_card::choices(ui, |ui| {
                accept = event_card::choice(ui, Icon::Confirm, "Accept")
                    .on_hover_text("Its deadline starts now. Quests count against no job limit.")
                    .clicked();
                later = event_card::choice(ui, Icon::Clock, "Later")
                    .on_hover_text("It waits in Menu → Jobs, under Requests.")
                    .clicked();
            });
        });
        if accept {
            self.command(ClientCommand::AcceptJob {
                job: job.id,
                ship: self.active_ship,
                expected_revision: self.state.revision,
            });
        }
        if accept || later {
            self.jobs.quest_cards.pop_front();
        }
    }

    /// The Jobs window's Requests (quest offers) and story progress.
    pub(super) fn quests_ui(&mut self, ui: &mut egui::Ui) {
        if !self.factions_active() {
            return;
        }
        let offers = self.state.jobs.quest_offers.clone();
        let mut accept = None;
        if !offers.is_empty() {
            ui.label(RichText::new("REQUESTS").small().color(egui::Color32::GRAY));
            for job in &offers {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(&job.title).strong());
                    ui.label(RichText::new(&job.brief).small());
                    if icons::button(ui, Icon::Confirm, "Accept").clicked() {
                        accept = Some(job.id);
                    }
                });
            }
        }
        if let Some(job) = accept {
            self.command(ClientCommand::AcceptJob {
                job,
                ship: self.active_ship,
                expected_revision: self.state.revision,
            });
        }
        ui.label(RichText::new("STORY").small().color(egui::Color32::GRAY));
        ui.horizontal_wrapped(|ui| {
            for faction in self.economy.factions.in_play.clone() {
                let (done, total) =
                    game_core::jobs::quests::chain_progress(&self.state, &self.economy, faction);
                if total == 0 {
                    continue;
                }
                let treaty = self.state.jobs.treaty == Some(faction);
                let color = if treaty { GREEN } else { CYAN };
                ui.label(
                    RichText::new(format!("{} {done}/{total}", faction.label()))
                        .color(color)
                        .small(),
                )
                .on_hover_text(
                    "Story steps done. A faction's requests open at Friendly standing; \
                         its treaty, the last step, at Honored, and carrying it wins the \
                         Diplomat road.",
                );
            }
        });
        ui.separator();
    }
}

#[cfg(any(test, feature = "dev-tools"))]
impl LocalSession {
    /// Capture "quest": the Federation's first story request as its card
    /// (client state only; the request is drawn from the home board).
    pub fn capture_quest(&mut self) -> Result<(), String> {
        self.capture_jobs("jobs")?;
        self.market.open = false;
        let faction = game_core::factions::Faction::Federation;
        let step = self
            .economy
            .rules
            .quest_catalog
            .chain(faction)
            .and_then(|chain| chain.steps.first())
            .ok_or("No Federation quest")?
            .clone();
        let mut job = self
            .state
            .jobs
            .offers
            .first()
            .ok_or("No job on the board")?
            .clone();
        job.id = u64::MAX;
        job.title = step.title;
        job.brief = "Bring 8 Medicine to Sol Inner world within 16 runs.".into();
        job.quest = Some(game_core::jobs::quests::QuestRef {
            faction,
            step: 0,
            runs: step.runs,
            intro: step.intro,
            treaty: false,
        });
        self.state.jobs.quest_offers.push(job);
        self.jobs.quest_cards.push_back(u64::MAX);
        Ok(())
    }
}
