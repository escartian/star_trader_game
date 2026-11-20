//! Non-home system bodies. Wide, separate orbital rings make every layout
//! valid for all ticks; the moon rides the first planet's ring.
use super::next;
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

fn body(id: u64, system: &str, kind: BodyKind, position: LocalPosition) -> LocalBody {
    let noun = match kind {
        BodyKind::RockyPlanet => "world",
        BodyKind::GasGiant => "giant",
        BodyKind::Moon => "moon",
        BodyKind::Asteroid => "asteroid",
        BodyKind::Station => "port",
    };
    LocalBody {
        id: BodyId(id),
        name: format!("{system} {noun} {id}"),
        kind,
        position,
        planet: None,
        orbit: None,
    }
}

pub(super) fn generate(system: &str, max_bodies: u8, state: &mut u64) -> Vec<LocalBody> {
    let mut bodies = Vec::new();
    let target_count = 5 + next(state) % (u64::from(max_bodies) - 4);
    // IDs 1–3 are always present, including a port and a planet market.
    let first = position(35, next(state));
    bodies.push(body(1, system, BodyKind::RockyPlanet, first));
    let second = if next(state).is_multiple_of(2) {
        BodyKind::GasGiant
    } else {
        BodyKind::RockyPlanet
    };
    bodies.push(body(2, system, second, position(50, next(state))));
    bodies.push(body(
        3,
        system,
        BodyKind::Station,
        position(65, next(state)),
    ));

    // A moon is optional; its 3 million km orbit is clear of its parent.
    if next(state).is_multiple_of(2) {
        let offset = position(3, next(state));
        bodies.push(body(
            4,
            system,
            BodyKind::Moon,
            LocalPosition {
                x_km: first.x_km + offset.x_km,
                y_km: 0,
                z_km: first.z_km + offset.z_km,
            },
        ));
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
        bodies.push(body(
            5 + index,
            system,
            kind,
            position(80 + 10 * index as i64, next(state)),
        ));
    }
    let parents = if bodies.iter().any(|body| body.id == BodyId(4)) {
        vec![(BodyId(4), BodyId(1))]
    } else {
        Vec::new()
    };
    orbits::assign_orbits(&mut bodies, &parents);
    bodies
}
