//! What the galaxy map draws besides systems: own fleets as stacked chips (all
//! fleets in one system share a chip, tethered beside its star; in flight, a
//! faint green line to their heading), NPC traders as small ship markers, and
//! a red marker beside each system that holds a raider band (or, for a band
//! crossing to another system, on the red line between the two).
use super::{Map, position};
use super::{
    art::Artwork,
    fleets::{Berth, StackEntry},
    trader_marker,
};
use egui::{Painter, Rect, Stroke, Vec2};

/// Points between a system's marker and the chips resting there.
const SYSTEM_CLEARANCE: f32 = 12.0;
/// A star marker's radius in points, where resting traders' tethers start.
const STAR_CLEARANCE: f32 = 10.0;

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
        // Own fleets in flight: a faint green line to where they are heading.
        for entry in &entries {
            if let (Berth::Free, Some(heading)) = (entry.berth, entry.heading) {
                super::local::dash(
                    painter,
                    entry.anchor,
                    heading,
                    Stroke::new(1.2_f32, crate::theme::iff::FRIENDLY.gamma_multiply(0.5)),
                );
            }
        }
        self.galaxy_fleet_hits =
            super::fleets::paint_stacks(painter, &art, rect, &entries, selected, labels);
    }

    /// NPC traders (`trader_marker`): resting ones spread around their star
    /// on short tethers, flying ones on their gliding position with a faint
    /// dashed line, in their friend-or-foe colour, to the system they are
    /// bound for. Raider bands crossing between systems follow, in red.
    pub(super) fn draw_galaxy_threats(&self, ui: &egui::Ui, painter: &Painter, rect: Rect) {
        let art = Artwork::preload(ui.ctx());
        let gliding = self.session.gliding();
        let mut resting: Vec<(game_core::SystemId, usize)> = Vec::new();
        for trader in self.session.traffic() {
            let Some(ship) = trader.state.ships.first() else {
                continue;
            };
            let ring = self.session.trader_stance(trader).color();
            let star = ship
                .location
                .system()
                .filter(|_| ship.location.stationary() && !gliding)
                .and_then(|id| self.systems.iter().find(|known| known.id == id));
            let at = match star {
                Some(system) => {
                    let count = match resting.iter_mut().find(|(known, _)| *known == system.id) {
                        Some((_, count)) => {
                            *count += 1;
                            *count - 1
                        }
                        None => {
                            resting.push((system.id, 1));
                            0
                        }
                    };
                    let anchor = self.camera.project(position(system), rect);
                    // Clear of the star and of own chips (lower left).
                    let at = trader_marker::berth_spot(anchor, STAR_CLEARANCE, count);
                    super::fleets::tether(
                        painter,
                        anchor,
                        STAR_CLEARANCE,
                        at,
                        trader_marker::TRADER,
                        ring,
                    );
                    at
                }
                None => {
                    let Some(world) = self.session.trader_display_position(trader, &self.systems)
                    else {
                        continue;
                    };
                    let at = self.camera.project(world, rect);
                    if let Some(bound) = self
                        .session
                        .trader_heading(trader)
                        .and_then(|id| self.systems.iter().find(|known| known.id == id))
                    {
                        super::local::dash(
                            painter,
                            at,
                            self.camera.project(position(bound), rect),
                            Stroke::new(1.0_f32, ring.gamma_multiply(0.45)),
                        );
                    }
                    at
                }
            };
            if rect.expand(trader_marker::TRADER).contains(at) {
                trader_marker::paint(
                    painter,
                    &art,
                    at,
                    ship.kind,
                    ring,
                    self.session.trader_color(trader),
                );
            }
        }
        let red = crate::theme::iff::HOSTILE;
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
