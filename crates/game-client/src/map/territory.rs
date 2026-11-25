//! Galaxy territory display: compact Voronoi cells join into faction regions.
//! All geometry is in the flat galaxy's x/z light-year plane; camera projection
//! happens only after the borders are built. Ownership stays in the session.
use super::Map;
use egui::{Color32, Mesh, Painter, Pos2, Rect, Stroke};
use game_core::factions::Faction;
use glam::{DVec2, DVec3};

const REACH_LY: f64 = 10.0;
const SIDES: usize = 24;
const BORDER_PROBE_LY: f64 = 0.05;

#[derive(Clone, Copy)]
struct Site {
    point: DVec2,
    owner: Option<(Faction, Color32)>,
}

impl Map {
    pub(super) fn draw_territory(&self, painter: &Painter, rect: Rect) {
        let sites: Vec<_> = self
            .systems
            .iter()
            .map(|system| Site {
                point: DVec2::new(system.position.x as f64, system.position.z as f64),
                owner: self
                    .session
                    .system_holder(system.id)
                    .map(|(faction, color, _)| (faction, color)),
            })
            .collect();
        let mut fill = Mesh::default();
        let mut borders = Vec::new();
        for site in sites.iter().filter(|site| site.owner.is_some()) {
            let (faction, color) = site.owner.expect("filtered to held sites");
            let polygon = cell(site.point, &sites);
            if polygon.len() < 3 {
                continue;
            }
            let vertices: Vec<Pos2> = polygon
                .iter()
                .map(|point| self.camera.project(DVec3::new(point.x, 0.0, point.y), rect))
                .collect();
            let first = fill.vertices.len() as u32;
            for vertex in &vertices {
                fill.colored_vertex(*vertex, color.gamma_multiply(0.13));
            }
            for index in 1..vertices.len() - 1 {
                fill.add_triangle(first, first + index as u32, first + index as u32 + 1);
            }
            // A cell edge is a faction border only when the region immediately
            // beyond it is independent or held by another faction.
            for index in 0..polygon.len() {
                let next = (index + 1) % polygon.len();
                let edge = polygon[next] - polygon[index];
                if edge.length_squared() < 1e-12 {
                    continue;
                }
                let outward = DVec2::new(edge.y, -edge.x).normalize();
                let probe = (polygon[index] + polygon[next]) * 0.5 + outward * BORDER_PROBE_LY;
                let other = owner_at(probe, &sites);
                if other == Some(faction) || other.is_some_and(|other| faction > other) {
                    continue;
                }
                borders.push(([vertices[index], vertices[next]], color));
            }
        }
        if !fill.is_empty() {
            painter.add(egui::Shape::mesh(fill));
        }
        for (edge, color) in borders {
            painter.line_segment(edge, Stroke::new(1.5_f32, color.gamma_multiply(0.78)));
        }
    }
}

/// A claimed star owns points within ten light-years that are closer to it
/// than to any other star. Adjacent cells of one faction make a single region.
fn cell(center: DVec2, sites: &[Site]) -> Vec<DVec2> {
    // Circumscribed polygon gives the straight edge segments a 10 ly apothem.
    let vertex_radius = REACH_LY / (std::f64::consts::PI / SIDES as f64).cos();
    let mut polygon = (0..SIDES)
        .map(|index| {
            let angle = index as f64 * std::f64::consts::TAU / SIDES as f64;
            center + DVec2::new(angle.cos(), angle.sin()) * vertex_radius
        })
        .collect::<Vec<_>>();
    for other in sites {
        let difference = other.point - center;
        if difference.length_squared() < 1e-12
            || difference.length_squared() >= (REACH_LY * 2.1).powi(2)
        {
            continue;
        }
        polygon = clip_nearer(polygon, (center + other.point) * 0.5, difference);
        if polygon.is_empty() {
            break;
        }
    }
    polygon
}

/// Keep the side of a bisector nearer `center` than the other star.
fn clip_nearer(polygon: Vec<DVec2>, midpoint: DVec2, toward_other: DVec2) -> Vec<DVec2> {
    let mut clipped = Vec::new();
    for index in 0..polygon.len() {
        let from = polygon[index];
        let to = polygon[(index + 1) % polygon.len()];
        let from_side = (from - midpoint).dot(toward_other);
        let to_side = (to - midpoint).dot(toward_other);
        if (from_side < 0.0) != (to_side < 0.0) {
            let fraction = from_side / (from_side - to_side);
            clipped.push(from + (to - from) * fraction);
        }
        if to_side <= 0.0 {
            clipped.push(to);
        }
    }
    clipped
}

fn owner_at(point: DVec2, sites: &[Site]) -> Option<Faction> {
    sites
        .iter()
        .map(|site| ((point - site.point).length_squared(), site.owner))
        .min_by(|(left, _), (right, _)| left.total_cmp(right))
        .and_then(|(distance, owner)| (distance <= REACH_LY.powi(2)).then_some(owner))
        .flatten()
        .map(|(faction, _)| faction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(x: f64, owner: Option<Faction>) -> Site {
        Site {
            point: DVec2::new(x, 0.0),
            owner: owner.map(|faction| (faction, Color32::WHITE)),
        }
    }

    #[test]
    fn nearby_holdings_join_without_an_internal_border() {
        let sites = [
            site(0.0, Some(Faction::Federation)),
            site(12.0, Some(Faction::Federation)),
        ];
        assert_eq!(
            owner_at(DVec2::new(6.1, 0.0), &sites),
            Some(Faction::Federation)
        );
        assert!(
            cell(sites[0].point, &sites)
                .iter()
                .all(|point| point.x <= 6.0 + 1e-8)
        );
    }

    #[test]
    fn neutral_stars_and_distant_holdings_split_regions() {
        let sites = [
            site(0.0, Some(Faction::Federation)),
            site(7.0, None),
            site(14.0, Some(Faction::Federation)),
        ];
        assert_eq!(owner_at(DVec2::new(7.0, 0.0), &sites), None);
        let distant = [
            site(0.0, Some(Faction::Federation)),
            site(25.0, Some(Faction::Federation)),
        ];
        assert_eq!(owner_at(DVec2::new(12.5, 0.0), &distant), None);
    }
}
