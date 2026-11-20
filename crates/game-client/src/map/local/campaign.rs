//! Campaign marks on the system map: the Raider Haven as a red fortress beside
//! its body, and shipping lanes (routes between two of your chartered ports)
//! as bold cyan lines between those bodies.
use super::{KM_PER_VIEW_UNIT, LocalView, kilometers};
use crate::theme::CYAN;
use egui::{Color32, Painter, Rect, Stroke, Vec2};
use game_core::{economy::MarketId, navigation::RaiderHaven};

const HAVEN: Color32 = Color32::from_rgb(255, 96, 80);
/// Screen offset of the Haven's mark from its body, in points.
const HAVEN_OFFSET: Vec2 = Vec2::new(-22.0, -22.0);

/// What the campaign adds to the map (copied from the session each frame).
#[derive(Clone, Debug, Default)]
pub struct CampaignMarks {
    pub haven: Option<RaiderHaven>,
    /// Ports you hold the charter of.
    pub chartered: Vec<MarketId>,
}

impl LocalView {
    pub fn set_campaign_marks(&mut self, marks: CampaignMarks) {
        self.campaign = marks;
    }

    pub(super) fn draw_campaign(&self, painter: &Painter, rect: Rect) {
        let at_body = |body| {
            self.system
                .bodies
                .iter()
                .find(|known| known.id == body)
                .map(|known| {
                    self.camera
                        .project(kilometers(known.position) / KM_PER_VIEW_UNIT, rect)
                })
        };
        let ports: Vec<_> = self
            .campaign
            .chartered
            .iter()
            .filter(|market| market.system == self.system.id)
            .filter_map(|market| at_body(market.body))
            .collect();
        for (index, from) in ports.iter().enumerate() {
            for to in &ports[index + 1..] {
                painter.line_segment(
                    [*from, *to],
                    Stroke::new(4.0_f32, CYAN.gamma_multiply(0.35)),
                );
                painter.line_segment([*from, *to], Stroke::new(1.5_f32, CYAN));
            }
        }
        let Some(haven) = self
            .campaign
            .haven
            .as_ref()
            .filter(|haven| haven.system == self.system.id)
        else {
            return;
        };
        let Some(body) = at_body(haven.body) else {
            return;
        };
        let at = body + HAVEN_OFFSET;
        fortress(painter, at, haven.standing());
        let label = if haven.standing() {
            format!("RAIDER HAVEN · {}", super::raiders::crew(&haven.garrison))
        } else {
            "RAIDER HAVEN · DESTROYED".to_owned()
        };
        let galley = painter.layout_no_wrap(label, egui::FontId::proportional(11.0), HAVEN);
        let origin = at + Vec2::new(-galley.size().x / 2.0, -26.0);
        painter.rect_filled(
            Rect::from_min_size(origin, galley.size()).expand(2.0),
            2.0,
            Color32::from_black_alpha(170),
        );
        painter.galley(origin, galley, HAVEN);
    }
}

/// A small crenellated tower; struck through once destroyed.
pub(crate) fn fortress(painter: &Painter, at: egui::Pos2, standing: bool) {
    let color = if standing { HAVEN } else { Color32::GRAY };
    let body = Rect::from_center_size(at, Vec2::new(16.0, 14.0));
    painter.rect_filled(body, 1.0, color.gamma_multiply(0.35));
    painter.rect_stroke(
        body,
        1.0,
        Stroke::new(1.6_f32, color),
        egui::StrokeKind::Middle,
    );
    for step in 0..3 {
        let x = body.left() + 1.0 + step as f32 * 6.0;
        let merlon = Rect::from_min_size(egui::pos2(x, body.top() - 4.0), Vec2::new(3.0, 4.0));
        painter.rect_filled(merlon, 0.0, color);
    }
    if !standing {
        painter.line_segment(
            [body.left_bottom(), body.right_top()],
            Stroke::new(2.0_f32, Color32::GRAY),
        );
    }
}
