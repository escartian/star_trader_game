//! Cards for NPC traders that come to you: a trade offer and a corsair's
//! shakedown. Display only; the rules live in `game_core::approach`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use game_core::{
    approach,
    economy::Commodity,
    fleet::Fleet,
    navigation::{Encounter, EncounterChoice, TraderContact},
};
use game_protocol::ClientCommand;

impl LocalSession {
    /// A trader alongside: its hold at the offered prices, then buy or let go.
    pub(super) fn trader_offer_ui(&mut self, ui: &mut egui::Ui, encounter: &Encounter) {
        let Some(contact) = &encounter.trader else {
            return;
        };
        let Some(npc) = self.economy.traders.iter().find(|npc| npc.id == contact.id) else {
            return;
        };
        let hold = &npc.state.ships[0].cargo;
        let goods: Vec<String> = Commodity::ALL
            .iter()
            .filter(|good| hold[good.index()] > 0)
            .map(|good| {
                let price = approach::offer_price(&self.economy, *good);
                format!("{} {} at {price} cr each", hold[good.index()], good.label())
            })
            .collect();
        let percent = self
            .economy
            .rules
            .personality_catalog
            .approaches
            .offer_price_percent;
        ui.label(format!(
            "{} comes alongside and offers its hold below list price ({percent}%): {}.",
            describe(self, contact),
            goods.join(", ")
        ));
        ui.small(
            "Buying takes the most valuable goods first, as far as your holds and wallet allow.",
        );
        let mut choice = None;
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Cargo, "Buy its cargo").clicked() {
                choice = Some(EncounterChoice::Trade);
            }
            if icons::button(ui, Icon::Bypass, "Decline").clicked() {
                choice = Some(EncounterChoice::Bypass);
            }
        });
        self.resolve_approach(encounter, choice);
    }

    /// A corsair's shakedown: what it wants, then comply, fight or run.
    pub(super) fn shakedown_ui(&mut self, ui: &mut egui::Ui, fleet: &Fleet, encounter: &Encounter) {
        let Some(contact) = &encounter.trader else {
            return;
        };
        if self
            .state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| open.encounter == encounter.id)
        {
            ui.label(format!("Battle in progress against {}.", contact.name));
            return;
        }
        let aboard: u32 = fleet
            .ships
            .iter()
            .filter_map(|id| self.state.ship(*id).ok())
            .map(|ship| ship.cargo.iter().sum::<u32>())
            .sum();
        ui.label(format!(
            "{} runs out its guns and demands half your cargo ({} of {aboard} units).",
            describe(self, contact),
            aboard.div_ceil(2)
        ));
        ui.small("Resisting is self-defence: it costs no standing, and a destroyed corsair's hold is yours.");
        let chance = self
            .state
            .evade_chance(fleet, &encounter.hostiles, &self.economy.rules)
            .unwrap_or(0);
        let mut choice = None;
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Confirm, "Hand it over").clicked() {
                choice = Some(EncounterChoice::Comply);
            }
            if icons::button(ui, Icon::Weapon, "Fight")
                .on_hover_text("Command your ships turn by turn.")
                .clicked()
            {
                choice = Some(EncounterChoice::Fight);
            }
            if icons::button(ui, Icon::Execute, "Auto-resolve")
                .on_hover_text("Let the AI fight it out.")
                .clicked()
            {
                choice = Some(EncounterChoice::AutoResolve);
            }
            if icons::button(ui, Icon::Engine, &format!("Evade {chance}%"))
                .on_hover_text("Run for it; a failed escape starts the battle.")
                .clicked()
            {
                choice = Some(EncounterChoice::Evade);
            }
        });
        self.resolve_approach(encounter, choice);
    }

    fn resolve_approach(&mut self, encounter: &Encounter, choice: Option<EncounterChoice>) {
        if let Some(choice) = choice {
            self.command(ClientCommand::ResolveEncounter {
                encounter: encounter.id,
                choice,
                expected_revision: self.state.revision,
            });
        }
    }
}

/// "Name (corsair, Federation)": who it is and whom it flies for.
fn describe(session: &LocalSession, contact: &TraderContact) -> String {
    let Some(npc) = session
        .economy
        .traders
        .iter()
        .find(|npc| npc.id == contact.id)
    else {
        return contact.name.clone();
    };
    let rules = &session.economy.rules;
    let temper = rules
        .personality_catalog
        .get(npc.personality)
        .label
        .to_lowercase();
    match npc.affiliation {
        Some(faction) => format!(
            "{} ({temper}, {})",
            contact.name,
            rules.faction_catalog.get(faction).name
        ),
        None => format!("{} ({temper}, independent)", contact.name),
    }
}
