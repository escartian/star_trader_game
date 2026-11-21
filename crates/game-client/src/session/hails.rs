//! NPC traffic in the map roster, the Hail button for traders the flagship can
//! reach, and a hailed trader's contact card. Rules live in `game_core::piracy`.
use super::LocalSession;
use crate::theme::AMBER;
use crate::theme::icons::{self, Icon};
use game_core::{
    StarSystem,
    economy::Commodity,
    navigation::{Encounter, EncounterChoice},
    npc::Personality,
    piracy,
    progress::Feature,
};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

impl LocalSession {
    /// The roster's TRAFFIC section: each trader's latest news, and a Hail
    /// button beside those the flagship can reach (once battles are unlocked).
    pub(super) fn traffic_list(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        ui.horizontal(|ui| {
            icons::glyph(ui, Icon::Contact, AMBER, "Other traders");
            ui.colored_label(
                AMBER,
                egui::RichText::new("TRAFFIC").font(crate::theme::fonts::display(ui.ctx(), 12.0)),
            );
        });
        let placed = game_core::orbits::placed(systems, self.state.tick);
        let reach = if self.unlocked(Feature::Battles) {
            piracy::in_reach(&self.state, &self.economy, LOCAL_PLAYER, &placed)
        } else {
            Vec::new()
        };
        let mut hail = None;
        for trader in &self.economy.traders {
            ui.horizontal(|ui| {
                ui.strong(&trader.name)
                    .on_hover_text(trader.personality.label());
                let armed = defences(trader);
                if !armed.is_empty() {
                    ui.small(armed)
                        .on_hover_text("Anti-piracy gear and hired escort");
                }
                if reach.contains(&trader.id)
                    && icons::button(ui, Icon::Contact, "Hail")
                        .on_hover_text("Stop this trader: let it go, demand cargo or attack it.")
                        .clicked()
                {
                    hail = Some(trader.id);
                }
            });
            // Raid and piracy news stands out from routine hauling.
            let news = &trader.last_action;
            if news.contains("raid") || news.contains("pirate") {
                ui.colored_label(AMBER, egui::RichText::new(news).small());
            } else {
                ui.small(news);
            }
        }
        if let (Some(trader), Some(fleet)) = (hail, self.state.flagship(LOCAL_PLAYER)) {
            self.command(ClientCommand::HailTrader {
                fleet,
                trader,
                expected_revision: self.state.revision,
            });
        }
    }

    /// A hailed trader's card: its hold and hull, then let it go, demand
    /// cargo (when it would yield) or attack it.
    pub(super) fn trader_ui(&mut self, ui: &mut egui::Ui, encounter: &Encounter) {
        let Some(hailed) = &encounter.trader else {
            return;
        };
        if self
            .state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| open.encounter == encounter.id)
        {
            ui.label(format!("Attack on {} in progress.", hailed.name));
            return;
        }
        let Some(npc) = self.economy.traders.iter().find(|npc| npc.id == hailed.id) else {
            return;
        };
        let ship = &npc.state.ships[0];
        let hull = self
            .economy
            .rules
            .ship_catalog
            .spec(ship.kind)
            .map_or(0, |spec| spec.combat.hull);
        let hold: Vec<String> = Commodity::ALL
            .iter()
            .filter(|good| ship.cargo[good.index()] > 0)
            .map(|good| format!("{} {}", ship.cargo[good.index()], good.label()))
            .collect();
        let hold = if hold.is_empty() {
            "an empty hold".to_owned()
        } else {
            hold.join(", ")
        };
        ui.label(format!(
            "{} ({}) answers your hail: a {} with {hold}, hull {}/{hull}.",
            hailed.name,
            npc.personality.label().to_lowercase(),
            ship.kind.label().to_lowercase(),
            hull.saturating_sub(ship.hull_damage),
        ));
        let armed = defences(npc);
        if !armed.is_empty() {
            ui.small(format!("Defences: {armed}."));
        }
        let threshold = piracy::yield_percent(npc.personality);
        let yields = piracy::would_yield(&self.state, &self.economy, encounter);
        ui.small(match (yields, npc.personality) {
            (true, _) => "Outgunned: it will hand over half its cargo if you demand it.".to_owned(),
            (false, Personality::Cautious) => format!(
                "It will only give way to a fleet with {threshold}% of its fighting strength."
            ),
            (false, Personality::Opportunist) => format!(
                "An opportunist: it gives way only to {threshold}% of its fighting strength."
            ),
        });
        let mut choice = None;
        ui.horizontal_wrapped(|ui| {
            let blocked = (!yields).then(|| "It is not outgunned enough to give way.".to_owned());
            if icons::labeled_action(ui, Icon::Cargo, "Demand cargo", blocked).clicked() {
                choice = Some(EncounterChoice::Demand);
            }
            if icons::button(ui, Icon::Weapon, "Attack")
                .on_hover_text("Fight it turn by turn; destroy it to plunder its hold.")
                .clicked()
            {
                choice = Some(EncounterChoice::Fight);
            }
            if icons::button(ui, Icon::Execute, "Auto-resolve")
                .on_hover_text("Fight the whole attack at once with the AI on both sides.")
                .clicked()
            {
                choice = Some(EncounterChoice::AutoResolve);
            }
            if icons::button(ui, Icon::Bypass, "Let it go").clicked() {
                choice = Some(EncounterChoice::Bypass);
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

    /// The hailed trader's name if the open battle is an attack on one.
    pub(super) fn attacked_trader(&self) -> Option<String> {
        let open = self.state.navigation.battle.as_ref()?;
        self.state
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.id == open.encounter)?
            .trader
            .as_ref()
            .map(|trader| trader.name.clone())
    }
}

/// A trader's anti-piracy gear and escort, as a short list ("" when none).
fn defences(trader: &game_core::npc::Trader) -> String {
    let mut parts: Vec<String> = trader.state.ships[0]
        .gear
        .iter()
        .map(|gear| gear.label().to_lowercase())
        .collect();
    if trader.escort_runs > 0 {
        parts.push(format!("escort ({} runs)", trader.escort_runs));
    }
    parts.join(", ")
}
