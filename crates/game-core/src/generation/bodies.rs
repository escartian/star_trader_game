//! Non-home system bodies. Wide, separate orbital rings make every layout
//! valid for all ticks; the moon rides the first planet's ring.
use super::{names::Namer, next};
use crate::{BodyId, BodyKind, LocalBody, LocalPosition, orbits};

const MILLION_KM: i64 = 1_000_000;

fn position(radius_mkm: i64, draw: u64) -> LocalPosition {
    let radius = radius_mkm * MILLION_KM;
    let (x_km, z_km) = match draw % 8 {
        0 => (radius, 0),
        1 => (radius * 3 / 5, radius * 4 / 5),
        2 => (0, radius),
        3 => (-radius * 3 / 5, radius * 4 / 5),
        4 => (-radius, 0),
        5 => (-radius * 3 / 5, -radius * 4 / 5),
        6 => (0, -radius),
        _ => (radius * 3 / 5, -radius * 4 / 5),
    };
    LocalPosition {
        x_km,
        y_km: 0,
        z_km,
    }
}

fn body(
    id: u64,
    namer: &mut Namer,
    state: &mut u64,
    kind: BodyKind,
    position: LocalPosition,
) -> LocalBody {
    let name = if kind == BodyKind::Station {
        namer.station(state)
    } else {
        namer.place(state)
    };
    LocalBody {
        id: BodyId(id),
        name,
        kind,
        position,
        planet: None,
        orbit: None,
        surface: None,
    }
}

pub(super) fn generate(namer: &mut Namer, max_bodies: u8, state: &mut u64) -> Vec<LocalBody> {
    let mut bodies = Vec::new();
    let target_count = 5 + next(state) % (u64::from(max_bodies) - 4);
    // IDs 1–3 are always present, including a port and a planet market.
    let first = position(35, next(state));
    bodies.push(body(1, namer, state, BodyKind::RockyPlanet, first));
    let second = if next(state).is_multiple_of(2) {
        BodyKind::GasGiant
    } else {
        BodyKind::RockyPlanet
    };
    let at = position(50, next(state));
    bodies.push(body(2, namer, state, second, at));
    let at = position(65, next(state));
    bodies.push(body(3, namer, state, BodyKind::Station, at));

    // A moon is optional; its 3 million km orbit is clear of its parent.
    if next(state).is_multiple_of(2) {
        let offset = position(3, next(state));
        let at = LocalPosition {
            x_km: first.x_km + offset.x_km,
            y_km: 0,
            z_km: first.z_km + offset.z_km,
        };
        bodies.push(body(4, namer, state, BodyKind::Moon, at));
    }
    // Outer rings hold a varying mix of planets and asteroids. At the
    // 15-body limit the last ring is 190 million km out, inside the cube.
    let extra = target_count - bodies.len() as u64;
    for index in 0..extra {
        let kind = match next(state) % 3 {
            0 => BodyKind::RockyPlanet,
            1 => BodyKind::GasGiant,
            _ => BodyKind::Asteroid,
        };
        let at = position(80 + 10 * index as i64, next(state));
        bodies.push(body(5 + index, namer, state, kind, at));
    }
    let parents = if bodies.iter().any(|body| body.id == BodyId(4)) {
        vec![(BodyId(4), BodyId(1))]
    } else {
        Vec::new()
    };
    orbits::assign_orbits(&mut bodies, &parents);
    bodies
}
