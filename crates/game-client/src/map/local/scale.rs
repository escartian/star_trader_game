//! Readable symbolic sizes: stars > planets > stations, at every zoom level.
use game_core::{BodyKind, StarKind};

pub(super) struct SymbolScale {
    factor: f32,
}

impl SymbolScale {
    pub(super) fn new(zoom: f32) -> Self {
        Self {
            factor: zoom.sqrt().clamp(0.75, 1.5),
        }
    }

    pub(super) fn star_radius(&self, kind: StarKind) -> f32 {
        let radius = match kind {
            StarKind::RedGiant => 44.0,
            StarKind::BlueGiant => 38.0,
            StarKind::YellowDwarf => 32.0,
            StarKind::RedDwarf => 28.0,
            StarKind::WhiteDwarf => 26.0,
        };
        radius * self.factor
    }

    pub(super) fn body_radius(&self, kind: BodyKind) -> f32 {
        match kind {
            BodyKind::RockyPlanet => 11.0 * self.factor,
            BodyKind::GasGiant => 18.0 * self.factor,
            BodyKind::Station => self.station_size() / 2.0,
        }
    }

    pub(super) fn station_size(&self) -> f32 {
        22.0 * self.factor
    }
}
