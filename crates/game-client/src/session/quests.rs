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
use game_core::factions::Faction;
use game_core::jobs::{Job, quests};
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
        let name = self.patron_name(quest.faction);
        let kicker = format!(
            "{} · {}",
            name,
            if quest.treaty { "Treaty" } else { "Request" }
        );
        let mood = match (quest.faction, quest.treaty) {
            (None, _) => Mood::Hostile,
            (Some(_), true) => Mood::Triumph,
            (Some(_), false) => Mood::Lawful,
        };
        let mut art = Illustration::new(mood, job.id.wrapping_mul(131));
        art.station = quest.faction.is_some();
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
            let terms = match quest.faction {
                Some(faction) => format!(
                    "Reward {} cr · +{} {} standing (−{} if it fails)",
                    job.reward,
                    job.standing,
                    faction.label(),
                    job.penalty
                ),
                None => format!("Reward {} cr · no questions asked", job.reward),
            };
            ui.label(RichText::new(terms).color(GREEN).small());
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

    /// Who offers a quest chain: a faction's full name, or the Free Captains.
    fn patron_name(&self, patron: Option<Faction>) -> String {
        let rules = &self.economy.rules;
        match patron {
            Some(faction) => rules.faction_catalog.get(faction).name.clone(),
            None => rules.quest_catalog.pirate_name().to_owned(),
        }
    }

    /// The Jobs window's Requests (quest offers) and story progress.
    pub(super) fn quests_ui(&mut self, ui: &mut egui::Ui) {
        let mut patrons: Vec<Option<Faction>> = self
            .economy
            .factions
            .in_play
            .iter()
            .copied()
            .map(Some)
            .collect();
        if quests::pirates_active(&self.economy) {
            patrons.push(None);
        }
        if patrons.is_empty() {
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
            for patron in patrons {
                let (done, total) = quests::chain_progress(&self.state, &self.economy, patron);
                if total == 0 {
                    continue;
                }
                let won = match patron {
                    Some(faction) => self.state.jobs.treaty == Some(faction),
                    None => done >= total,
                };
                let color = if won { GREEN } else { CYAN };
                let label = patron.map_or_else(
                    || self.economy.rules.quest_catalog.pirate_name().to_owned(),
                    |faction| faction.label().to_owned(),
                );
                let tip = match patron {
                    Some(_) => "Story steps done. A faction's requests open at Friendly \
                                standing; its treaty, the last step, at Honored, and carrying \
                                it wins the Diplomat road."
                        .to_owned(),
                    None => {
                        let next = quests::pirate_needs(&self.state, &self.economy)
                            .map_or_else(String::new, |needs| format!(" Next step needs {needs}."));
                        format!(
                            "Story steps done. The Free Captains' requests open as you seize \
                             cargo from traders and escape the police; finishing them wins the \
                             Pirate road.{next}"
                        )
                    }
                };
                ui.label(
                    RichText::new(format!("{label} {done}/{total}"))
                        .color(color)
                        .small(),
                )
                .on_hover_text(tip);
            }
        });
        ui.separator();
    }
}

#[cfg(any(test, feature = "dev-tools"))]
impl LocalSession {
    /// Capture "quest" (the Federation's first story request) or
    /// "pirate-quest" (the Free Captains' first) as its card (client state
    /// only; the request is drawn from the home board).
    pub fn capture_quest(&mut self, pirate: bool) -> Result<(), String> {
        self.capture_jobs("jobs")?;
        self.market.open = false;
        let patron = (!pirate).then_some(game_core::factions::Faction::Federation);
        let step = self
            .economy
            .rules
            .quest_catalog
            .steps(patron)
            .first()
            .ok_or("No quest chain")?
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
        job.brief = if pirate {
            "Seize 8 units from traders, by demand or plunder, within 16 runs.".into()
        } else {
            "Bring 8 Medicine to Sol Inner world within 16 runs.".into()
        };
        job.quest = Some(game_core::jobs::quests::QuestRef {
            faction: patron,
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
