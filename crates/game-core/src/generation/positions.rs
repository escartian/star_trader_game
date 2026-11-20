//! Connected, spaced galaxy positions. Every new star is within the starter
//! ship's 20 ly range of a previously placed star.
use super::{GalaxyConfig, next};
use crate::GalaxyPosition;

pub(super) fn place(
    existing: &[GalaxyPosition],
    config: GalaxyConfig,
    state: &mut u64,
) -> Result<GalaxyPosition, String> {
    let radius = i64::from(config.radius_ly);
    let spacing = (radius * 3 / (2 * i64::from(config.systems).isqrt())).clamp(2, 8);
    let valid = |x: i64, z: i64| {
        x * x + z * z <= radius * radius
            && existing.iter().all(|other| {
                let ox = x - other.x;
                let oz = z - other.z;
                ox * ox + oz * oz >= spacing * spacing
            })
            && existing.iter().any(|other| {
                let dx = x - other.x;
                let dz = z - other.z;
                dx * dx + dz * dz <= 19 * 19
            })
    };
    for _ in 0..20_000 {
        let parent = existing[next(state) as usize % existing.len()];
        let dx = (next(state) % 37) as i64 - 18;
        let dz = (next(state) % 37) as i64 - 18;
        let hop = dx * dx + dz * dz;
        if hop > 19 * 19 || hop < spacing * spacing {
            continue;
        }
        let x = parent.x + dx;
        let z = parent.z + dz;
        if valid(x, z) {
            return Ok(GalaxyPosition { x, y: 0, z });
        }
    }
    // Random placement can stall in a crowded map. A bounded grid scan keeps
    // a valid requested size deterministic instead of failing on a rare seed.
    for x in -radius..=radius {
        for z in -radius..=radius {
            if valid(x, z) {
                return Ok(GalaxyPosition { x, y: 0, z });
            }
        }
    }
    Err("Could not place all stars at this size and density".into())
}
