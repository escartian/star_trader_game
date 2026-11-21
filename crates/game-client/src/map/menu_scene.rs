//! The main menu's 3D scene: the battle view's low-poly ship models flying the
//! generated routes of `menu_flights`, one planet (picked per launch, fixed
//! until the game restarts) and a tilted orbit with a moon and a prospector
//! going round it. Decorative only.
//!
//! Scene space is the battle camera's convention: world units, z up; the camera
//! looks along +y, so screen x follows scene x and depth grows with y. Planet
//! and moon are the baked sprites placed at projected 3D points.
use super::art::{ArtKind, Artwork};
use super::menu_flights::{self, Flight, orientation};
use super::menu_trails;
use crate::battle::camera::{Projector, ScenePoint};
use crate::battle::model;
use egui::{Color32, Painter, Rect, Stroke};
use game_core::fleet::ShipKind;
use glam::Vec3;
use std::f32::consts::TAU;

/// Slightly above the scene, looking a little down so ships show their tops.
const EYE: Vec3 = Vec3::new(0.0, -60.0, 22.0);
const FOCUS: Vec3 = Vec3::new(0.0, 0.0, 4.0);

pub(super) const PLANET_CENTRE: Vec3 = Vec3::new(32.0, 20.0, -24.0);
pub(super) const PLANET_RADIUS: f32 = 14.0;
/// The trade station's docking bay, high over the planet toward the camera.
pub(super) const STATION: Vec3 = Vec3::new(21.0, 2.0, 7.0);
/// Station sprite width in world units.
const STATION_SIZE: f32 = 18.0;
/// The planet is one of these, chosen by the launch seed.
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

/// Width / height the scene is composed for; narrower screens step back.
const COMPOSED_ASPECT: f32 = 1.39;

/// The camera position for `rect`: on a narrow (portrait) screen it moves back
/// along its view line so the planet and the routes stay in frame.
fn eye(rect: Rect) -> Vec3 {
    let aspect = rect.width() / rect.height().max(1.0);
    FOCUS + (EYE - FOCUS) * (COMPOSED_ASPECT / aspect).max(1.0)
}

/// Point on the orbit ring at `angle` radians (0 is nearest the camera's right).
fn orbit_point(angle: f32, radius: f32) -> Vec3 {
    let across = Vec3::X;
    let along = Vec3::new(0.0, ORBIT_TILT.cos(), ORBIT_TILT.sin());
    PLANET_CENTRE + radius * (angle.cos() * across + angle.sin() * along)
}

/// One thing to draw, ordered far to near.
enum Item {
    /// A flight and its progress (0-1) along its route.
    Ship(Flight, f32),
    Planet,
    Station,
}

/// Draws the scene at `time` seconds; `launch_seed` picks the planet and the
/// flights, so they stay the same until the game restarts.
pub(super) fn paint(painter: &Painter, rect: Rect, time: f64, art: &Artwork, launch_seed: u64) {
    let projector = Projector::looking_at(eye(rect), FOCUS, rect);
    let time = time as f32;
    let mut items = vec![
        (depth(&projector, PLANET_CENTRE), Item::Planet),
        // Just behind its bay, so ships docking are drawn over the station.
        (depth(&projector, STATION) + 0.5, Item::Station),
    ];
    for (flight, progress) in menu_flights::active(time, launch_seed) {
        let at = flight.position(progress);
        items.push((depth(&projector, at), Item::Ship(flight, progress)));
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    let planet = PLANETS[(launch_seed % PLANETS.len() as u64) as usize];
    for (_, item) in items {
        match item {
            Item::Ship(flight, progress) => {
                let size = flight.size_at(progress);
                if size <= 0.05 {
                    continue;
                }
                let trail = menu_trails::Trail {
                    kind: flight.kind,
                    size,
                    progress,
                    time,
                    phase: flight.phase,
                };
                trail.paint(painter, &projector, |progress| flight.position(progress));
                if let Some(ship) = model::for_kind(flight.kind) {
                    let at = ScenePoint(flight.position(progress));
                    ship.paint_oriented(
                        painter,
                        &projector,
                        at,
                        flight.size,
                        flight.rotation(progress),
                    );
                }
            }
            Item::Planet => paint_planet_system(painter, &projector, time, art, planet),
            Item::Station => paint_station(painter, &projector, time, art),
        }
    }
}

/// The trade station sprite with a slow beacon over its bay.
fn paint_station(painter: &Painter, projector: &Projector, time: f32, art: &Artwork) {
    let Some(at) = projector.project(ScenePoint(STATION)) else {
        return;
    };
    let size = at.scale * STATION_SIZE;
    art.paint_tinted(
        ArtKind::Station,
        painter,
        at.pos,
        size,
        Color32::from_gray(205),
    );
    // A green docking beacon, blinking once every two seconds.
    let blink = ((time * TAU / 2.0).sin() * 4.0).clamp(0.0, 1.0);
    let beacon = at.pos + egui::vec2(0.0, -size * 0.36);
    painter.circle_filled(
        beacon,
        size * 0.03 + 1.0,
        Color32::from_rgba_unmultiplied(148, 255, 85, (230.0 * blink) as u8),
    );
    painter.circle_filled(
        beacon,
        size * 0.08 + 2.0,
        Color32::from_rgba_unmultiplied(148, 255, 85, (50.0 * blink) as u8),
    );
}

fn depth(projector: &Projector, point: Vec3) -> f32 {
    projector
        .project(ScenePoint(point))
        .map_or(f32::MAX, |projected| projected.depth)
}

/// The orbit's far half, whatever orbits behind, the planet, then the near half
/// and whatever orbits in front.
fn paint_planet_system(
    painter: &Painter,
    projector: &Projector,
    time: f32,
    art: &Artwork,
    planet: ArtKind,
) {
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
            paint_orbiter(painter, projector, art, point, is_moon, time);
        }
    }
    // Dimmed so the menu stays the focus.
    let size = centre.scale * PLANET_RADIUS * 2.0;
    art.paint_tinted(planet, painter, centre.pos, size, Color32::from_gray(170));
    paint_ring(painter, projector, false);
    for (point, is_moon) in [(moon, true), (prospector, false)] {
        if !behind(point) {
            paint_orbiter(painter, projector, art, point, is_moon, time);
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
    time: f32,
) {
    let angle = time / ORBIT_SECONDS * TAU;
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
    let trail = menu_trails::Trail {
        kind: ShipKind::Prospector,
        size: 2.4,
        progress: 1.0,
        time,
        phase: 42.0,
    };
    trail.paint(painter, projector, route);
    if let Some(ship) = model::for_kind(ShipKind::Prospector) {
        let rotation = orientation(ahead - point, -0.3);
        ship.paint_oriented(painter, projector, ScenePoint(point), 2.4, rotation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

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
