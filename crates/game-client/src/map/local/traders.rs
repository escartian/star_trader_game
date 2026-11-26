//! NPC traders on the system map: resting ones beside their body (tethered,
//! like own fleets, on the other side), flying ones on their gliding
//! position. Display only; traders are hailed from the Fleets page. Also
//! the double-click that opens the market where your fleet rests.
use super::{LocalView, scale::SymbolScale};
use crate::map::{fleets::tether, npc_marker};
use egui::{Painter, Rect};
use game_core::{BodyId, BodyKind, pursuit::Quarry, travel::Location};

impl LocalView {
    pub(super) fn draw_traders(&mut self, painter: &Painter, rect: Rect) {
        let Some(art) = &self.art else {
            return;
        };
        let mut hits = Vec::new();
        let scale = SymbolScale::new(self.camera.zoom);
        // How many traders already rest at each body, to spread them out.
        let mut resting: Vec<(BodyId, usize)> = Vec::new();
        for trader in &self.traders {
            let Some(anchor) = self.ship_anchor(&trader.location) else {
                continue;
            };
            let anchor = self.camera.project(anchor, rect);
            let body = self.berth_body(&trader.location);
            let at = match body {
                Some((id, kind)) => {
                    let count = match resting.iter_mut().find(|(known, _)| *known == id) {
                        Some((_, count)) => {
                            *count += 1;
                            *count - 1
                        }
                        None => {
                            resting.push((id, 1));
                            0
                        }
                    };
                    let clearance = scale.body_radius(kind);
                    let at = npc_marker::berth_spot(anchor, clearance, count);
                    tether(
                        painter,
                        anchor,
                        clearance,
                        at,
                        npc_marker::TRADER,
                        trader.ring,
                    );
                    at
                }
                None => anchor,
            };
            hits.push(npc_marker::NpcHit {
                quarry: Quarry::Trader(trader.id),
                at,
            });
            if rect.expand(npc_marker::TRADER).contains(at) {
                npc_marker::paint(
                    painter,
                    art,
                    at,
                    trader.kind,
                    trader.ring,
                    Some(trader.pip),
                    1,
                );
            }
        }
        self.npc_hits.extend(hits);
    }

    /// Where a trader or band in this system is drawn (view units), when it is here.
    pub(super) fn npc_anchor(&self, quarry: Quarry) -> Option<glam::DVec3> {
        match quarry {
            Quarry::Trader(id) => {
                let trader = self.traders.iter().find(|trader| trader.id == id)?;
                self.ship_anchor(&trader.location)
            }
            Quarry::Patrol(id) => self
                .police
                .iter()
                .find(|marker| marker.patrol.id == id && marker.patrol.system == self.system.id)
                .map(|marker| super::kilometers(marker.patrol.position) / super::KM_PER_VIEW_UNIT),
            Quarry::Band(id) => self
                .bands
                .iter()
                .find(|band| {
                    band.id == id && band.system == self.system.id && band.transit.is_none()
                })
                .map(|band| super::kilometers(band.position) / super::KM_PER_VIEW_UNIT),
        }
    }

    /// The selected NPC's ring and, while pursuing, the dashed line from the
    /// flagship's chip to its quarry with a crosshair. After fleets are drawn.
    pub(super) fn draw_npc_focus(&self, painter: &Painter) {
        let (selected, pursued, flagship) = self.npc_focus;
        let chip = flagship.and_then(|fleet| self.fleet_hits.iter().find(|hit| hit.fleet == fleet));
        for hit in &self.npc_hits {
            let chased = Some(hit.quarry) == pursued;
            npc_marker::paint_focus(painter, hit.at, Some(hit.quarry) == selected, chased);
            if let (true, Some(chip)) = (chased, chip) {
                npc_marker::paint_pursuit(painter, chip.bounds.center(), hit.at);
            }
        }
    }

    /// A double-click on a body (or fleet chip) where one of your fleets can
    /// trade opens that market, preferring the selected fleet.
    pub(super) fn open_market_at(&mut self, pointer: egui::Pos2, rect: Rect) {
        let chip = crate::map::fleets::pick(&self.fleet_hits, pointer).map(|hit| hit.ship);
        let body = super::pick(&self.system, &self.camera, rect, pointer);
        let option = self
            .trade_options
            .iter()
            .filter(|option| Some(option.ship) == chip || body.contains(&option.body))
            .max_by_key(|option| option.selected);
        if let Some(option) = option {
            self.market_request = Some(option.ship);
        }
    }

    /// The body a location rests at (the trade port when docked).
    fn berth_body(&self, location: &Location) -> Option<(BodyId, BodyKind)> {
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
        }?;
        Some((body.id, body.kind))
    }
}
