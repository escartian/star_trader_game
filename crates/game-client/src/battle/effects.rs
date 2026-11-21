//! Display-only animation that makes each battle update readable: ships glide
//! to their new hexes, then each new shot flies as a beam and flashes on its
//! target. The rules never wait for these; captures switch them off.
use super::{camera::ScenePoint, canvas::Canvas, scene::side_color};
use crate::theme::{AMBER, CYAN};
use egui::{Color32, Stroke};
use game_core::combat::tactical::{Battle, Hex};
use glam::{Vec2, Vec3};

/// Seconds a ship takes to glide to its new position.
pub const GLIDE_SECONDS: f64 = 0.35;
/// Seconds a beam takes to reach its target.
const BEAM_SECONDS: f64 = 0.2;
/// Seconds a hit flash or explosion lasts after the beam arrives.
const FLASH_SECONDS: f64 = 0.5;
/// Seconds between consecutive shots.
const SHOT_GAP: f64 = 0.16;

struct Glide {
    unit: usize,
    from: Hex,
    start: f64,
}

struct Beam {
    from: ScenePoint,
    to: ScenePoint,
    color: Color32,
    hit: bool,
    /// Hull damage was dealt (otherwise shields took the hit).
    hull: bool,
    destroyed: bool,
    start: f64,
}

#[derive(Default)]
pub struct Effects {
    /// The battle the state below describes; a different seed starts afresh.
    seed: Option<u64>,
    cells: Vec<Hex>,
    shots: usize,
    glides: Vec<Glide>,
    beams: Vec<Beam>,
}

impl Effects {
    /// Compares `battle` with the last state seen and queues animation for what
    /// changed. With `animate` off, the new state is adopted without effects.
    pub fn observe(&mut self, battle: &Battle, now: f64, animate: bool) {
        let shots: usize = battle.rounds.iter().map(Vec::len).sum();
        let cells: Vec<Hex> = battle.units.iter().map(|unit| unit.at).collect();
        let fresh = self.seed != Some(battle.seed) || cells.len() != self.cells.len();
        if fresh || shots < self.shots || !animate {
            *self = Self {
                seed: Some(battle.seed),
                cells,
                shots,
                ..Self::default()
            };
            return;
        }
        let mut moved = false;
        for (unit, (old, new)) in self.cells.iter().zip(&cells).enumerate() {
            if old != new {
                self.glides.push(Glide {
                    unit,
                    from: *old,
                    start: now,
                });
                moved = true;
            }
        }
        let first = now + if moved { GLIDE_SECONDS } else { 0.0 };
        let new_shots = battle.rounds.iter().flatten().skip(self.shots);
        for (index, shot) in new_shots.enumerate() {
            let attacker = &battle.units[usize::from(shot.attacker)];
            let target = &battle.units[usize::from(shot.target)];
            self.beams.push(Beam {
                from: ScenePoint::of(attacker.at),
                to: ScenePoint::of(target.at),
                color: side_color(attacker.side),
                hit: shot.hit,
                hull: shot.hull_damage > 0,
                destroyed: shot.destroyed,
                start: first + index as f64 * SHOT_GAP,
            });
        }
        self.cells = cells;
        self.shots = shots;
        self.glides
            .retain(|glide| now < glide.start + GLIDE_SECONDS);
        self.beams
            .retain(|beam| now < beam.start + BEAM_SECONDS + FLASH_SECONDS);
    }

    /// Whether anything is still playing (the view keeps repainting).
    pub fn busy(&self, now: f64) -> bool {
        self.glides
            .iter()
            .any(|glide| now < glide.start + GLIDE_SECONDS)
            || self
                .beams
                .iter()
                .any(|beam| now < beam.start + BEAM_SECONDS + FLASH_SECONDS)
    }

    /// Where to draw `unit`, standing at `at`, at time `now`.
    pub fn position(&self, unit: usize, at: Hex, now: f64) -> ScenePoint {
        let to = ScenePoint::of(at);
        let Some(glide) = self.glides.iter().rev().find(|glide| glide.unit == unit) else {
            return to;
        };
        let t = ((now - glide.start) / GLIDE_SECONDS).clamp(0.0, 1.0) as f32;
        let eased = t * t * (3.0 - 2.0 * t);
        ScenePoint(ScenePoint::of(glide.from).0.lerp(to.0, eased))
    }

    /// Heading while a ship glides; otherwise it keeps its
    /// existing side-facing pose. This is presentation state, not a battle rule.
    pub fn heading(&self, unit: usize, at: Hex, now: f64) -> Option<Vec2> {
        let glide = self.glides.iter().rev().find(|glide| {
            glide.unit == unit && (glide.start..glide.start + GLIDE_SECONDS).contains(&now)
        })?;
        let from = ScenePoint::of(glide.from).0;
        let to = ScenePoint::of(at).0;
        let heading = (to - from).truncate().normalize_or_zero();
        (heading != Vec2::ZERO).then_some(heading)
    }

