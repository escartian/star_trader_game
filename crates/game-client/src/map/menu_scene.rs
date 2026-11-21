//! The main menu's 3D scene: the battle view's low-poly ship models flying
//! looping routes, a planet that changes every `PLANET_SECONDS` and a tilted
//! orbit with a moon and a prospector going round it. Decorative only.
//!
//! Scene space is the battle camera's convention: world units, z up; the camera
//! looks along +y, so screen x follows scene x and depth grows with y. Planet
//! and moon are the baked sprites placed at projected 3D points.
use super::art::{ArtKind, Artwork};
use crate::battle::camera::{Projector, ScenePoint};
use crate::battle::model;
use egui::{Color32, Painter, Pos2, Rect, Stroke};
use game_core::fleet::ShipKind;
use glam::{Mat3, Vec3};
use std::f32::consts::TAU;

/// Slightly above the scene, looking a little down so ships show their tops.
const EYE: Vec3 = Vec3::new(0.0, -60.0, 22.0);
const FOCUS: Vec3 = Vec3::new(0.0, 0.0, 4.0);

const PLANET_CENTRE: Vec3 = Vec3::new(32.0, 20.0, -24.0);
const PLANET_RADIUS: f32 = 14.0;
/// Seconds each planet is shown, then the next fades in over `FADE_SECONDS`.
const PLANET_SECONDS: f32 = 24.0;
const FADE_SECONDS: f32 = 3.0;
const PLANETS: [ArtKind; 7] = [
    ArtKind::GasGiant,
    ArtKind::OceanPlanet,
    ArtKind::StormGiant,
    ArtKind::JunglePlanet,
    ArtKind::FrozenPlanet,
    ArtKind::MethaneGiant,
    ArtKind::VolcanicPlanet,
];

const ORBIT_RADIUS: f32 = 23.0;
/// Ring tilt out of the horizontal plane, radians.
const ORBIT_TILT: f32 = 0.32;
/// Seconds for one trip round the orbit.
const ORBIT_SECONDS: f32 = 45.0;
const MOON_RADIUS: f32 = 2.2;

/// A ship flying a straight route on a loop (both ends are off screen).
struct Flight {
    kind: ShipKind,
    from: Vec3,
    to: Vec3,
    /// Seconds per pass, including time off screen.
    period: f32,
    /// Seconds added to the clock, so ships do not start together.
    offset: f32,
    /// Ship length in world units.
    size: f32,
}

static FLIGHTS: [Flight; 5] = [
    // A two-freighter convoy coming in from the distance down the left side,
    // passing the camera.
    Flight {
        kind: ShipKind::Freighter,
        from: Vec3::new(-34.0, 190.0, 34.0),
        to: Vec3::new(-40.0, -18.0, 2.0),
        period: 48.0,
        offset: 30.0,
        size: 10.0,
    },
    Flight {
        kind: ShipKind::Freighter,
        from: Vec3::new(-24.0, 205.0, 30.0),
        to: Vec3::new(-30.0, -3.0, -2.0),
        period: 48.0,
        offset: 27.0,
        size: 9.0,
    },
    // A patrol pair climbing away over the planet, top right.
    Flight {
        kind: ShipKind::Patrol,
        from: Vec3::new(62.0, -10.0, 12.0),
        to: Vec3::new(10.0, 230.0, 30.0),
        period: 34.0,
        offset: 0.0,
        size: 5.0,
    },
    Flight {
        kind: ShipKind::Patrol,
        from: Vec3::new(70.0, -4.0, 8.0),
        to: Vec3::new(18.0, 236.0, 26.0),
        period: 34.0,
        offset: -1.0,
        size: 5.0,
    },
    // A scout racing from behind the camera toward the planet's horizon.
    Flight {
        kind: ShipKind::Scout,
        from: Vec3::new(34.0, -36.0, -8.0),
        to: Vec3::new(14.0, 220.0, 0.0),
        period: 22.0,
        offset: 9.0,
        size: 3.5,
    },
];

/// Width / height the scene is composed for; narrower screens step back.
const COMPOSED_ASPECT: f32 = 1.39;

