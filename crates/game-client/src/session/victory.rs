//! The Victory panel (Menu → Victory): progress on every road to victory, the
//! score and its parts, the Raider Haven's whereabouts, chartered ports and
//! the campaign length. When the campaign ends, an end card opens once.
//! Display only: the rules live in `game_core::campaign` and every change is
//! a command.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::{
    StarSystem,
    campaign::{self, CampaignEnd, RUN_LIMIT_CHOICES},
    travel::{Location, TravelState},
};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

const HOSTILE: Color32 = crate::battle::scene::HOSTILE;

#[derive(Default)]
pub(super) struct VictoryPanel {
    pub(super) open: bool,
    /// The campaign end the player has already been shown.
    seen_end: Option<CampaignEnd>,
}

impl VictoryPanel {
    /// A loaded session's existing end is not announced again.
    pub(super) fn seen(state: &TravelState) -> Self {
        Self {
            open: false,
            seen_end: state.campaign.ended,
        }
    }
}

/// "Body, System" for a body, or the system alone.
fn place_name(
    systems: &[StarSystem],
    system: game_core::SystemId,
    body: game_core::BodyId,
) -> String {
    let Some(world) = systems.iter().find(|known| known.id == system) else {
        return "an unknown system".into();
    };
    match world.bodies.iter().find(|known| known.id == body) {
        // Generated body names often start with their system's name already.
        Some(body) if body.name.starts_with(&world.name) => body.name.clone(),
        Some(body) => format!("{}, {}", body.name, world.name),
        None => world.name.clone(),
    }
}

impl LocalSession {
    /// The Haven and your chartered ports, for the maps.
    pub fn campaign_marks(&self) -> crate::map::local::CampaignMarks {
        crate::map::local::CampaignMarks {
            haven: self.state.navigation.haven.clone(),
            chartered: self.economy.chartered_by(LOCAL_PLAYER),
        }
    }

    pub fn show_victory(&mut self, ctx: &egui::Context, systems: &[StarSystem]) {
        let mut open = self.victory.open;
        metal::window(ctx, "Victory", &mut open, 520.0, |ui| {
            self.victory_ui(ui, systems)
        });
        self.victory.open = open;
        self.end_card(ctx);
    }

