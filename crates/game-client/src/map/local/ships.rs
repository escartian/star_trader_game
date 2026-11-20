//! Fleet symbols are grouped, clickable and separated from all star/body silhouettes.
use super::{KM_PER_VIEW_UNIT, LocalView, display_position, kilometers, scale::SymbolScale};
use crate::map::fleets;
use egui::Rect;
use game_core::{
    BodyKind,
    fleet::Fleet,
    movement::SpacePosition,
    travel::{Location, Ship},
};
use glam::DVec3;

impl LocalView {
    /// Ships the authority reports as able to trade where they are.
    pub fn set_trade_ready(&mut self, ships: Vec<game_core::travel::ShipId>) {
        self.trade_ready = ships;
    }

    /// Where each own fleet under way is heading, in galaxy-space kilometres.
    pub fn set_headings(&mut self, headings: Vec<(game_core::fleet::FleetId, SpacePosition)>) {
        self.headings = headings;
    }

    /// Fleets paused by an unresolved contact, for the marker badge.
    pub fn set_contacts(&mut self, fleets: Vec<game_core::fleet::FleetId>) {
        self.contacts = fleets;
    }

    pub fn focus_ship(&mut self, ship: &Ship) {
        if ship.location.system() == Some(self.system.id) {
            self.selected = match ship.location {
                Location::Docked(_) => self
                    .system
                    .bodies
                    .iter()
                    .find(|body| body.kind == BodyKind::Station)
                    .map(|body| body.id),
                Location::Orbiting { body, .. } | Location::Landed { body, .. } => Some(body),
                _ => None,
            };
        }
        // Navigation following retains the star and destinations as spatial context.
        // Explicit player focus remains available for close inspection.
        self.frame_overview = true;
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
            Location::Orbiting { body, .. } | Location::Landed { body, .. } => {
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
        labels: &[Rect],
    ) {
        self.fleet_hits.clear();
        let Some(art) = &self.art else {
            return;
        };
        let scale = SymbolScale::new(self.camera.zoom);
        let centre = SpacePosition::from_galaxy(self.system.position).ok();
        let mut entries = Vec::new();
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
            let body = match ship.location {
                Location::Docked(_) => self
                    .system
                    .bodies
                    .iter()
                    .find(|body| body.kind == BodyKind::Station),
                Location::Orbiting { body, .. } | Location::Landed { body, .. } => {
                    self.system.bodies.iter().find(|known| known.id == body)
                }
                _ => None,
            };
            let clearance = body.map_or(0.0, |body| scale.body_radius(body.kind));
            let ring = match ship.location {
                Location::Orbiting { .. } => clearance + 16.0,
                _ => 0.0,
            };
            let heading = centre.and_then(|centre| {
                let (_, target) = self.headings.iter().find(|(id, _)| *id == fleet.id)?;
                let local = game_core::LocalPosition {
                    x_km: target.x_km - centre.x_km,
                    y_km: target.y_km - centre.y_km,
                    z_km: target.z_km - centre.z_km,
                };
                Some(
                    self.camera
                        .project(kilometers(local) / KM_PER_VIEW_UNIT, rect),
                )
            });
            entries.push(fleets::StackEntry {
                fleet,
                ship,
                anchor: self.camera.project(anchor, rect),
                berth: fleets::Berth::new(&ship.location, clearance),
                contact: self.contacts.contains(&fleet.id),
                heading,
                ring,
            });
        }
        let selected = records
            .iter()
            .find(|fleet| fleet.ships.contains(&self.active_ship))
            .map(|fleet| fleet.id);
        self.fleet_hits = fleets::paint_stacks(painter, art, rect, &entries, selected, labels);
    }
}
