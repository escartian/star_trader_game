//! What the galaxy map draws besides systems: own fleets as stacked chips (all
//! fleets in one system share a chip), NPC traders as quiet amber dots, and a
//! red marker beside each system that holds a raider band.
use super::fleets::{Berth, StackEntry};
use super::{Map, position};
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

    /// NPC traders as small amber dots (no labels), and a red diamond beside each
    /// system that holds a raider band.
    pub(super) fn draw_galaxy_threats(&self, painter: &Painter, rect: Rect) {
        for trader in self.session.traffic() {
            if let Some(world) = self.session.trader_display_position(trader, &self.systems) {
                let at = self.camera.project(world, rect);
                painter.circle_filled(at, 3.0, crate::theme::AMBER.gamma_multiply(0.8));
            }
        }
        let red = crate::battle::scene::HOSTILE;
        for band in self.session.raider_bands() {
            let Some(system) = self.systems.iter().find(|known| known.id == band.system) else {
                continue;
            };
            let at = self.camera.project(position(system), rect) + Vec2::new(-15.0, -15.0);
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
