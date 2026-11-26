//! The Factions panel (Menu → Factions): each power's territory, temperament
//! and relations, your standing and what it means at their ports. Also the
//! colours and holders the maps and market window show. Display only: the
//! rules live in `game_core::factions`.
use super::LocalSession;
use crate::theme::{AMBER, CYAN, GREEN, iff::Stance, metal};
use egui::{Color32, RichText};
use game_core::{
    StarSystem, SystemId,
    factions::{Faction, MAX_STANDING, MIN_STANDING, Relation, Tier},
};

const HOSTILE: Color32 = crate::battle::scene::HOSTILE;

#[derive(Default)]
pub(super) struct FactionsPanel {
    pub(super) open: bool,
}

/// A colour for a standing tier.
pub fn tier_color(tier: Tier) -> Color32 {
    match tier {
        Tier::Hostile => HOSTILE,
        Tier::Unfriendly => AMBER,
        Tier::Neutral => CYAN,
        Tier::Friendly | Tier::Honored => GREEN,
    }
}

impl LocalSession {
    /// Whether this game has factions at all.
    pub fn factions_active(&self) -> bool {
        self.economy.factions.is_active()
    }

    pub fn faction_color(&self, faction: Faction) -> Color32 {
        let [r, g, b] = self.economy.rules.faction_catalog.get(faction).color;
        Color32::from_rgb(r, g, b)
    }

    /// How `faction` stands toward you on the map: hostile when it wants you
    /// or counts you Hostile, friendly from Friendly up, otherwise neutral.
    pub fn faction_stance(&self, faction: Faction) -> Stance {
        let factions = &self.economy.factions;
        if !factions.is_active() {
            return Stance::Neutral;
        }
        match factions.tier(faction) {
            _ if factions.is_wanted(faction) => Stance::Hostile,
            Tier::Hostile => Stance::Hostile,
            Tier::Friendly | Tier::Honored => Stance::Friendly,
            Tier::Unfriendly | Tier::Neutral => Stance::Neutral,
        }
    }

    /// Who holds `system`, in its colour, and your tier with them.
    pub fn system_holder(&self, system: SystemId) -> Option<(Faction, Color32, Tier)> {
        let faction = self.economy.factions.owner(system)?;
        Some((
            faction,
            self.faction_color(faction),
            self.economy.factions.tier(faction),
        ))
    }

    /// "Federation standing +3 → 18 (Friendly)", noting a change of tier.
    pub(super) fn standing_message(&self, faction: Faction, change: i32, standing: i32) -> String {
        let tier = Tier::of(standing);
        let crossed = Tier::of(standing - change) != tier;
        format!(
            "{} standing {change:+} → {standing} ({}){}",
            faction.label(),
            tier.label(),
            if crossed {
                ". Your terms at their ports changed."
            } else {
                "."
            }
        )
    }

    pub fn show_factions(&mut self, ctx: &egui::Context, systems: &[StarSystem]) {
        let mut open = self.factions.open;
        metal::window(ctx, "Factions", &mut open, 520.0, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.factions_ui(ui, systems));
        });
        self.factions.open = open;
    }

    fn factions_ui(&self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let factions = &self.economy.factions;
        if !factions.is_active() {
            ui.label("No factions take part in this game: every port trades at list prices.");
            return;
        }
        ui.label(match factions.allegiance {
            Some(faction) => format!("You declared for the {} at the start.", faction.label()),
            None => "You declared for no one: you start neutral with every faction.".into(),
        });
        ui.small(
            "Standing runs from −100 to 100. Piracy in a faction's space costs standing \
             and makes you wanted there; destroying raiders and selling cargo there earns \
             it. Half of every change spreads to its allies, and against you to its rivals.",
        );
        ui.add_space(6.0);
        let catalog = &self.economy.rules.faction_catalog;
        for faction in &factions.in_play {
            let definition = catalog.get(*faction);
            let color = self.faction_color(*faction);
            let standing = factions.standing(*faction);
            let tier = factions.tier(*faction);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    super::faction_emblems::show(ui, *faction);
                    ui.label(
                        RichText::new(faction.label().to_uppercase())
                            .color(color)
                            .strong(),
                    );
                    ui.label(RichText::new(&definition.name).small());
                    ui.label(RichText::new(format!("· {}", definition.species)).small());
                    ui.label(
                        RichText::new(format!(
                            "· holds {}% of systems",
                            factions.influence_percent(*faction, systems.len())
                        ))
                        .small(),
                    );
                });
                ui.small(&definition.description);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{standing:+}"))
                            .color(tier_color(tier))
                            .strong(),
                    );
                    ui.label(RichText::new(tier.label()).color(tier_color(tier)));
                    let share =
                        (standing - MIN_STANDING) as f32 / (MAX_STANDING - MIN_STANDING) as f32;
                    ui.add(egui::ProgressBar::new(share).desired_width(160.0));
                });
                let terms = game_core::factions::PortTerms {
                    holder: Some((*faction, tier)),
                    wanted: factions.is_wanted(*faction),
                };
                ui.small(terms.summary());
                self.wanted_line(ui, *faction);
                ui.small(format!(
                    "Piracy here −{} · each raider destroyed +{} · +1 per {} sold",
                    definition.piracy_penalty,
                    definition.raider_kill_standing,
                    game_core::economy::Credits(definition.sale_credits_per_point)
                ));
                let ties: Vec<String> = factions
                    .in_play
                    .iter()
                    .filter(|other| *other != faction)
                    .filter_map(|other| match factions.relation(*faction, *other) {
                        Relation::Allied => Some(format!("allied with the {}", other.label())),
                        Relation::Rival => Some(format!("rival of the {}", other.label())),
                        Relation::Neutral => None,
                    })
                    .collect();
                if !ties.is_empty() {
                    ui.small(format!("Relations: {}.", ties.join(", ")));
                }
            });
        }
    }
}
