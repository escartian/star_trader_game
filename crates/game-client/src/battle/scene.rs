//! Paints the battle volume and finds what the pointer is over. Ships are
//! sprite billboards standing on stalks over the floor grid, so their height
//! reads at a glance; real 3D models replace the sprites later.
use super::camera::{OrbitCamera, ScenePoint};
use crate::map::art::{ArtKind, Artwork};
use crate::theme::{AMBER, CYAN, GREEN};
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};
use game_core::combat::{
    Side,
    tactical::{Battle, Cell, DEPTH, HEIGHT, WIDTH},
};
use glam::Vec3;

pub const HOSTILE: Color32 = Color32::from_rgb(255, 104, 84);
const GRID: Color32 = Color32::from_rgba_premultiplied(60, 84, 86, 120);

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Cell(Cell),
    Unit(usize),
}

pub struct SceneInput<'a> {
    pub battle: &'a Battle,
    /// Cells the acting ship may still move to.
    pub reachable: &'a [Cell],
    /// Enemies the acting ship can fire on now.
    pub targets: &'a [usize],
    /// The layer whose move cells are shown and clickable.
    pub layer: i8,
    pub artwork: &'a Artwork,
}

pub fn side_color(side: Side) -> Color32 {
    match side {
        Side::Player => CYAN,
        Side::Hostile => HOSTILE,
    }
}

/// Billboard edge length in world units.
const SHIP_SIZE: f32 = 1.45;
/// Length of a 3D ship model in world units (one cell is 1.0).
const MODEL_SIZE: f32 = 1.6;

/// What is under `pointer`: ships first (nearest wins), then move cells on the
/// shown layer.
pub fn pick(
    rect: Rect,
    camera: &OrbitCamera,
    input: &SceneInput,
    pointer: Option<Pos2>,
) -> Option<Pick> {
    let pointer = pointer.filter(|pointer| rect.contains(*pointer))?;
    let ship = input
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.active())
        .filter_map(|(index, unit)| {
            let at = camera.project(rect, ScenePoint::of_cell(unit.cell))?;
            let radius = at.scale * SHIP_SIZE * 0.42;
            (at.pos.distance(pointer) <= radius).then_some((at.depth, index))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, index)) = ship {
        return Some(Pick::Unit(index));
    }
    input
        .reachable
        .iter()
        .filter(|cell| cell.z == input.layer)
        .filter_map(|cell| {
            let at = camera.project(rect, ScenePoint::of_cell(*cell))?;
            let reach = (at.scale * 0.32).max(7.0);
            let distance = at.pos.distance(pointer);
            (distance <= reach).then_some((distance, *cell))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, cell)| Pick::Cell(cell))
}

pub fn paint(
    painter: &Painter,
    rect: Rect,
    camera: &OrbitCamera,
    input: &SceneInput,
    hovered: Option<Pick>,
) {
    let painter = painter.with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(6, 10, 14));
    let view = View {
        painter: &painter,
        rect,
        camera,
    };
    view.floor();
    view.layer(input.layer);
    let active = input.battle.active();
    if let (Some(me), Some(Pick::Cell(cell))) = (active, hovered) {
        let from = ScenePoint::of_cell(input.battle.units[me].cell);
        let to = ScenePoint::of_cell(cell);
        view.line(from, to, Stroke::new(2.0_f32, GREEN.gamma_multiply(0.8)));
        view.line(
            to,
            to.on_floor(),
            Stroke::new(1.0_f32, GREEN.gamma_multiply(0.4)),
        );
    }
    for cell in input.reachable.iter().filter(|cell| cell.z == input.layer) {
        if let Some(at) = camera.project(rect, ScenePoint::of_cell(*cell)) {
            let lit = hovered == Some(Pick::Cell(*cell));
            let radius = (at.scale * if lit { 0.14 } else { 0.07 }).max(2.0);
            painter.circle_filled(
                at.pos,
                radius,
                GREEN.gamma_multiply(if lit { 1.0 } else { 0.55 }),
            );
        }
    }
    if let (Some(me), Some(Pick::Unit(target))) = (active, hovered)
        && input.targets.contains(&target)
    {
        let from = ScenePoint::of_cell(input.battle.units[me].cell);
        let to = ScenePoint::of_cell(input.battle.units[target].cell);
        view.line(from, to, Stroke::new(2.0_f32, HOSTILE));
    }
    view.ships(input, active, hovered);
}