    pub fn paint(&self, canvas: &Canvas, now: f64) {
        for beam in &self.beams {
            let age = now - beam.start;
            if age < 0.0 {
                continue;
            }
            // A miss streaks past, above and beyond its target.
            let end = if beam.hit {
                beam.to.0
            } else {
                beam.to.0 + (beam.to.0 - beam.from.0).normalize_or_zero() * 1.2 + Vec3::Z * 0.6
            };
            let travel = (age / BEAM_SECONDS).min(1.0) as f32;
            let head = beam.from.0.lerp(end, travel);
            let tail = beam.from.0.lerp(end, (travel - 0.45).max(0.0));
            let fade = if age < BEAM_SECONDS {
                1.0
            } else {
                (1.0 - (age - BEAM_SECONDS) / 0.25).max(0.0) as f32
            };
            if fade > 0.0 {
                let alpha = if beam.hit { 1.0 } else { 0.55 } * fade;
                canvas.line(
                    ScenePoint(tail),
                    ScenePoint(head),
                    Stroke::new(3.0_f32, beam.color.gamma_multiply(alpha)),
                );
                canvas.line(
                    ScenePoint(tail),
                    ScenePoint(head),
                    Stroke::new(1.0_f32, Color32::WHITE.gamma_multiply(alpha)),
                );
            }
            if beam.hit && age >= BEAM_SECONDS {
                flash(canvas, beam, ((age - BEAM_SECONDS) / FLASH_SECONDS) as f32);
            }
        }
    }
}

/// Hit flash (cyan on shields, amber on hull) or an explosion; `t` runs 0 to 1.
fn flash(canvas: &Canvas, beam: &Beam, t: f32) {
    let Some(at) = canvas.projector.project(beam.to) else {
        return;
    };
    let fade = (1.0 - t).max(0.0);
    let (size, color) = match (beam.destroyed, beam.hull) {
        (true, _) => (1.1, AMBER),
        (false, true) => (0.45, AMBER),
        (false, false) => (0.55, CYAN),
    };
    let radius = at.scale * size * (0.3 + 0.7 * t);
    canvas
        .painter
        .circle_filled(at.pos, radius * 0.6, color.gamma_multiply(0.45 * fade));
    canvas.painter.circle_stroke(
        at.pos,
        radius,
        Stroke::new(2.0_f32, color.gamma_multiply(fade)),
    );
    if beam.destroyed {
        canvas
            .painter
            .circle_filled(at.pos, radius * 0.35, Color32::WHITE.gamma_multiply(fade));
        canvas.painter.circle_stroke(
            at.pos,
            radius * 1.5,
            Stroke::new(
                1.0_f32,
                Color32::from_rgb(255, 120, 60).gamma_multiply(fade),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat::{CombatCatalog, Side, engagement::CombatantSetup};
    use game_core::fleet::{ShipCatalog, ShipKind};

    fn battle() -> Battle {
        let setup = |side| CombatantSetup {
            name: format!("{side:?}"),
            side,
            ship: None,
            kind: ShipKind::Scout,
            profile: ShipCatalog::default()
                .spec(ShipKind::Scout)
                .unwrap()
                .combat
                .clone(),
            hull_damage: 0,
            gear: Vec::new(),
        };
        Battle::new(
            &[setup(Side::Player), setup(Side::Hostile)],
            &CombatCatalog::default(),
            4,
        )
        .unwrap()
    }

    #[test]
    fn moves_glide_then_shots_play_and_everything_settles() {
        let mut battle = battle();
        let mut effects = Effects::default();
        effects.observe(&battle, 0.0, true);
        assert!(
            !effects.busy(0.0),
            "a new battle starts without replaying history"
        );
        let me = battle.active().unwrap();
        let from = battle.units[me].at;
        let to = battle.reachable()[0];
        battle.move_to(to).unwrap();
        effects.observe(&battle, 1.0, true);
        assert!(effects.busy(1.0));
        assert_eq!(effects.position(me, to, 1.0), ScenePoint::of(from));
        let horizontal = (ScenePoint::of(to).0 - ScenePoint::of(from).0).truncate();
        assert_eq!(
            effects.heading(me, to, 1.0),
            (horizontal != glam::Vec2::ZERO).then_some(horizontal.normalize_or_zero())
        );
        assert_eq!(
            effects.position(me, to, 1.0 + GLIDE_SECONDS),
            ScenePoint::of(to)
        );
        assert!(!effects.busy(10.0));
        effects.observe(&battle, 10.0, true);
        assert_eq!(effects.glides.len(), 0, "finished glides are dropped");
    }

    #[test]
    fn with_animation_off_the_state_is_adopted_at_once() {
        let mut battle = battle();
        let mut effects = Effects::default();
        effects.observe(&battle, 0.0, false);
        let me = battle.active().unwrap();
        let to = battle.reachable()[0];
        battle.move_to(to).unwrap();
        effects.observe(&battle, 1.0, false);
        assert!(!effects.busy(1.0));
        assert_eq!(effects.position(me, to, 1.0), ScenePoint::of(to));
    }
}
