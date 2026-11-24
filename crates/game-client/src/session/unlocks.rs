//! Milestone tips on the client: every control is shown from the start; each
//! milestone the host records (`game_core::progress`) brings a one-line guide
//! card to the top of the sheet until dismissed.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{CYAN, GREEN};
use egui::RichText;
use game_core::progress::Feature;

impl LocalSession {
    /// Queues the guide card for a feature the host just unlocked.
    pub(super) fn announce_unlock(&mut self, feature: Feature) -> String {
        if !self.unlock_cards.contains(&feature) {
            self.unlock_cards.push(feature);
        }
        format!("Tip: {}.", feature.guide().0)
    }

    /// Whether the guide card for `feature` is still showing.
    pub(super) fn unlock_card_open(&self, feature: Feature) -> bool {
        self.unlock_cards.contains(&feature)
    }

    /// The pending unlock cards, newest first, each with a dismiss button.
    pub(super) fn unlock_cards(&mut self, ui: &mut egui::Ui) {
        let mut dismissed = None;
        for feature in self.unlock_cards.iter().rev() {
            let (title, line) = feature.guide();
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.0_f32, GREEN.gamma_multiply(0.6)))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        icons::glyph(ui, Icon::Star, GREEN, "New");
                        ui.label(
                            RichText::new(format!("NEW · {title}"))
                                .color(GREEN)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::icon_button(ui, Icon::Cancel, "Got it").clicked() {
                                dismissed = Some(*feature);
                            }
                        });
                    });
                    ui.label(RichText::new(line).small().color(CYAN));
                });
        }
        if let Some(feature) = dismissed {
            self.unlock_cards.retain(|card| *card != feature);
        }
    }
}
