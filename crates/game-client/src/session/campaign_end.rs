//! The campaign's end screen. Over a dimmed map it shows an illustrated
//! card: how it ended, runs played, the score and its parts, net worth, each
//! road's progress and faction standings. A win offers Keep playing; a defeat
//! or a completed campaign offers a new game, a save to load, the main menu,
//! or a last look at the frozen map. Shown once per end, after any open
//! combat report. Display only.
use super::LocalSession;
use super::factions::tier_color;
use crate::theme::event_art::{self, Illustration, Mood};
use crate::theme::event_card;
use crate::theme::icons::Icon;
use crate::theme::{AMBER, GREEN, metal};
use egui::{Color32, RichText, Vec2};
use game_core::campaign::{self, CampaignEnd};
use game_core::combat::Side;
use game_server::LOCAL_PLAYER;

/// What the player picked on the end screen.
enum Exit {
    KeepPlaying,
    NewGame,
    Load,
    MainMenu,
    ViewMap,
}

impl LocalSession {
    pub(super) fn campaign_end_screen(&mut self, ctx: &egui::Context) {
        let Some(end) = self.state.campaign.ended else {
            return;
        };
        if self.victory.seen_end == Some(end) || self.battle_open() || self.combat_report.open {
            return;
        }
        let art = self.end_art(end);
        let (title, mood) = match end {
            CampaignEnd::Won { .. } => ("VICTORY", Mood::Triumph),
            CampaignEnd::Lost { .. } => ("GAME OVER", Mood::Loss),
            CampaignEnd::TimeUp { .. } => ("CAMPAIGN COMPLETE", Mood::Triumph),
        };
        let screen = ctx.screen_rect();
        let width = 520.0_f32.min(screen.width() - 40.0);
        let mut exit = None;
        egui::Modal::new(egui::Id::new("campaign-end"))
            .frame(egui::Frame::NONE)
            .backdrop_color(Color32::from_black_alpha(190))
            .show(ctx, |ui| {
                metal::frame(ui, 255, |ui| {
                    ui.set_width(width);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(title)
                                .font(crate::theme::fonts::display(ui.ctx(), 28.0))
                                .color(mood.accent()),
                        );
                    });
                    let height = if screen.width() < metal::FULL_SCREEN_BELOW {
                        130.0
                    } else {
                        170.0
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                    event_art::paint(&ui.painter_at(rect), rect, &art);
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .max_height(screen.height() * 0.5)
                        .show(ui, |ui| self.end_summary(ui, end));
                    exit = end_choices(ui, end);
                });
            });
        let Some(exit) = exit else {
            return;
        };
        self.victory.seen_end = Some(end);
        match exit {
            Exit::KeepPlaying => self.victory.open = true,
            Exit::NewGame => self.saves.new_open = true,
            Exit::Load => self.saves.open = true,
            // The game is over, so there is nothing worth saving over the autosave.
            Exit::MainMenu => self.leave_to_main_menu(false),
            Exit::ViewMap => {}
        }
    }

    /// Lost: the last ship, dimmed, facing what destroyed it. Otherwise your
    /// fleet before a station.
    fn end_art(&self, end: CampaignEnd) -> Illustration {
        let run = match end {
            CampaignEnd::Won { run, .. }
            | CampaignEnd::Lost { run }
            | CampaignEnd::TimeUp { run } => run,
        };
        let report = self.state.navigation.last_combat.as_deref();
        if let CampaignEnd::Lost { .. } = end {
            let mut art = Illustration::new(Mood::Loss, run);
            if let Some(report) = report {
                art.ours = report
                    .combatants
                    .iter()
                    .filter(|entry| entry.ship.is_some() && entry.ship == report.crippled)
                    .map(|entry| entry.kind)
                    .collect();
                art.theirs = report.side(Side::Hostile).map(|entry| entry.kind).collect();
            }
            return art;
        }
        let mut art = Illustration::new(Mood::Triumph, run);
        art.station = true;
        art.ours = self
            .state
            .ships
            .iter()
            .map(|ship| ship.kind)
            .take(3)
            .collect();
        art
    }

    fn end_summary(&self, ui: &mut egui::Ui, end: CampaignEnd) {
        ui.label(RichText::new(end.label()).strong());
        if let CampaignEnd::Lost { .. } = end
            && let Some(report) = self.state.navigation.last_combat.as_deref()
            && let Some(lost) = report
                .combatants
                .iter()
                .find(|entry| entry.ship.is_some() && entry.ship == report.crippled)
        {
            let foes = report.side(Side::Hostile).count();
            ui.label(format!(
                "{} went down fighting {foes} hostile ship{}. With no other fleet left \
                 to carry on, your trading house is finished.",
                lost.name,
                if foes == 1 { "" } else { "s" }
            ));
        }
        ui.add_space(6.0);
        let score = campaign::score(&self.state, &self.economy, LOCAL_PLAYER);
        let worth = campaign::net_worth(&self.state, &self.economy, LOCAL_PLAYER);
        egui::Grid::new("campaign-end-stats")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                let mut row = |name: &str, value: String| {
                    ui.label(RichText::new(name).color(Color32::GRAY));
                    ui.label(value);
                    ui.end_row();
                };
                row("Runs played", self.state.navigation.run.to_string());
                row("Net worth", format!("{worth} cr"));
                row(
                    "Score",
                    format!(
                        "{}  (net worth {} · charters {} · Haven {} · diplomacy {})",
                        score.total(),
                        score.net_worth,
                        score.charters,
                        score.haven,
                        score.diplomacy
                    ),
                );
            });
        ui.add_space(6.0);
        ui.label(
            RichText::new("ROADS TO VICTORY")
                .small()
                .color(Color32::GRAY),
        );
        for road in campaign::progress(&self.state, &self.economy, LOCAL_PLAYER) {
            let color = if road.complete {
                GREEN
            } else {
                Color32::LIGHT_GRAY
            };
            let state = if road.open {
                format!("{}%", road.percent)
            } else {
                "locked".into()
            };
            ui.label(RichText::new(format!("{} · {state}", road.road.label())).color(color));
        }
        let factions = &self.economy.factions;
        if factions.is_active() {
            ui.add_space(6.0);
            ui.label(RichText::new("STANDING").small().color(Color32::GRAY));
            ui.horizontal_wrapped(|ui| {
                for faction in &factions.in_play {
                    let tier = factions.tier(*faction);
                    ui.label(
                        RichText::new(format!(
                            "{} {:+} {}",
                            faction.label(),
                            factions.standing(*faction),
                            tier.label()
                        ))
                        .color(tier_color(tier)),
                    );
                }
            });
        }
    }
}

