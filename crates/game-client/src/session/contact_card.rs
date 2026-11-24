//! The contact decision as an event card: when the selected fleet is paused
//! by a contact, its choice pops up over the map with a scene of both sides.
//! The card can be hidden (a view preference kept in egui memory per
//! encounter); the sheet's contact line then brings it back.
use super::LocalSession;
use crate::theme::AMBER;
use crate::theme::event_art::{Illustration, Mood};
use crate::theme::event_card::{self, Card};
use crate::theme::icons::{self, Icon};
use game_core::fleet::Fleet;
use game_core::navigation::{Encounter, EncounterKind};

fn hidden_key(encounter: u64) -> egui::Id {
    egui::Id::new(("contact-card-hidden", encounter))
}

impl LocalSession {
    /// The selected fleet and its pending contact, if it has one.
    fn active_contact(&self) -> Option<(Fleet, Encounter)> {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok()?.clone();
        let encounter = self
            .state
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.fleet == fleet.id)?
            .clone();
        Some((fleet, encounter))
    }

    /// The pop-up for the selected fleet's contact (none during a battle).
    pub fn show_contact_card(&mut self, ctx: &egui::Context) {
        if self.battle_open() {
            return;
        }
        let Some((fleet, encounter)) = self.active_contact() else {
            return;
        };
        let key = hidden_key(encounter.id);
        if ctx.data(|data| data.get_temp::<bool>(key).unwrap_or(false)) {
            return;
        }
        let art = self.contact_art(&fleet, &encounter);
        let kicker = format!("Contact · {}", fleet.name);
        let title = if encounter.haven {
            "Raider Haven assault"
        } else {
            encounter.kind.label()
        };
        let card = Card {
            id: egui::Id::new(("contact-card", encounter.id)),
            kicker: &kicker,
            title,
            art: &art,
            can_hide: true,
        };
        let hide = event_card::show(ctx, card, |ui| {
            ui.small("Paused until you decide; its course continues on the next run.");
            self.encounter_body(ui, &fleet, &encounter);
        });
        if hide {
            ctx.data_mut(|data| data.insert_temp(key, true));
        }
    }

    /// The sheet's reminder while a contact waits: one line and a Show button.
    pub(super) fn contact_panel(&mut self, ui: &mut egui::Ui) {
        let Some((fleet, encounter)) = self.active_contact() else {
            return;
        };
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Warning, AMBER, "Decision needed");
            ui.colored_label(AMBER, format!("CONTACT · {}", fleet.name));
            let key = hidden_key(encounter.id);
            let hidden = ui.data(|data| data.get_temp::<bool>(key).unwrap_or(false));
            if hidden && icons::button(ui, Icon::Contact, "Decide").clicked() {
                ui.data_mut(|data| data.insert_temp(key, false));
            }
        });
    }

    fn contact_art(&self, fleet: &Fleet, encounter: &Encounter) -> Illustration {
        let mood = match encounter.kind {
            EncounterKind::Raiders | EncounterKind::Shakedown => Mood::Hostile,
            EncounterKind::Police
            | EncounterKind::CustomsInspection
            | EncounterKind::AuthorityCutter => Mood::Lawful,
            EncounterKind::TraderDistress
            | EncounterKind::DistressSignal
            | EncounterKind::DebrisField
            | EncounterKind::Comet
            | EncounterKind::Anomaly => Mood::Distress,
            _ => Mood::Neutral,
        };
        let mut art = Illustration::new(mood, encounter.id.wrapping_mul(31) ^ fleet.id.0);
        art.ours = fleet
            .ships
            .iter()
            .filter_map(|id| self.state.ship(*id).ok())
            .map(|ship| ship.kind)
            .collect();
        art.theirs = encounter.hostiles.clone();
        art
    }
}
