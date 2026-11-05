//! Schematic moon spacing is presentation-only; saved kilometers are unchanged.
use super::view_position;
use game_core::{BodyKind, LocalBody, StarSystem};
use glam::DVec3;

const MIN_MOON_SEPARATION: f64 = 4.0;

pub(super) fn moon_parent<'a>(system: &'a StarSystem, moon: &LocalBody) -> Option<&'a LocalBody> {
    system
        .bodies
        .iter()
        .filter(|body| matches!(body.kind, BodyKind::RockyPlanet | BodyKind::GasGiant))
        .min_by(|a, b| {
            view_position(a)
                .distance_squared(view_position(moon))
                .total_cmp(&view_position(b).distance_squared(view_position(moon)))
                .then_with(|| a.id.0.cmp(&b.id.0))
        })
}

pub(super) fn display_position(system: &StarSystem, body: &LocalBody) -> DVec3 {
    let actual = view_position(body);
    if body.kind != BodyKind::Moon {
        return actual;
    }
    let Some(parent) = moon_parent(system, body) else {
        return actual;
    };
    let parent = view_position(parent);
    let offset = actual - parent;
    if offset.length() >= MIN_MOON_SEPARATION {
        return actual;
    }
    let direction = offset
        .try_normalize()
        .unwrap_or(DVec3::new(1.0, 0.5, -0.8).normalize());
    parent + direction * MIN_MOON_SEPARATION
}
