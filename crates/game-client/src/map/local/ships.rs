//! Ship illustrations are tethered to authoritative body/waypoint anchors.
use super::{KM_PER_VIEW_UNIT, LocalView, display_position, kilometers};
use crate::map::art::ArtKind;
use egui::{Color32, Painter, Rect, Stroke, Vec2};
use game_core::{
    BodyKind,
    travel::{Location, Ship},
};

impl LocalView {
    pub fn focus_ship(&mut self, ship: &Ship) {
        if let Some(anchor) = self.ship_anchor(&ship.location) {
            self.camera.focus_on(anchor);
        }
    }

    pub(super) fn ship_anchor(&self, location: &Location) -> Option<glam::DVec3> {
        if location.system() != Some(self.system.id) || !location.stationary() {
            return None;
        }
        let body = match location {
            Location::Docked(_) => self
                .system
                .bodies
                .iter()
                .find(|body| body.kind == BodyKind::Station),
            Location::Orbiting { body, .. } => {
                self.system.bodies.iter().find(|known| known.id == *body)
            }
            _ => None,
        };
        if let Some(body) = body {
            Some(display_position(&self.system, body))
        } else if let Location::InSystem { position, .. } = location {
            Some(kilometers(*position) / KM_PER_VIEW_UNIT)
        } else {
            None
        }
    }

    pub(super) fn draw_ships(
        &self,
        painter: &Painter,
        rect: Rect,
        ships: &[Ship],
        label_bounds: &[Rect],
        marker_bounds: &[Rect],
    ) {
        let mut reserved = label_bounds.to_vec();
        reserved.extend_from_slice(marker_bounds);
        for ship in ships.iter().filter(|ship| {
            ship.location.system() == Some(self.system.id) && ship.location.stationary()
        }) {
            let Some(anchor) = self.ship_anchor(&ship.location) else {
                continue;
            };
            let anchor = self.camera.project(anchor, rect);
            let offsets = [
                Vec2::new(50.0, -45.0),
                Vec2::new(-50.0, 45.0),
                Vec2::new(50.0, 45.0),
                Vec2::new(-50.0, -45.0),
                Vec2::new(85.0, -65.0),
                Vec2::new(-85.0, 65.0),
            ];
            let point = offsets
                .into_iter()
                .map(|offset| anchor + offset)
                .find(|point| {
                    let bounds = Rect::from_center_size(*point, Vec2::new(42.0, 60.0));
                    rect.contains_rect(bounds)
                        && !reserved
                            .iter()
                            .any(|other| other.expand(4.0).intersects(bounds))
                })
                .unwrap_or(anchor + Vec2::new(50.0, 45.0));
            reserved.push(Rect::from_center_size(point, Vec2::new(42.0, 60.0)));
            painter.line_segment([anchor, point], Stroke::new(0.5_f32, Color32::LIGHT_GREEN));
            painter.circle_stroke(anchor, 3.0, Stroke::new(1.0_f32, Color32::LIGHT_GREEN));
            if ship.id == self.active_ship {
                painter.circle_stroke(point, 19.0, Stroke::new(1.0_f32, Color32::LIGHT_GREEN));
            }
            if let Some(art) = &self.art {
                art.paint(ArtKind::from(ship.kind), painter, point, 32.0);
            }
            painter.text(
                point + Vec2::new(0.0, 17.0),
                egui::Align2::CENTER_TOP,
                format!("#{}", ship.id.0),
                egui::FontId::proportional(11.0),
                Color32::LIGHT_GREEN,
            );
        }
    }
}