struct View<'a> {
    painter: &'a Painter,
    rect: Rect,
    camera: &'a OrbitCamera,
}

impl View<'_> {
    fn at(&self, point: ScenePoint) -> Option<Pos2> {
        self.camera.project(self.rect, point).map(|at| at.pos)
    }

    fn line(&self, a: ScenePoint, b: ScenePoint, stroke: Stroke) {
        if let (Some(a), Some(b)) = (self.at(a), self.at(b)) {
            self.painter.line_segment([a, b], stroke);
        }
    }

    fn quad(&self, corners: [Vec3; 4], fill: Color32, stroke: Stroke) {
        let points: Option<Vec<Pos2>> = corners
            .iter()
            .map(|corner| self.at(ScenePoint(*corner)))
            .collect();
        if let Some(points) = points {
            self.painter
                .add(Shape::convex_polygon(points, fill, stroke));
        }
    }

    /// Floor grid, both deployment edges and the volume's outline.
    fn floor(&self) {
        let (hx, hy, hz) = half_extents();
        let floor = -hz;
        let edge = |x0: f32, x1: f32| {
            [
                Vec3::new(x0, -hy, floor),
                Vec3::new(x1, -hy, floor),
                Vec3::new(x1, hy, floor),
                Vec3::new(x0, hy, floor),
            ]
        };
        self.quad(
            edge(-hx, -hx + 1.0),
            CYAN.gamma_multiply(0.12),
            Stroke::NONE,
        );
        self.quad(
            edge(hx - 1.0, hx),
            HOSTILE.gamma_multiply(0.12),
            Stroke::NONE,
        );
        let stroke = Stroke::new(1.0_f32, GRID);
        for step in 0..=WIDTH {
            let x = -hx + f32::from(step);
            self.line(
                ScenePoint(Vec3::new(x, -hy, floor)),
                ScenePoint(Vec3::new(x, hy, floor)),
                stroke,
            );
        }
        for step in 0..=DEPTH {
            let y = -hy + f32::from(step);
            self.line(
                ScenePoint(Vec3::new(-hx, y, floor)),
                ScenePoint(Vec3::new(hx, y, floor)),
                stroke,
            );
        }
        let faint = Stroke::new(1.0_f32, GRID.gamma_multiply(0.5));
        for (x, y) in [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
            self.line(
                ScenePoint(Vec3::new(x, y, floor)),
                ScenePoint(Vec3::new(x, y, hz)),
                faint,
            );
        }
        let top = [
            Vec3::new(-hx, -hy, hz),
            Vec3::new(hx, -hy, hz),
            Vec3::new(hx, hy, hz),
            Vec3::new(-hx, hy, hz),
        ];
        for index in 0..4 {
            self.line(
                ScenePoint(top[index]),
                ScenePoint(top[(index + 1) % 4]),
                faint,
            );
        }
    }

    /// Outline of the layer whose move cells are shown.
    fn layer(&self, layer: i8) {
        let (hx, hy, _) = half_extents();
        let z = ScenePoint::of_cell(Cell {
            x: 0,
            y: 0,
            z: layer,
        })
        .0
        .z;
        self.quad(
            [
                Vec3::new(-hx, -hy, z),
                Vec3::new(hx, -hy, z),
                Vec3::new(hx, hy, z),
                Vec3::new(-hx, hy, z),
            ],
            GREEN.gamma_multiply(0.04),
            Stroke::new(1.0_f32, GREEN.gamma_multiply(0.35)),
        );
    }

    /// Ships far to near, each on a stalk to the floor, with hull and shield bars.
    fn ships(&self, input: &SceneInput, active: Option<usize>, hovered: Option<Pick>) {
        let mut ships: Vec<_> = input
            .battle
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.active())
            .filter_map(|(index, unit)| {
                let at = self
                    .camera
                    .project(self.rect, ScenePoint::of_cell(unit.cell))?;
                Some((index, unit, at))
            })
            .collect();
        ships.sort_by(|a, b| b.2.depth.total_cmp(&a.2.depth));
        for (index, unit, at) in ships {
            let color = side_color(unit.side);
            let point = ScenePoint::of_cell(unit.cell);
            self.line(
                point,
                point.on_floor(),
                Stroke::new(1.0_f32, color.gamma_multiply(0.45)),
            );
            if let Some(foot) = self.at(point.on_floor()) {
                self.painter
                    .circle_filled(foot, 2.5, color.gamma_multiply(0.6));
            }
            let size = at.scale * SHIP_SIZE;
            let ring = if Some(index) == active {
                Some((GREEN, 2.5_f32))
            } else if input.targets.contains(&index) {
                let lit = hovered == Some(Pick::Unit(index));
                Some((HOSTILE, if lit { 3.0 } else { 1.5 }))
            } else {
                None
            };
            if let Some((ring, width)) = ring {
                self.painter
                    .circle_stroke(at.pos, size * 0.45, Stroke::new(width, ring));
            }
            let facing_left = unit.side == Side::Hostile;
            if let Some(model) = super::model::for_kind(unit.kind) {
                let projector = self.camera.projector(self.rect);
                model.paint(self.painter, &projector, point, MODEL_SIZE, facing_left);
            } else if !input.artwork.paint_facing(
                ArtKind::from(unit.kind),
                self.painter,
                at.pos,
                size,
                facing_left,
            ) {
                self.painter.circle_filled(at.pos, size * 0.2, color);
            }
            self.bars(unit, at.pos, size);
        }
    }

    fn bars(&self, unit: &game_core::combat::tactical::Unit, centre: Pos2, size: f32) {
        let width = (size * 0.7).max(18.0);
        let left = centre.x - width / 2.0;
        let top = centre.y - size * 0.55;
        let bar = |row: f32, fraction: f32, color: Color32| {
            let back = Rect::from_min_size(Pos2::new(left, top + row), egui::vec2(width, 3.0));
            self.painter
                .rect_filled(back, 0.0, Color32::from_black_alpha(170));
            let mut fill = back;
            fill.set_width(width * fraction.clamp(0.0, 1.0));
            self.painter.rect_filled(fill, 0.0, color);
        };
        let hull = unit.profile.hull.max(1) as f32;
        let left_hull = (unit.profile.hull - unit.damage) as f32 / hull;
        let hull_color = match left_hull {
            f if f > 0.6 => GREEN,
            f if f > 0.3 => AMBER,
            _ => HOSTILE,
        };
        bar(0.0, left_hull, hull_color);
        if unit.profile.shields > 0 {
            bar(4.0, unit.shields as f32 / unit.profile.shields as f32, CYAN);
        }
    }
}

