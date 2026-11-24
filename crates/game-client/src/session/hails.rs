//! NPC traffic in the map roster, the Hail button for traders the flagship can
//! reach, and a hailed trader's contact card. Rules live in `game_core::piracy`.
use super::LocalSession;
use crate::theme::AMBER;
use crate::theme::event_card;
use crate::theme::icons::{self, Icon};
use game_core::{
    StarSystem,
    economy::Commodity,
    navigation::{Encounter, EncounterChoice},
    npc::Trader,
    piracy,
};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

/// What a traffic row's button asked for.
enum Row {
    Hail,
    Pursue,
}

impl LocalSession {
    /// The roster's TRAFFIC section: traders near the flagship first (those
    /// it can hail, with a Hail button once battles are unlocked, then the
    /// rest of its system), and everyone else folded away under Elsewhere, so
    /// a 32-trader galaxy stays readable.
    pub(super) fn traffic_list(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let total = self.economy.traders.len();
        ui.horizontal(|ui| {
            icons::glyph(ui, Icon::Contact, AMBER, "Other traders");
            ui.colored_label(
                AMBER,
                egui::RichText::new(format!("TRAFFIC · {total}"))
                    .font(crate::theme::fonts::display(ui.ctx(), 12.0)),
            );
        });
        self.pursuit_status(ui);
        let placed = game_core::orbits::placed(systems, self.state.tick);
        let reach = piracy::in_reach(&self.state, &self.economy, LOCAL_PLAYER, &placed);
        let here = self.flagship_system();
        let (mut near, far): (Vec<&Trader>, Vec<&Trader>) =
            self.economy.traders.iter().partition(|trader| {
                reach.contains(&trader.id)
                    || (here.is_some() && trader.state.ships[0].location.system() == here)
            });
        near.sort_by_key(|trader| (!reach.contains(&trader.id), trader.id));
        let mut hail = None;
        let mut chase = None;
        if near.is_empty() {
            ui.small("No traders near your flagship.");
        }
        let pursue = self.can_pursue();
        for trader in near {
            let in_system = here.is_some() && trader.state.ships[0].location.system() == here;
            match self.trader_row(ui, trader, reach.contains(&trader.id), pursue && in_system) {
                Some(Row::Hail) => hail = Some(trader.id),
                Some(Row::Pursue) => chase = Some(trader.id),
                None => {}
            }
        }
        let band = self.raider_rows(ui, here);
        if !far.is_empty() {
            egui::CollapsingHeader::new(format!("Elsewhere ({})", far.len()))
                .id_salt("traffic-elsewhere")
                .default_open(false)
                .show(ui, |ui| {
                    for trader in far {
                        self.trader_row(ui, trader, false, false);
                    }
                });
        }
        use game_core::pursuit::Quarry;
        if let Some(quarry) = chase.map(Quarry::Trader).or(band.map(Quarry::Band)) {
            self.pursue(quarry);
        }
        if let (Some(trader), Some(fleet)) = (hail, self.state.flagship(LOCAL_PLAYER)) {
            self.command(ClientCommand::HailTrader {
                fleet,
                trader,
                expected_revision: self.state.revision,
            });
        }
    }

    /// The system the flagship's lead ship is in, if any.
    fn flagship_system(&self) -> Option<game_core::SystemId> {
        let fleet = self.state.flagship(LOCAL_PLAYER)?;
        let record = self.state.fleets.iter().find(|record| record.id == fleet)?;
        let lead = self.state.ship(*record.ships.first()?).ok()?;
        lead.location.system()
    }

    /// One trader: name, faction, defences, a Hail button (when in reach) or
    /// a Pursue button (when in the flagship's system), then its latest news.
    fn trader_row(
        &self,
        ui: &mut egui::Ui,
        trader: &Trader,
        can_hail: bool,
        can_pursue: bool,
    ) -> Option<Row> {
        let mut clicked = None;
        ui.horizontal(|ui| {
            let temper = self
                .economy
                .rules
                .personality_catalog
                .get(trader.personality);
            ui.strong(&trader.name)
                .on_hover_text(format!("{}: {}", temper.label, temper.description));
            if let Some(faction) = trader.affiliation {
                let name = &self.economy.rules.faction_catalog.get(faction).name;
                ui.colored_label(
                    self.faction_color(faction),
                    egui::RichText::new(name).small(),
                )
                .on_hover_text(format!(
                    "Flies for the {name}: robbing it also costs standing with them."
                ));
            }
            let armed = defences(trader);
            if !armed.is_empty() {
                ui.small(armed)
                    .on_hover_text("Anti-piracy gear and hired escort");
            }
            let docked = matches!(
                trader.state.ships[0].location,
                game_core::travel::Location::Docked(_)
            );
            let tip = if docked {
                "Docked at your port: under the station's guns it will only trade."
            } else {
                "Stop this trader: let it go, demand cargo or attack it."
            };
            let label = if docked { "Hail · trade" } else { "Hail" };
            if can_hail
                && icons::button(ui, Icon::Contact, label)
                    .on_hover_text(tip)
                    .clicked()
            {
                clicked = Some(Row::Hail);
            } else if !can_hail && can_pursue && Self::pursue_button(ui, "this trader") {
                clicked = Some(Row::Pursue);
            }
        });
        // Raid and piracy news stands out from routine hauling.
        let news = &trader.last_action;
        if news.contains("raid") || news.contains("pirate") {
            ui.colored_label(AMBER, egui::RichText::new(news).small());
        } else {
            ui.small(news);
        }
        clicked
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
        let temper = self.economy.rules.personality_catalog.get(npc.personality);
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
            temper.label.to_lowercase(),
            ship.kind.label().to_lowercase(),
            hull.saturating_sub(ship.hull_damage),
        ));
        let armed = defences(npc);
        if !armed.is_empty() {
            ui.small(format!("Defences: {armed}."));
        }
        let threshold = temper.yield_percent;
        let yields = piracy::would_yield(&self.state, &self.economy, encounter);
        ui.small(if yields {
            "Outgunned: it will hand over half its cargo if you demand it.".to_owned()
        } else {
            format!("It gives way only to a fleet with {threshold}% of its fighting strength.")
        });
        let mut choice = None;
        event_card::choices(ui, |ui| {
            let blocked = (!yields).then(|| "It is not outgunned enough to give way.".to_owned());
            if event_card::choice_action(ui, Icon::Cargo, "Demand cargo", blocked).clicked() {
                choice = Some(EncounterChoice::Demand);
            }
            if event_card::choice(ui, Icon::Weapon, "Attack")
                .on_hover_text("Fight it turn by turn; destroy it to plunder its hold.")
                .clicked()
            {
                choice = Some(EncounterChoice::Fight);
            }
            if event_card::choice(ui, Icon::Execute, "Auto-resolve")
                .on_hover_text("Fight the whole attack at once with the AI on both sides.")
                .clicked()
            {
                choice = Some(EncounterChoice::AutoResolve);
            }
            if event_card::choice(ui, Icon::Bypass, "Let it go").clicked() {
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