/// The camera position for `rect`: on a narrow (portrait) screen it moves back
/// along its view line so the planet and the routes stay in frame.
fn eye(rect: Rect) -> Vec3 {
    let aspect = rect.width() / rect.height().max(1.0);
    FOCUS + (EYE - FOCUS) * (COMPOSED_ASPECT / aspect).max(1.0)
}

/// Model rotation for a ship flying along `forward`, rolled by `bank` radians.
fn orientation(forward: Vec3, bank: f32) -> Mat3 {
    let forward = forward.normalize();
    let right = forward.cross(Vec3::Z).normalize();
    let up = right.cross(forward);
    let roll = Mat3::from_axis_angle(forward, bank);
    Mat3::from_cols(roll * right, forward, roll * up)
}

/// Point on the orbit ring at `angle` radians (0 is nearest the camera's right).
fn orbit_point(angle: f32, radius: f32) -> Vec3 {
    let across = Vec3::X;
    let along = Vec3::new(0.0, ORBIT_TILT.cos(), ORBIT_TILT.sin());
    PLANET_CENTRE + radius * (angle.cos() * across + angle.sin() * along)
}

/// One thing to draw, ordered far to near.
enum Item {
    /// A flight, its progress through the pass (0-1), position and rotation.
    Ship(&'static Flight, f32, Vec3, Mat3),
    Planet,
}

impl Flight {
    /// Route position at `progress` (0-1 through a pass), with a gentle sway.
    fn position(&self, progress: f32) -> Vec3 {
        self.from.lerp(self.to, progress) + Vec3::Z * (progress * TAU).sin() * 1.5
    }
}

pub(super) fn paint(painter: &Painter, rect: Rect, time: f64, art: &Artwork) {
    let projector = Projector::looking_at(eye(rect), FOCUS, rect);
    let time = time as f32;
    let mut items = vec![(depth(&projector, PLANET_CENTRE), Item::Planet)];
    for flight in &FLIGHTS {
        let progress = ((time + flight.offset) / flight.period).rem_euclid(1.0);
        let at = flight.position(progress);
        let bank = (time * 0.4 + flight.offset).sin() * 0.25;
        let rotation = orientation(flight.to - flight.from, bank);
        items.push((
            depth(&projector, at),
            Item::Ship(flight, progress, at, rotation),
        ));
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, item) in items {
        match item {
            Item::Ship(flight, progress, at, rotation) => {
                let length = flight.from.distance(flight.to);
                let route = |progress| flight.position(progress);
                let (kind, size) = (flight.kind, flight.size);
                super::menu_trails::paint(painter, &projector, kind, size, progress, length, route);
                if let Some(ship) = model::for_kind(kind) {
                    ship.paint_oriented(painter, &projector, ScenePoint(at), size, rotation);
                }
            }
            Item::Planet => paint_planet_system(painter, &projector, time, art),
        }
    }
}

fn depth(projector: &Projector, point: Vec3) -> f32 {
    projector
        .project(ScenePoint(point))
        .map_or(f32::MAX, |projected| projected.depth)
}

/// The orbit's far half, whatever orbits behind, the planet, then the near half
/// and whatever orbits in front.
fn paint_planet_system(painter: &Painter, projector: &Projector, time: f32, art: &Artwork) {
    let Some(centre) = projector.project(ScenePoint(PLANET_CENTRE)) else {
        return;
    };
    let angle = time / ORBIT_SECONDS * TAU;
    let moon = orbit_point(angle, ORBIT_RADIUS);
    let prospector = orbit_point(angle + 2.2, ORBIT_RADIUS * 0.82);
    let behind = |point: Vec3| point.y > PLANET_CENTRE.y;
    paint_ring(painter, projector, true);
    for (point, is_moon) in [(moon, true), (prospector, false)] {
        if behind(point) {
            paint_orbiter(painter, projector, art, point, is_moon, angle);
        }
    }
    paint_planet(
        painter,
        centre.pos,
        centre.scale * PLANET_RADIUS * 2.0,
        time,
        art,
    );
    paint_ring(painter, projector, false);
    for (point, is_moon) in [(moon, true), (prospector, false)] {
        if !behind(point) {
            paint_orbiter(painter, projector, art, point, is_moon, angle);
        }
    }
}

fn paint_ring(painter: &Painter, projector: &Projector, far_half: bool) {
    const SEGMENTS: usize = 96;
    let stroke = Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(91, 225, 226, 55));
    let points: Vec<_> = (0..=SEGMENTS)
        .map(|step| orbit_point(step as f32 / SEGMENTS as f32 * TAU, ORBIT_RADIUS))
        .collect();
    for pair in points.windows(2) {
        if (pair[0].y > PLANET_CENTRE.y) != far_half {
            continue;
        }
        let ends = pair
            .iter()
            .map(|point| projector.project(ScenePoint(*point)));
        if let [Some(a), Some(b)] = ends.collect::<Vec<_>>()[..] {
            painter.line_segment([a.pos, b.pos], stroke);
        }
    }
}

