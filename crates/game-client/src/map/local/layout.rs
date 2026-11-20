//! Schematic moon spacing is presentation-only; saved kilometers are unchanged.
use super::view_position;
use game_core::{BodyKind, LocalBody, StarSystem};
use glam::DVec3;

/// How far apart a moon is drawn from its planet at most, in view units
/// (10 million km each), so their markers never overlap.
const MIN_MOON_SEPARATION: f64 = 4.0;
/// A push toward the star never exceeds this share of the planet's own
/// distance from the star, so a moon is never thrown across the star (an inner
/// planet 35 million km out would otherwise get a moon 40 million km away).
const MAX_SEPARATION_SHARE: f64 = 0.4;

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
    let direction = offset
        .try_normalize()
        .unwrap_or(DVec3::new(1.0, 0.5, -0.8).normalize());
    // Outward pushes cannot cross the star; inward ones are kept short.
    let separation = if direction.dot(parent) < 0.0 {
        MIN_MOON_SEPARATION.min(parent.length() * MAX_SEPARATION_SHARE)
    } else {
        MIN_MOON_SEPARATION
    };
    if offset.length() >= separation {
        return actual;
    }
    parent + direction * separation
}

/// Body names are generated as "<system> <name>"; inside that system's own view
/// the prefix is redundant, so show just "<name>". Falls back to the full name.
pub(super) fn short_name<'a>(system: &StarSystem, body: &'a LocalBody) -> &'a str {
    body.name
        .strip_prefix(system.name.as_str())
        .map(str::trim_start)
        .filter(|rest| !rest.is_empty())
        .unwrap_or(&body.name)
}
