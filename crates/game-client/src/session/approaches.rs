//! Cards for NPC traders that come to you: a trade offer, a corsair's
//! shakedown and a stalked trader's distress call. Display only; the rules live in `game_core::approach`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use game_core::{
    approach,
    economy::Commodity,
    fleet::Fleet,
    navigation::{Encounter, EncounterChoice},
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
        let docked = matches!(
            npc.state.ships[0].location,
            game_core::travel::Location::Docked(_)
        );
        let how = if docked {
            "answers your hail at the port and, under the station's guns, offers"
        } else {
            "comes alongside and offers"
        };
        ui.label(format!(
            "{} {how} its hold below list price ({percent}%): {}.",
            describe(self, contact.id, &contact.name),
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
            describe(self, contact.id, &contact.name),
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

    /// A stalked trader's call for help: the band, the reward, then fight
    /// or leave it.
    pub(super) fn distress_ui(&mut self, ui: &mut egui::Ui, encounter: &Encounter) {
        if self
            .state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| open.encounter == encounter.id)
        {
            ui.label("Battle in progress against the raider band.");
            return;
        }
        let Some(npc) = encounter
            .rescue
            .and_then(|id| self.economy.traders.iter().find(|npc| npc.id == id))
        else {
            return;
        };
        let rules = &self.economy.rules;
        let approaches = &rules.personality_catalog.approaches;
        let reward =
            game_core::economy::Credits(npc.credits.0 / 100 * approaches.rescue_reward_percent);
        let band: Vec<_> = encounter.hostiles.iter().map(|kind| kind.label()).collect();
        ui.label(format!(
            "{} calls for help: a raider band ({}) is stalking it.",
            describe(self, npc.id, &npc.name),
            band.join(", ")
        ));
        let thanks = match npc.affiliation {
            Some(faction) => format!(
                " and +{} standing with the {}",
                approaches.rescue_standing,
                rules.faction_catalog.get(faction).name
            ),
            None => String::new(),
        };
        ui.small(format!(
            "Drive the band off and it pays you about {reward} cr{thanks}. The raiders' bounty is yours too."
        ));
        let mut choice = None;
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Weapon, "Fight the band")
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
            if icons::button(ui, Icon::Bypass, "Leave it").clicked() {
                choice = Some(EncounterChoice::Bypass);
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
fn describe(session: &LocalSession, id: u64, name: &str) -> String {
    let Some(npc) = session.economy.traders.iter().find(|npc| npc.id == id) else {
        return name.to_owned();
    };
    let rules = &session.economy.rules;
    let temper = rules
        .personality_catalog
        .get(npc.personality)
        .label
        .to_lowercase();
    match npc.affiliation {
        Some(faction) => format!(
            "{name} ({temper}, {})",
            rules.faction_catalog.get(faction).name
        ),
        None => format!("{name} ({temper}, independent)"),
    }
}