fn paint_orbiter(
    painter: &Painter,
    projector: &Projector,
    art: &Artwork,
    point: Vec3,
    is_moon: bool,
    angle: f32,
) {
    if is_moon {
        if let Some(at) = projector.project(ScenePoint(point)) {
            let tint = Color32::from_gray(190);
            art.paint_tinted(
                ArtKind::Moon,
                painter,
                at.pos,
                at.scale * MOON_RADIUS * 2.0,
                tint,
            );
        }
        return;
    }
    // The prospector flies along the ring: its heading is the ring's tangent.
    let ahead = orbit_point(angle + 2.21, ORBIT_RADIUS * 0.82);
    // Its trail curves back along the ring: one whole lap is progress 0-1.
    let radius = ORBIT_RADIUS * 0.82;
    let route = |progress: f32| orbit_point(angle + 2.2 + (progress - 1.0) * TAU, radius);
    let (kind, size) = (ShipKind::Prospector, 2.4);
    super::menu_trails::paint(painter, projector, kind, size, 1.0, TAU * radius, route);
    if let Some(ship) = model::for_kind(kind) {
        let rotation = orientation(ahead - point, -0.3);
        ship.paint_oriented(painter, projector, ScenePoint(point), 2.4, rotation);
    }
}

/// The current planet, cross-fading into the next at each change.
fn paint_planet(painter: &Painter, centre: Pos2, size: f32, time: f32, art: &Artwork) {
    let index = (time / PLANET_SECONDS).floor() as usize;
    let into_change = time.rem_euclid(PLANET_SECONDS);
    let current = PLANETS[index % PLANETS.len()];
    // Dimmed so the menu stays the focus.
    let dim = |alpha: f32| {
        let value = (170.0 * alpha) as u8;
        Color32::from_rgba_premultiplied(value, value, value, (255.0 * alpha) as u8)
    };
    if into_change < FADE_SECONDS {
        let previous = PLANETS[(index + PLANETS.len() - 1) % PLANETS.len()];
        art.paint_tinted(previous, painter, centre, size, dim(1.0));
        art.paint_tinted(
            current,
            painter,
            centre,
            size,
            dim(into_change / FADE_SECONDS),
        );
    } else {
        art.paint_tinted(current, painter, centre, size, dim(1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_flying_along_x_keeps_its_nose_forward_and_its_top_up() {
        let rotation = orientation(Vec3::X, 0.0);
        assert!((rotation * Vec3::Y).abs_diff_eq(Vec3::X, 1e-5));
        assert!((rotation * Vec3::Z).abs_diff_eq(Vec3::Z, 1e-5));
    }

    #[test]
    fn the_planet_and_its_orbit_sit_in_front_of_the_camera() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 720.0));
        let phone = Rect::from_min_size(Pos2::ZERO, egui::vec2(390.0, 844.0));
        for rect in [rect, phone] {
            let projector = Projector::looking_at(eye(rect), FOCUS, rect);
            let centre = projector.project(ScenePoint(PLANET_CENTRE)).unwrap();
            assert!(rect.contains(centre.pos), "planet off screen in {rect:?}");
        }
        let projector = Projector::looking_at(eye(rect), FOCUS, rect);
        for step in 0..8 {
            let point = orbit_point(step as f32 / 8.0 * TAU, ORBIT_RADIUS);
            assert!(projector.project(ScenePoint(point)).is_some());
        }
    }
}
