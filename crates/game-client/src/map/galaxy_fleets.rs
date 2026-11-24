//! What the galaxy map draws besides systems: own fleets as stacked chips (all
//! fleets in one system share a chip), NPC traders as small 3D ship models, and a
//! red marker beside each system that holds a raider band (or, for a band
//! crossing to another system, on the line between the two).
use super::{Map, position};
use super::{
    art::{ArtKind, Artwork},
    fleets::{Berth, StackEntry},
};
use egui::{Painter, Rect, Stroke, Vec2};

/// Points between a system's marker and the chips resting there.
const SYSTEM_CLEARANCE: f32 = 12.0;

impl Map {
    pub(super) fn draw_galaxy_fleets(
        &mut self,
        ui: &egui::Ui,
        painter: &Painter,
        rect: Rect,
        labels: &[Rect],
    ) {
        let art = super::art::Artwork::preload(ui.ctx());
        let contacts = self.session.contact_fleets();
        let headings = self.session.fleet_headings(&self.systems);
        let selected = self
            .session
            .fleets()
            .iter()
            .find(|fleet| fleet.ships.contains(&self.session.selected_ship().id))
            .map(|fleet| fleet.id);
        let mut entries = Vec::new();
        for fleet in self.session.fleets() {
            let Some(ship) = fleet
                .ships
                .first()
                .and_then(|id| self.session.ships().iter().find(|ship| ship.id == *id))
            else {
                continue;
            };
            // In a system, every fleet rests beside its star; in flight, on its course.
            let (anchor, berth) = match ship.location.system() {
                Some(system) => {
                    let Some(world) = self.systems.iter().find(|known| known.id == system) else {
                        continue;
                    };
                    let berth = Berth::AtBody {
                        clearance: SYSTEM_CLEARANCE,
                        landed: false,
                    };
                    (self.camera.project(position(world), rect), berth)
                }
                None => {
                    let Some(world) = self.session.display_ship_position(ship.id, &self.systems)
                    else {
                        continue;
                    };
                    (self.camera.project(world, rect), Berth::Free)
                }
            };
            let heading = headings
                .iter()
                .find(|(id, _)| *id == fleet.id)
                .map(|(_, target)| {
                    self.camera
                        .project(crate::session::light_years(*target), rect)
                });
            entries.push(StackEntry {
                fleet,
                ship,
                anchor,
                berth,
                contact: contacts.contains(&fleet.id),
                heading,
                ring: 0.0,
            });
        }
        self.galaxy_fleet_hits =
            super::fleets::paint_stacks(painter, &art, rect, &entries, selected, labels);
    }

    /// NPC traders use the same GLB hull models as battle, with a small
    /// outline in their faction's colour (grey if independent) to
    /// distinguish them from owned ships.
    pub(super) fn draw_galaxy_threats(&self, ui: &egui::Ui, painter: &Painter, rect: Rect) {
        let art = Artwork::preload(ui.ctx());
        for trader in self.session.traffic() {
            if let Some(world) = self.session.trader_display_position(trader, &self.systems) {
                let at = self.camera.project(world, rect);
                if !rect.expand(12.0).contains(at) {
                    continue;
                }
                let outline = self.session.trader_color(trader);
                painter.circle_filled(at, 12.0, egui::Color32::from_black_alpha(190));
                painter.circle_stroke(at, 12.0, Stroke::new(1.0_f32, outline));
                if let Some(ship) = trader.state.ships.first() {
                    if let Some(model) = crate::battle::model::for_kind(ship.kind) {
                        model.paint_marker(painter, at, 22.0);
                    } else if !art.paint(ArtKind::from(ship.kind), painter, at, 20.0) {
                        painter.circle_filled(at, 3.0, outline);
                    }
                }
            }
        }
        let red = crate::battle::scene::HOSTILE;
        let find = |id| self.systems.iter().find(|known| known.id == id);
        for band in self.session.raider_bands() {
            let Some(system) = find(band.system) else {
                continue;
            };
            let destination = self.camera.project(position(system), rect);
            let at = match band
                .transit
                .and_then(|transit| Some((transit, find(transit.from)?)))
            {
                // Crossing: along a faint line from its origin to its destination.
                Some((transit, origin)) => {
                    let origin = self.camera.project(position(origin), rect);
                    painter.line_segment(
                        [origin, destination],
                        Stroke::new(1.0_f32, red.gamma_multiply(0.45)),
                    );
                    origin.lerp(destination, transit.progress())
                }
                None => destination + Vec2::new(-15.0, -15.0),
            };
            let points = vec![
                at + Vec2::new(0.0, -5.0),
                at + Vec2::new(5.0, 0.0),
                at + Vec2::new(0.0, 5.0),
                at + Vec2::new(-5.0, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                red.gamma_multiply(0.5),
                Stroke::new(1.0_f32, red),
            ));
        }
    }
}