    fn victory_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        if let Some(end) = self.state.campaign.ended {
            let color = if end.playable() { GREEN } else { AMBER };
            ui.label(RichText::new(end.label()).color(color).strong());
            ui.separator();
        }
        let score = campaign::score(&self.state, &self.economy, LOCAL_PLAYER);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("SCORE {}", score.total()))
                    .font(crate::theme::fonts::display(ui.ctx(), 16.0))
                    .color(GREEN),
            );
            ui.label(
                RichText::new(format!(
                    "net worth {} · charters {} · Haven {}",
                    score.net_worth, score.charters, score.haven
                ))
                .small(),
            )
            .on_hover_text(
                "1 point per 100 cr of net worth (credits, hulls at resale, cargo at \
                 the best bid), 100 per charter, 20 per Haven garrison ship sunk and \
                 500 for destroying the Haven.",
            );
        });
        ui.add_space(6.0);
        for road in campaign::progress(&self.state, &self.economy, LOCAL_PLAYER) {
            road_row(ui, &road);
        }
        ui.separator();
        if let Some(haven) = &self.state.navigation.haven {
            let place = place_name(systems, haven.system, haven.body);
            let line = if haven.standing() {
                format!(
                    "Raider Haven: beside {place}, {} garrison ships. Orbit there and Assault.",
                    haven.garrison.len()
                )
            } else {
                format!("Raider Haven at {place}: destroyed.")
            };
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Danger, HOSTILE, "Raider Haven");
                ui.label(RichText::new(line).small());
            });
        }
        let charters = self.economy.chartered_by(LOCAL_PLAYER);
        let names: Vec<String> = charters
            .iter()
            .map(|market| place_name(systems, market.system, market.body))
            .collect();
        let rules = &self.economy.rules.campaign;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Pin, CYAN, "Port charters");
            let held = if names.is_empty() {
                "none yet".into()
            } else {
                names.join(" · ")
            };
            ui.label(RichText::new(format!("Charters: {held}")).small())
                .on_hover_text(format!(
                    "Buy a port's charter in its market for {} cr. It pays you {}% of \
                     every trade there; routes between two of your ports are never \
                     struck by raiders.",
                    rules.charter_price, rules.port_share_percent
                ));
        });
        ui.separator();
        self.run_limit_picker(ui);
    }

    /// Open-ended or a fixed number of runs, ending on score.
    fn run_limit_picker(&mut self, ui: &mut egui::Ui) {
        let run = self.state.navigation.run;
        let current = self.state.campaign.run_limit;
        let label = |limit: Option<u64>| match limit {
            None => "Open-ended".to_owned(),
            Some(runs) => format!("{runs} runs"),
        };
        let mut choice = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("CAMPAIGN LENGTH")
                    .small()
                    .color(Color32::GRAY),
            )
            .on_hover_text("With a length set, the game ends on score at that run.");
            egui::ComboBox::from_id_salt("run-limit")
                .selected_text(label(current))
                .show_ui(ui, |ui| {
                    for runs in RUN_LIMIT_CHOICES {
                        let limit = (runs > 0).then_some(runs);
                        if limit.is_some_and(|limit| limit <= run) {
                            continue;
                        }
                        if ui
                            .selectable_label(current == limit, label(limit))
                            .clicked()
                        {
                            choice = Some(limit);
                        }
                    }
                });
        });
        if let Some(runs) = choice.filter(|limit| *limit != current) {
            self.command(ClientCommand::SetRunLimit {
                runs,
                expected_revision: self.state.revision,
            });
        }
    }

    /// Announces a new campaign end once, with the score.
    fn end_card(&mut self, ctx: &egui::Context) {
        let Some(end) = self.state.campaign.ended else {
            return;
        };
        if self.victory.seen_end == Some(end) || self.battle_open() {
            return;
        }
        let (title, color) = match end {
            CampaignEnd::Won { .. } => ("VICTORY", GREEN),
            CampaignEnd::Lost { .. } => ("DEFEAT", HOSTILE),
            CampaignEnd::TimeUp { .. } => ("CAMPAIGN COMPLETE", AMBER),
        };
        let score = campaign::score(&self.state, &self.economy, LOCAL_PLAYER).total();
        let mut close = false;
        egui::Window::new("campaign-end")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                metal::frame(ui, 255, |ui| {
                    ui.set_width(320.0_f32.min(ctx.screen_rect().width() - 40.0));
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(title)
                                .font(crate::theme::fonts::display(ui.ctx(), 24.0))
                                .color(color),
                        );
                        ui.label(end.label());
                        ui.label(RichText::new(format!("Score {score}")).color(GREEN));
                        ui.add_space(8.0);
                        let next = if end.playable() {
                            "Keep playing"
                        } else {
                            "Close"
                        };
                        if ui
                            .add(egui::Button::new(next).min_size(Vec2::new(140.0, 40.0)))
                            .clicked()
                        {
                            close = true;
                        }
                        if !end.playable() {
                            ui.label(
                                RichText::new("Load a save or start a new game from Menu.").small(),
                            );
                        }
                    });
                });
            });
        if close {
            self.victory.seen_end = Some(end);
            self.victory.open = true;
        }
    }

    /// At the Haven: its garrison and an Assault button for the selected fleet.
    pub(super) fn haven_prompt(&mut self, ui: &mut egui::Ui) {
        let Some(haven) = self.state.navigation.haven.clone().filter(|h| h.standing()) else {
            return;
        };
        let Ok(fleet) = self
            .state
            .fleet_for_ship(self.active_ship)
            .map(|fleet| fleet.id)
        else {
            return;
        };
        let at_haven = matches!(
            self.selected_ship().location,
            Location::Orbiting { system, body } | Location::Landed { system, body }
                if system == haven.system && body == haven.body
        );
        let busy = self.battle_open() || self.contact_fleets().contains(&fleet);
        if !at_haven || busy {
            return;
        }
        let mut assault = false;
        egui::Frame::new()
            .fill(HOSTILE.gamma_multiply(0.12))
            .stroke(egui::Stroke::new(1.0_f32, HOSTILE.gamma_multiply(0.6)))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.label(RichText::new("RAIDER HAVEN").color(HOSTILE).strong());
                ui.label(
                    RichText::new(format!(
                        "Garrison: {} ships. Destroy them all to win the Soldier road; \
                         losses you inflict stay lost.",
                        haven.garrison.len()
                    ))
                    .small(),
                );
                assault = ui
                    .add(
                        egui::Button::new(RichText::new("ASSAULT").color(Color32::BLACK))
                            .fill(HOSTILE)
                            .min_size(Vec2::new(140.0, 40.0)),
                    )
                    .on_hover_text("Opens a contact: Fight, Auto-resolve, or Evade to call it off.")
                    .clicked();
            });
        if assault {
            self.command(ClientCommand::AssaultHaven {
                fleet,
                expected_revision: self.state.revision,
            });
        }
    }

    /// The market's charter: for sale (with its price and why it matters),
    /// yours, or another holder's.
    pub(super) fn charter_row(
        &mut self,
        ui: &mut egui::Ui,
        ship: &game_core::travel::Ship,
        market: game_core::economy::MarketId,
    ) {
        let rules = self.economy.rules.campaign.clone();
        let mut buy = false;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Pin, CYAN, "Port charter");
            match self.economy.charter_holder(market) {
                Some(owner) if owner == LOCAL_PLAYER => {
                    ui.label(
                        RichText::new(format!(
                            "Your charter: {}% of every trade here is yours.",
                            rules.port_share_percent
                        ))
                        .small()
                        .color(GREEN),
                    );
                }
                Some(_) => {
                    ui.label(RichText::new("This port's charter is held.").small());
                }
                None => {
                    let blocked = self
                        .economy
                        .quote_charter(ship)
                        .err()
                        .map(|e| e.to_string());
                    let label = format!("Buy charter · {} cr", rules.charter_price);
                    buy = icons::labeled_action(ui, Icon::Buy, &label, blocked)
                        .on_hover_text(format!(
                            "The port pays you {}% of every trade here, NPC traders' \
                             included. Routes between two of your ports are never struck \
                             by raiders. {} charters win the Trader road.",
                            rules.port_share_percent, rules.charters_to_win
                        ))
                        .clicked();
                }
            }
        });
        if buy {
            self.command(ClientCommand::BuyCharter {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
    }
}

fn road_row(ui: &mut egui::Ui, road: &campaign::RoadProgress) {
    let color = match (road.open, road.complete) {
        (_, true) => GREEN,
        (true, false) => CYAN,
        (false, _) => Color32::GRAY,
    };
    ui.horizontal(|ui| {
        ui.add_sized(
            [118.0, 18.0],
            egui::Label::new(RichText::new(road.road.label()).color(color).strong()),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(140.0, 8.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_black_alpha(160));
        let mut fill = rect;
        fill.set_width(rect.width() * road.percent as f32 / 100.0);
        ui.painter().rect_filled(fill, 2.0, color);
        ui.label(
            RichText::new(format!("{}%", road.percent))
                .small()
                .color(color),
        );
    });
    ui.label(RichText::new(&road.detail).small());
    ui.add_space(4.0);
}
