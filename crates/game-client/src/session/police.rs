//! The law on screen: the police contact card, the wanted banner (with Pay
//! fine) in the market window, wanted lines in the Factions panel and log
//! messages. Display only; the rules live in `game_core::police` and
//! `game_core::factions::wanted`.
use super::LocalSession;
use crate::theme::event_card;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN};
use egui::RichText;
use game_core::{
    factions::Faction,
    fleet::Fleet,
    navigation::{Encounter, EncounterChoice},
    police,
    travel::Ship,
};
use game_protocol::ClientCommand;

const WANTED: egui::Color32 = crate::battle::scene::HOSTILE;

/// A police patrol as the system map draws it.
#[derive(Clone, Debug)]
pub struct PatrolMarker {
    pub patrol: game_core::police::FactionPatrol,
    /// Fill: whose police these are.
    pub faction: egui::Color32,
    /// Outline and pursuit line: friend or foe.
    pub stance: crate::theme::iff::Stance,
}

impl LocalSession {
    /// Every police patrol, with its faction's colour, for the system map.
    /// Each patrol with its faction's colour and how it stands toward you
    /// (one pursuing your fleets is hostile whatever your standing).
    pub fn police_patrols(&self) -> Vec<PatrolMarker> {
        self.state
            .navigation
            .police
            .patrols
            .iter()
            .map(|patrol| PatrolMarker {
                faction: self.faction_color(patrol.faction),
                stance: if patrol.pursuing.is_some() {
                    crate::theme::iff::Stance::Hostile
                } else {
                    self.faction_stance(patrol.faction)
                },
                patrol: patrol.clone(),
            })
            .collect()
    }

    pub(super) fn wanted_message(&self, faction: Faction, runs: u32) -> String {
        match runs {
            0 => format!("The {} no longer wants you.", faction.label()),
            runs => format!(
                "Wanted by the {} for {runs} runs: their police will stop your fleets and \
                 their ports refuse your trade. Pay the fine ({}) to clear it.",
                faction.label(),
                police::fine(&self.economy, faction)
            ),
        }
    }

    /// A police patrol's card: why it stopped you and the fine, then comply,
    /// fight or run.
    pub(super) fn police_ui(&mut self, ui: &mut egui::Ui, fleet: &Fleet, encounter: &Encounter) {
        let Some(contact) = encounter.police else {
            return;
        };
        if self
            .state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| open.encounter == encounter.id)
        {
            ui.label(format!(
                "Battle in progress against the {} patrol.",
                contact.faction.label()
            ));
            return;
        }
        let faction = contact.faction;
        let factions = &self.economy.factions;
        let names: Vec<_> = encounter.hostiles.iter().map(|kind| kind.label()).collect();
        let reason = match factions.wanted_runs(faction) {
            0 => "your standing with them is Hostile".to_owned(),
            runs => format!("you are wanted for {runs} more runs"),
        };
        let article = if faction.label().starts_with(['A', 'E', 'I', 'O', 'U']) {
            "An"
        } else {
            "A"
        };
        ui.label(format!(
            "{article} {} patrol ({}) orders you to heave to: {reason}.",
            faction.label(),
            names.join(", ")
        ));
        let fine = police::fine(&self.economy, faction);
        let short = fine.0 > self.economy.credits.0;
        ui.small(if short {
            format!(
                "The fine is {fine}, more than you hold: complying hands over your {} and \
                 every cargo unit aboard.",
                self.economy.credits
            )
        } else {
            format!("Complying pays the {fine} fine and clears your record.")
        });
        ui.small(format!(
            "Fighting the police is a crime: it extends your wanted status, and each patrol \
             ship destroyed costs {} standing.",
            self.economy
                .rules
                .faction_catalog
                .get(faction)
                .piracy_penalty
        ));
        let chance = self
            .state
            .evade_chance(fleet, &encounter.hostiles, &self.economy.rules)
            .unwrap_or(0);
        let mut choice = None;
        event_card::choices(ui, |ui| {
            if event_card::choice(ui, Icon::Confirm, &format!("Comply · {fine}")).clicked() {
                choice = Some(EncounterChoice::Comply);
            }
            if event_card::choice(ui, Icon::Weapon, "Fight")
                .on_hover_text("Resist arrest: command your ships turn by turn.")
                .clicked()
            {
                choice = Some(EncounterChoice::Fight);
            }
            if event_card::choice(ui, Icon::Execute, "Auto-resolve")
                .on_hover_text("Resist arrest and let the AI fight it out.")
                .clicked()
            {
                choice = Some(EncounterChoice::AutoResolve);
            }
            if event_card::choice(ui, Icon::Engine, &format!("Evade {chance}%"))
                .on_hover_text("Run for it; a failed escape starts the battle.")
                .clicked()
            {
                choice = Some(EncounterChoice::Evade);
            }
        });
        if let Some(choice) = choice {
            self.command(ClientCommand::ResolveEncounter {
                encounter: encounter.id,
                choice,
                expected_revision: self.state.revision,
            });
        }
    }

    /// In the market window: when the port's faction wants you, how long and
    /// what the fine is, with a button to pay it here.
    pub(super) fn wanted_banner(&mut self, ui: &mut egui::Ui, ship: &Ship) {
        let Some((faction, _)) = self.economy.port_terms(ship).holder else {
            return;
        };
        let runs = self.economy.factions.wanted_runs(faction);
        if runs == 0 {
            return;
        }
        let fine = police::fine(&self.economy, faction);
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Warning, WANTED, "Wanted");
            ui.label(
                RichText::new(format!("WANTED by the {} · {runs} runs", faction.label()))
                    .color(WANTED)
                    .strong(),
            );
            let blocked = (fine.0 > self.economy.credits.0)
                .then(|| format!("The fine is {fine}; you hold {}.", self.economy.credits));
            let label = format!("Pay fine · {fine}");
            if icons::labeled_action(ui, Icon::Credits, &label, blocked).clicked() {
                self.command(ClientCommand::PayFine {
                    ship: ship.id,
                    expected_revision: self.state.revision,
                });
            }
        });
    }

    /// In the Factions panel: your record with `faction`.
    pub(super) fn wanted_line(&self, ui: &mut egui::Ui, faction: Faction) {
        let factions = &self.economy.factions;
        let definition = self.economy.rules.faction_catalog.get(faction);
        let fine = police::fine(&self.economy, faction);
        match factions.wanted_runs(faction) {
            0 if factions.pursues_you(faction) => {
                ui.label(
                    RichText::new("Hostile: their police stop your fleets in their space.")
                        .color(AMBER)
                        .small(),
                );
            }
            0 => {
                ui.label(RichText::new("Record clean.").color(CYAN).small());
            }
            runs => {
                ui.label(
                    RichText::new(format!(
                        "WANTED · {runs} runs left · fine {fine}. Their police stop your \
                         fleets; pay at any of their ports."
                    ))
                    .color(WANTED)
                    .small(),
                );
            }
        }
        ui.small(format!(
            "Each crime here: wanted {} runs more · fine {} per run left",
            definition.wanted_runs,
            game_core::economy::Credits(definition.fine_per_run)
        ));
    }
}