fn end_choices(ui: &mut egui::Ui, end: CampaignEnd) -> Option<Exit> {
    let mut exit = None;
    event_card::choices(ui, |ui| {
        if end.playable() {
            if event_card::choice(ui, Icon::Execute, "Keep playing").clicked() {
                exit = Some(Exit::KeepPlaying);
            }
            return;
        }
        if event_card::choice(ui, Icon::New, "Start a new game").clicked() {
            exit = Some(Exit::NewGame);
        }
        if event_card::choice(ui, Icon::Load, "Load a save").clicked() {
            exit = Some(Exit::Load);
        }
        if event_card::choice(ui, Icon::Back, "Main menu").clicked() {
            exit = Some(Exit::MainMenu);
        }
        if event_card::choice(ui, Icon::Overview, "View the final map")
            .on_hover_text("The game is frozen; Menu still offers a new game or a save to load.")
            .clicked()
        {
            exit = Some(Exit::ViewMap);
        }
    });
    if !end.playable() {
        ui.small(RichText::new("Orders are closed. Saves and loads still work.").color(AMBER));
    }
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect};

    fn frame(
        ctx: &egui::Context,
        session: &mut LocalSession,
        events: Vec<egui::Event>,
    ) -> Vec<(String, Pos2)> {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0))),
                events,
                ..Default::default()
            },
            |ctx| session.show_victory(ctx, &[]),
        );
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some((
                    text.galley.job.text.clone(),
                    text.pos + text.galley.size() / 2.0,
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn losing_the_last_ship_shows_game_over_and_offers_a_new_game() {
        let ctx = egui::Context::default();
        let mut session = LocalSession::new(game_server::Session::default());
        session.state.campaign.ended = Some(CampaignEnd::Lost { run: 4 });
        let mut texts = Vec::new();
        for _ in 0..3 {
            texts = frame(&ctx, &mut session, Vec::new());
        }
        let shown: Vec<_> = texts.iter().map(|(text, _)| text.as_str()).collect();
        assert!(shown.contains(&"GAME OVER"), "{shown:?}");
        assert!(!shown.contains(&"Keep playing"), "{shown:?}");
        let (_, point) = texts
            .iter()
            .find(|(text, _)| text == "Start a new game")
            .cloned()
            .expect("a new-game choice");
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut session,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(session.saves.new_open);
        let after = frame(&ctx, &mut session, Vec::new());
        assert!(!after.iter().any(|(text, _)| text == "GAME OVER"));
    }

    #[test]
    fn a_loaded_lost_game_shows_its_end_again_but_a_win_does_not() {
        let mut state = LocalSession::new(game_server::Session::default()).state;
        state.campaign.ended = Some(CampaignEnd::Lost { run: 4 });
        assert_eq!(
            super::super::victory::VictoryPanel::seen(&state).seen_end,
            None
        );
        let win = CampaignEnd::Won {
            road: campaign::Road::Trader,
            run: 4,
        };
        state.campaign.ended = Some(win);
        assert_eq!(
            super::super::victory::VictoryPanel::seen(&state).seen_end,
            Some(win)
        );
    }
}