fn half_extents() -> (f32, f32, f32) {
    (
        f32::from(WIDTH) / 2.0,
        f32::from(DEPTH) / 2.0,
        f32::from(HEIGHT) / 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat::{CombatCatalog, engagement::CombatantSetup};
    use game_core::fleet::{ShipCatalog, ShipKind};

    fn battle() -> Battle {
        let setup = |kind, side| CombatantSetup {
            name: format!("{kind:?}"),
            side,
            ship: None,
            kind,
            profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
            hull_damage: 0,
        };
        let setups = [
            setup(ShipKind::Freighter, Side::Player),
            setup(ShipKind::Scout, Side::Hostile),
        ];
        Battle::new(&setups, &CombatCatalog::default(), 1).unwrap()
    }

    #[test]
    fn pointing_at_a_ship_or_a_move_dot_picks_it_and_other_layers_are_ignored() {
        let battle = battle();
        let camera = OrbitCamera::default();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 560.0));
        let artwork = Artwork::preload(&egui::Context::default());
        let here = battle.units[0].cell;
        let step = Cell {
            x: here.x + 1,
            ..here
        };
        let above = Cell {
            z: here.z + 1,
            x: here.x + 1,
            ..here
        };
        let reachable = [step, above];
        let input = SceneInput {
            battle: &battle,
            reachable: &reachable,
            targets: &[],
            layer: here.z,
            artwork: &artwork,
        };
        let screen = |cell| camera.project(rect, ScenePoint::of_cell(cell)).unwrap().pos;
        assert_eq!(
            pick(rect, &camera, &input, Some(screen(here))),
            Some(Pick::Unit(0))
        );
        let raider = battle.units[1].cell;
        assert_eq!(
            pick(rect, &camera, &input, Some(screen(raider))),
            Some(Pick::Unit(1))
        );
        assert_eq!(
            pick(rect, &camera, &input, Some(screen(step))),
            Some(Pick::Cell(step))
        );
        // `above` is on another layer: not shown, so not clickable.
        assert_ne!(
            pick(rect, &camera, &input, Some(screen(above))),
            Some(Pick::Cell(above))
        );
        assert_eq!(
            pick(rect, &camera, &input, Some(Pos2::new(-5.0, -5.0))),
            None
        );
    }
}
