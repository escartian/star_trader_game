//! Read-only combat report window. Opens automatically once per new engagement;
//! all numbers come from the authoritative saved report.
use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::combat::{CombatOutcome, CombatReport, Side, engagement::CombatantResult};

pub(super) struct CombatReportPanel {
    pub open: bool,
    /// Encounter ID of the report the player has already been shown.
    seen: Option<u64>,
}

impl CombatReportPanel {
    /// A loaded or new session starts with its existing report already seen.
    pub fn seen(state: &game_core::travel::TravelState) -> Self {
        Self {
            open: false,
            seen: state
                .navigation
                .last_combat
                .as_ref()
                .map(|report| report.encounter),
        }
    }
}

impl LocalSession {
    #[cfg(test)]
    pub fn combat_report_open(&self) -> bool {
        self.combat_report.open
    }

    pub fn open_combat_report(&mut self) {
        self.combat_report.open = self.state.navigation.last_combat.is_some();
    }

    pub fn show_combat_report(&mut self, ctx: &egui::Context) {
        let Some(report) = self.state.navigation.last_combat.clone() else {
            return;
        };
        if self.combat_report.seen != Some(report.encounter) {
            self.combat_report.seen = Some(report.encounter);
            // Secondary fleets' fights resolve on their own: their reports go
            // to the log (and stay one tap away) instead of interrupting.
            self.combat_report.open =
                self.state.flagship(game_server::LOCAL_PLAYER) == Some(report.fleet);
        }
        let mut open = self.combat_report.open;
        crate::theme::metal::window(ctx, "Combat report", &mut open, 560.0, |ui| {
            report_ui(ui, &report)
        });
        self.combat_report.open = open;
    }
}

fn report_ui(ui: &mut egui::Ui, report: &CombatReport) {
    let color = match report.outcome {
        CombatOutcome::Victory => GREEN,
        CombatOutcome::Defeat => AMBER,
        CombatOutcome::Disengaged => CYAN,
        CombatOutcome::Surrendered => AMBER,
    };
    ui.horizontal_wrapped(|ui| {
        icons::glyph(ui, Icon::Weapon, color, "Engagement outcome");
        ui.heading(egui::RichText::new(report.outcome.label()).color(color));
        ui.label(format!(
            "Run {} · {} rounds",
            report.run,
            report.rounds.len()
        ));
    });
    ui.horizontal_wrapped(|ui| {
        if report.bounty.0 > 0 {
            stat(
                ui,
                Icon::Credits,
                GREEN,
                &format!("+{} cr", report.bounty),
                "Bounty paid for the raiders destroyed",
            );
        }
        if report.rescue_reward.0 > 0 {
            stat(
                ui,
                Icon::Credits,
                GREEN,
                &format!("+{} cr", report.rescue_reward),
                "Paid by the trader you rescued",
            );
        }
        if report.salvage_ore > 0 {
            stat(
                ui,
                Icon::Good(game_core::economy::Commodity::Ore),
                GREEN,
                &format!("+{}", report.salvage_ore),
                "Ore salvaged from raider wrecks",
            );
        }
        if report.plunder > 0 {
            stat(
                ui,
                Icon::Cargo,
                GREEN,
                &format!("+{}", report.plunder),
                "Cargo plundered from the trader's hold",
            );
        }
        if report.cargo_lost > 0 {
            stat(
                ui,
                Icon::Cargo,
                AMBER,
                &format!("−{}", report.cargo_lost),
                "Cargo lost with destroyed ships, or seized on surrender",
            );
        }
    });
    if let Some(ship) = report.crippled
        && let Some(entry) = report
            .combatants
            .iter()
            .find(|entry| entry.ship == Some(ship))
    {
        ui.colored_label(
            AMBER,
            format!(
                "{} was your last ship. With it gone, the campaign is over.",
                entry.name
            ),
        );
    }
    for (side, title) in [(Side::Player, "Your fleet"), (Side::Hostile, "Raiders")] {
        ui.separator();
        ui.strong(title);
        for entry in report.side(side) {
            combatant_row(ui, entry);
        }
    }
    ui.separator();
    egui::CollapsingHeader::new("Round log")
        .default_open(false)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| round_log(ui, report));
        });
}

fn combatant_row(ui: &mut egui::Ui, entry: &CombatantResult) {
    ui.horizontal_wrapped(|ui| {
        let (state, color) = if entry.destroyed {
            ("Destroyed", AMBER)
        } else {
            ("Operational", GREEN)
        };
        ui.colored_label(color, format!("{} · {state}", entry.name));
        stat(
            ui,
            Icon::Hull,
            color,
            &format!("{}/{}", entry.hull - entry.final_damage, entry.hull),
            &format!(
                "Hull remaining / class hull (lost {} this fight)",
                entry.final_damage - entry.starting_damage
            ),
        );
        stat(
            ui,
            Icon::Weapon,
            CYAN,
            &format!("{}/{}", entry.hits, entry.shots),
            "Hits / shots fired",
        );
        stat(
            ui,
            Icon::Armor,
            CYAN,
            &entry.hull_dealt.to_string(),
            "Hull damage inflicted",
        );
    });
}

/// One line per attacker per round, aggregating its shots for readability.
fn round_log(ui: &mut egui::Ui, report: &CombatReport) {
    for (round, shots) in report.rounds.iter().enumerate() {
        ui.strong(format!("Round {}", round + 1));
        let mut attackers: Vec<u8> = shots.iter().map(|shot| shot.attacker).collect();
        attackers.dedup();
        for attacker in attackers {
            let fired: Vec<_> = shots
                .iter()
                .filter(|shot| shot.attacker == attacker)
                .collect();
            let hits = fired.iter().filter(|shot| shot.hit).count();
            let shield: u32 = fired.iter().map(|shot| shot.shield_damage).sum();
            let hull: u32 = fired.iter().map(|shot| shot.hull_damage).sum();
            let name = |index: u8| report.combatants[usize::from(index)].name.as_str();
            let mut line = format!(
                "{}: {hits}/{} hits, −{shield} shields, −{hull} hull",
                name(attacker),
                fired.len()
            );
            for shot in fired.iter().filter(|shot| shot.destroyed) {
                line.push_str(&format!(" · destroyed {}", name(shot.target)));
            }
            ui.label(line);
        }
    }
}
