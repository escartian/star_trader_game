//! Fleet symbols are grouped, clickable and separated from all star/body silhouettes.
use super::{KM_PER_VIEW_UNIT, LocalView, display_position, kilometers, scale::SymbolScale};
use crate::map::fleets;
use egui::{Rect, Vec2};
use game_core::{
    BodyKind,
    fleet::Fleet,
    travel::{Location, Ship},
};
use glam::DVec3;

impl LocalView {
    pub fn focus_ship(&mut self, ship: &Ship) {
        if ship.location.system() == Some(self.system.id) {
            self.selected = match ship.location {
                Location::Docked(_) => self
                    .system
                    .bodies
                    .iter()
                    .find(|body| body.kind == BodyKind::Station)
                    .map(|body| body.id),
                Location::Orbiting { body, .. } => Some(body),
                _ => None,
            };
        }
        if let Some(anchor) = self.ship_anchor(&ship.location) {
            self.camera.focus_on(anchor);
        }
    }

    pub(super) fn ship_anchor(&self, location: &Location) -> Option<DVec3> {
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

    pub(super) fn draw_fleets(
        &mut self,
        painter: &egui::Painter,
        rect: Rect,
        ships: &[Ship],
        records: &[Fleet],
        label_bounds: &[Rect],
        marker_bounds: &[Rect],
    ) {
        self.fleet_hits.clear();
        let Some(art) = &self.art else {
            return;
        };
        let mut reserved = label_bounds.to_vec();
        reserved.extend_from_slice(marker_bounds);
        let radius = SymbolScale::new(self.camera.zoom).star_radius(self.system.star_kind) + 24.0;
        reserved.push(Rect::from_center_size(
            self.camera.project(DVec3::ZERO, rect),
            Vec2::splat(radius * 2.0),
        ));
        for fleet in records {
            let Some(ship) = fleet
                .ships
                .first()
                .and_then(|id| ships.iter().find(|ship| ship.id == *id))
            else {
                continue;
            };
            let Some(anchor) = self.ship_anchor(&ship.location) else {
                continue;
            };
            let anchor = self.camera.project(anchor, rect);
            if let Some(hit) = fleets::paint(
                painter,
                art,
                rect,
                anchor,
                fleets::FleetSymbol {
                    fleet,
                    ship,
                    selected: fleet.ships.contains(&self.active_ship),
                },
                &mut reserved,
            ) {
                self.fleet_hits.push(hit);
            }
        }
    }
}
