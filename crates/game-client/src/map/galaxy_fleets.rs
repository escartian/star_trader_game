//! What the galaxy map draws besides systems: own fleets as stacked chips (all
//! fleets in one system share a chip, tethered beside its star; in flight, a
//! faint green line to their heading), NPC traders as small ship markers, and
//! a red-ringed ship marker beside each system that holds a raider band (or,
//! for a band crossing to another system, on the red line between the two).
use super::{Map, position};
use super::{
    art::Artwork,
    fleets::{Berth, StackEntry},
    npc_marker,
};
use egui::{Painter, Rect, Stroke, Vec2};
use game_core::pursuit::Quarry;

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

    /// NPC traders (`npc_marker`): resting ones spread around their star
    /// on short tethers, flying ones on their gliding position with a faint
    /// dashed line, in their friend-or-foe colour, to the system they are
    /// bound for. Raider bands crossing between systems follow, in red.
    /// Returns the drawn trader and band markers, for taps.
    pub(super) fn draw_galaxy_threats(
        &self,
        ui: &egui::Ui,
        painter: &Painter,
        rect: Rect,
    ) -> Vec<npc_marker::NpcHit> {
        let art = Artwork::preload(ui.ctx());
        let mut hits = Vec::new();
        let gliding = self.session.gliding();
        let mut resting: Vec<(game_core::SystemId, usize)> = Vec::new();
        // Markers already on the map (own chips, then traders as placed), so a
        // trader never covers one; (centre, radius) in points.
        let mut placed: Vec<(egui::Pos2, f32)> = self
            .galaxy_fleet_hits
            .iter()
            .map(|hit| (hit.bounds.center(), super::fleets::CHIP))
            .collect();
        let traders = self.session.traffic().iter();
        for trader in traders.filter(|trader| self.session.npc_shown(Quarry::Trader(trader.id))) {
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
                    let at = clear_of(
                        npc_marker::berth_spot(anchor, STAR_CLEARANCE, count),
                        &placed,
                    );
                    super::fleets::tether(
                        painter,
                        anchor,
                        STAR_CLEARANCE,
                        at,
                        npc_marker::TRADER,
                        ring,
                    );
                    at
                }
                None => {
                    let Some(world) = self.session.trader_display_position(trader, &self.systems)
                    else {
                        continue;
                    };
                    let at = clear_of(self.camera.project(world, rect), &placed);
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
            placed.push((at, npc_marker::TRADER));
            hits.push(npc_marker::NpcHit {
                quarry: Quarry::Trader(trader.id),
                at,
            });
            if rect.expand(npc_marker::TRADER).contains(at) {
                npc_marker::paint(
                    painter,
                    &art,
                    at,
                    ship.kind,
                    ring,
                    Some(self.session.trader_color(trader)),
                    1,
                );
            }
        }
        let red = crate::theme::iff::HOSTILE;
        let find = |id| self.systems.iter().find(|known| known.id == id);
        let bands = self.session.raider_bands().iter();
        for band in bands.filter(|band| self.session.npc_shown(Quarry::Band(band.id))) {
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
                None => destination + Vec2::new(-24.0, -24.0),
            };
            // Clear of own chips and traders already drawn there.
            let at = clear_of(at, &placed);
            placed.push((at, npc_marker::TRADER));
            hits.push(npc_marker::NpcHit {
                quarry: Quarry::Band(band.id),
                at,
            });
            if let Some(kind) = npc_marker::lead(&band.ships) {
                npc_marker::paint(painter, &art, at, kind, red, None, band.ships.len());
            }
        }
        self.paint_npc_focus(painter, rect, &hits);
        hits
    }

    /// Where the galaxy map draws a trader or band (light-years): a resting
    /// one at its star, a flying trader on its gliding position, a crossing
    /// band along its line between the two systems.
    pub(super) fn npc_position(&self, quarry: Quarry) -> Option<glam::DVec3> {
        let find = |id| self.systems.iter().find(|known| known.id == id);
        match quarry {
            Quarry::Trader(id) => {
                let trader = self.session.traffic().iter().find(|npc| npc.id == id)?;
                let ship = trader.state.ships.first()?;
                match ship
                    .location
                    .system()
                    .filter(|_| ship.location.stationary())
                {
                    Some(system) => find(system).map(position),
                    None => self.session.trader_display_position(trader, &self.systems),
                }
            }
            // Patrols stay in their system; the galaxy map shows the system.
            Quarry::Patrol(id) => self
                .session
                .police_patrols()
                .into_iter()
                .find(|marker| marker.patrol.id == id)
                .and_then(|marker| find(marker.patrol.system).map(position)),
            Quarry::Band(id) => {
                let band = self
                    .session
                    .raider_bands()
                    .iter()
                    .find(|band| band.id == id)?;
                let at = position(find(band.system)?);
                Some(
                    match band
                        .transit
                        .and_then(|transit| Some((transit, find(transit.from)?)))
                    {
                        Some((transit, origin)) => {
                            position(origin).lerp(at, f64::from(transit.progress()))
                        }
                        None => at,
                    },
                )
            }
        }
    }

    /// The other star system a trader or band is bound for, if any.
    fn quarry_bound(&self, quarry: Quarry) -> Option<&game_core::StarSystem> {
        let bound = match quarry {
            Quarry::Trader(id) => self
                .session
                .traffic()
                .iter()
                .find(|npc| npc.id == id)
                .and_then(|npc| self.session.trader_heading(npc)),
            Quarry::Band(id) => self
                .session
                .raider_bands()
                .iter()
                .find(|band| band.id == id && band.transit.is_some())
                .map(|band| band.system),
            Quarry::Patrol(_) => None,
        }?;
        self.systems.iter().find(|known| known.id == bound)
    }

    /// The selected NPC's ring, and the pursuit line and crosshair.
    fn paint_npc_focus(&self, painter: &Painter, rect: Rect, hits: &[npc_marker::NpcHit]) {
        let (selected, pursued) = self.session.npc_focus();
        for hit in hits {
            let (chosen, chased) = (Some(hit.quarry) == selected, Some(hit.quarry) == pursued);
            npc_marker::paint_focus(painter, hit.at, chosen, chased);
            let chip = self.session.flagship_fleet().and_then(|fleet| {
                self.galaxy_fleet_hits
                    .iter()
                    .find(|chip| chip.fleet == fleet)
            });
            if let (true, Some(chip)) = (chased, chip) {
                npc_marker::paint_pursuit(painter, chip.bounds.center(), hit.at);
            }
            // Where the chase is going: on to the system the quarry is bound for.
            if let (true, Some(bound)) = (chased, self.quarry_bound(hit.quarry)) {
                let star = self.camera.project(position(bound), rect);
                super::local::dash(
                    painter,
                    hit.at,
                    star,
                    Stroke::new(1.0_f32, crate::theme::AMBER.gamma_multiply(0.6)),
                );
            }
        }
    }
}

/// A trader's marker moved off the markers already `placed` (own fleet
/// chips, other traders) that it would overlap: traders resting at or
/// leaving a star sit right by your fleets there. Each overlap pushes it
/// straight away from that marker's centre; a few passes settle chains.
/// Display only.
fn clear_of(at: egui::Pos2, placed: &[(egui::Pos2, f32)]) -> egui::Pos2 {
    let mut at = at;
    for _ in 0..4 {
        let mut moved = false;
        for (centre, radius) in placed {
            let gap = radius + npc_marker::TRADER + 3.0;
            let offset = at - *centre;
            if offset.length() >= gap - 0.01 {
                continue;
            }
            // Dead centre: step aside to the right.
            let direction = if offset.length() > 0.5 {
                offset.normalized()
            } else {
                Vec2::X
            };
            at = *centre + direction * gap;
            moved = true;
        }
        if !moved {
            break;
        }
    }
    at
}
