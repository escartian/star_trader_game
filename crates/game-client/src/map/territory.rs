//! Galaxy territory display: smooth faction regions around held stars.
//!
//! Every star (held or independent) spreads a soft influence that fades to
//! nothing at [`FALLOFF_LY`]. A point belongs to the faction whose stars'
//! summed influence there is strongest, provided it beats [`THRESHOLD`] and
//! every rival (independent stars count as a rival, so they carve gaps). The
//! threshold puts a lone star's edge at about 10 ly; holdings up to about 20 ly
//! apart merge into one rounded region. Outlines are traced with marching
//! squares, so edges are smooth curves rather than arcs of circles.
//!
//! Geometry lives in the flat galaxy's x/z light-year plane and is cached
//! until ownership changes; only the projection runs each frame. Ownership
//! stays in the session.
use super::Map;
use egui::{Color32, Mesh, Painter, Rect, Stroke};
use game_core::factions::Faction;
use glam::{DVec2, DVec3};

/// Influence reaches zero this far from a star (ly).
const FALLOFF_LY: f64 = 14.0;
/// Influence a point needs to be held: a lone star's edge lands at 10 ly.
const THRESHOLD: f64 = 0.1176;
/// Sampling grid step (ly); outlines are interpolated between samples.
const STEP_LY: f64 = 1.25;

#[derive(Clone, Copy)]
struct Site {
    point: DVec2,
    owner: Option<Faction>,
}

/// One faction's traced region, in galaxy light-years.
struct Region {
    color: Color32,
    triangles: Vec<[DVec2; 3]>,
    /// Border lines, each a chain of points (closed loops where possible).
    outlines: Vec<Vec<DVec2>>,
}

/// Each system's position (ly) and holder: what the regions were traced from.
type Layout = Vec<(DVec2, Option<(Faction, Color32)>)>;

/// Regions for one ownership layout.
pub(super) struct TerritoryCache {
    key: Layout,
    regions: Vec<Region>,
}

impl Map {
    pub(super) fn draw_territory(&self, painter: &Painter, rect: Rect) {
        let key: Layout = self
            .systems
            .iter()
            .map(|system| {
                let point = DVec2::new(system.position.x as f64, system.position.z as f64);
                let holder = self
                    .session
                    .system_holder(system.id)
                    .map(|(faction, color, _)| (faction, color));
                (point, holder)
            })
            .collect();
        let mut cache = self.territory.borrow_mut();
        if cache.as_ref().is_none_or(|cache| cache.key != key) {
            let sites: Vec<Site> = key
                .iter()
                .map(|(point, owner)| Site {
                    point: *point,
                    owner: owner.map(|(faction, _)| faction),
                })
                .collect();
            let mut colors: Vec<(Faction, Color32)> =
                key.iter().filter_map(|(_, owner)| *owner).collect();
            colors.sort_by_key(|(faction, _)| *faction);
            colors.dedup_by_key(|(faction, _)| *faction);
            let regions = colors
                .into_iter()
                .map(|(faction, color)| {
                    let (triangles, border) = trace(&sites, faction);
                    Region {
                        color,
                        triangles,
                        outlines: chain(border),
                    }
                })
                .collect();
            *cache = Some(TerritoryCache { key, regions });
        }
        let project = |point: DVec2| self.camera.project(DVec3::new(point.x, 0.0, point.y), rect);
        for region in &cache.as_ref().expect("filled above").regions {
            let mut fill = Mesh::default();
            let tint = region.color.gamma_multiply(0.13);
            for triangle in &region.triangles {
                let first = fill.vertices.len() as u32;
                for point in triangle {
                    fill.colored_vertex(project(*point), tint);
                }
                fill.add_triangle(first, first + 1, first + 2);
            }
            painter.add(egui::Shape::mesh(fill));
            let stroke = Stroke::new(1.5_f32, region.color.gamma_multiply(0.78));
            for outline in &region.outlines {
                let points = outline.iter().map(|point| project(*point)).collect();
                painter.add(egui::Shape::line(points, stroke));
            }
        }
    }
}

/// Smooth falloff: 1 at the star, 0 at [`FALLOFF_LY`], flat at both ends.
fn kernel(distance_squared: f64) -> f64 {
    let t = 1.0 - distance_squared / (FALLOFF_LY * FALLOFF_LY);
    if t <= 0.0 { 0.0 } else { t * t * t }
}

/// How far `faction` holds `point`: positive inside its region, negative
/// outside, zero on the border.
fn margin(point: DVec2, sites: &[Site], faction: Faction) -> f64 {
    let mut ours = 0.0;
    let mut rivals: Vec<(Option<Faction>, f64)> = Vec::new();
    for site in sites {
        let weight = kernel((point - site.point).length_squared());
        if weight == 0.0 {
            continue;
        }
        if site.owner == Some(faction) {
            ours += weight;
        } else if let Some(entry) = rivals.iter_mut().find(|(owner, _)| *owner == site.owner) {
            entry.1 += weight;
        } else {
            rivals.push((site.owner, weight));
        }
    }
    let strongest = rivals
        .iter()
        .map(|(_, weight)| *weight)
        .fold(THRESHOLD, f64::max);
    ours - strongest
}

/// Fill triangles and border segments of `faction`'s region (marching squares).
fn trace(sites: &[Site], faction: Faction) -> (Vec<[DVec2; 3]>, Vec<[DVec2; 2]>) {
    let held: Vec<DVec2> = sites
        .iter()
        .filter(|site| site.owner == Some(faction))
        .map(|site| site.point)
        .collect();
    let (mut triangles, mut border) = (Vec::new(), Vec::new());
    let Some(first) = held.first() else {
        return (triangles, border);
    };
    let (mut low, mut high) = (*first, *first);
    for point in &held {
        low = low.min(*point);
        high = high.max(*point);
    }
    let low = low - DVec2::splat(FALLOFF_LY);
    let columns = ((high.x - low.x + FALLOFF_LY * 2.0) / STEP_LY).ceil() as usize + 1;
    let rows = ((high.y - low.y + FALLOFF_LY * 2.0) / STEP_LY).ceil() as usize + 1;
    let at = |column: usize, row: usize| low + DVec2::new(column as f64, row as f64) * STEP_LY;
    let values: Vec<f64> = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (column, row)))
        .map(|(column, row)| margin(at(column, row), sites, faction))
        .collect();
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            // Corners counter-clockwise from the low corner.
            let cells = [
                (column, row),
                (column + 1, row),
                (column + 1, row + 1),
                (column, row + 1),
            ];
            let corners = cells.map(|(c, r)| (at(c, r), values[r * columns + c]));
            square(corners, &mut triangles, &mut border);
        }
    }
    (triangles, border)
}

/// Joins border pieces end to end into lines, so the stroke is drawn as
/// continuous curves rather than many separate dashes.
fn chain(segments: Vec<[DVec2; 2]>) -> Vec<Vec<DVec2>> {
    use std::collections::HashMap;
    let key = |point: DVec2| {
        (
            (point.x * 1e6).round() as i64,
            (point.y * 1e6).round() as i64,
        )
    };
    let mut at: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, [from, to]) in segments.iter().enumerate() {
        at.entry(key(*from)).or_default().push(index);
        at.entry(key(*to)).or_default().push(index);
    }
    let mut used = vec![false; segments.len()];
    let mut lines = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut line = vec![segments[start][0], segments[start][1]];
        // Extend forward from the last point, then backward from the first.
        for _ in 0..2 {
            loop {
                let tail = *line.last().expect("two points");
                let next = at[&key(tail)].iter().copied().find(|index| !used[*index]);
                let Some(next) = next else { break };
                used[next] = true;
                let [from, to] = segments[next];
                line.push(if key(from) == key(tail) { to } else { from });
            }
            line.reverse();
        }
        lines.push(line);
    }
    lines
}

/// One grid square: its inside part as triangles, and its border pieces.
fn square(
    corners: [(DVec2, f64); 4],
    triangles: &mut Vec<[DVec2; 3]>,
    border: &mut Vec<[DVec2; 2]>,
) {
    let inside = corners.map(|(_, value)| value > 0.0);
    if inside.iter().all(|inside| !inside) {
        return;
    }
    // Where the border crosses edge `i` (from corner i to corner i + 1).
    let crossing = |i: usize| {
        let (from, a) = corners[i];
        let (to, b) = corners[(i + 1) % 4];
        (inside[i] != inside[(i + 1) % 4]).then(|| from + (to - from) * (a / (a - b)))
    };
    let edges = [crossing(0), crossing(1), crossing(2), crossing(3)];
    let saddle = inside[0] == inside[2] && inside[1] == inside[3] && inside[0] != inside[1];
    let center_inside = corners.iter().map(|(_, value)| value).sum::<f64>() > 0.0;
    if saddle && !center_inside {
        // Two separate corners inside: a small triangle at each.
        for i in [0, 2]
            .into_iter()
            .filter(|i| inside[*i])
            .chain([1, 3].into_iter().filter(|i| inside[*i]))
        {
            let (before, after) = (edges[(i + 3) % 4], edges[i]);
            if let (Some(before), Some(after)) = (before, after) {
                triangles.push([corners[i].0, after, before]);
                border.push([before, after]);
            }
        }
        return;
    }
    // Walk the outline: inside corners and crossings in order; fan-fill it.
    let mut outline = Vec::with_capacity(8);
    for i in 0..4 {
        if inside[i] {
            outline.push(corners[i].0);
        }
        if let Some(point) = edges[i] {
            outline.push(point);
        }
    }
    for index in 1..outline.len().saturating_sub(1) {
        triangles.push([outline[0], outline[index], outline[index + 1]]);
    }
    let crossings: Vec<DVec2> = edges.iter().flatten().copied().collect();
    match crossings.len() {
        2 => border.push([crossings[0], crossings[1]]),
        // Saddle with the centre inside: the two outside corners are cut off.
        4 => {
            for i in (0..4).filter(|i| !inside[*i]) {
                if let (Some(before), Some(after)) = (edges[(i + 3) % 4], edges[i]) {
                    border.push([before, after]);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(x: f64, owner: Option<Faction>) -> Site {
        Site {
            point: DVec2::new(x, 0.0),
            owner,
        }
    }

    fn holds(point: f64, sites: &[Site]) -> bool {
        margin(DVec2::new(point, 0.0), sites, Faction::Federation) > 0.0
    }

    #[test]
    fn a_lone_star_holds_about_ten_light_years() {
        let sites = [site(0.0, Some(Faction::Federation))];
        assert!(holds(9.8, &sites));
        assert!(!holds(10.2, &sites));
    }

    #[test]
    fn nearby_holdings_join_and_distant_ones_stay_apart() {
        let near = [
            site(0.0, Some(Faction::Federation)),
            site(12.0, Some(Faction::Federation)),
        ];
        assert!(holds(6.0, &near), "one region between them");
        let far = [
            site(0.0, Some(Faction::Federation)),
            site(25.0, Some(Faction::Federation)),
        ];
        assert!(!holds(12.5, &far));
    }

    #[test]
    fn independent_stars_and_rivals_carve_the_region() {
        let split = [
            site(0.0, Some(Faction::Federation)),
            site(7.0, None),
            site(14.0, Some(Faction::Federation)),
        ];
        assert!(!holds(7.0, &split), "the independent star keeps its space");
        let rivals = [
            site(0.0, Some(Faction::Federation)),
            site(12.0, Some(Faction::Empire)),
        ];
        assert!(
            holds(5.5, &rivals) && !holds(6.5, &rivals),
            "border near halfway"
        );
    }

    #[test]
    fn a_traced_region_is_closed_and_round() {
        let sites = [site(0.0, Some(Faction::Federation))];
        let (triangles, border) = trace(&sites, Faction::Federation);
        assert!(!triangles.is_empty());
        let outlines = chain(border.clone());
        assert_eq!(outlines.len(), 1, "one continuous line");
        assert_eq!(outlines[0].first(), outlines[0].last(), "that closes");
        // Every border point lies near the 10 ly edge.
        for [from, to] in &border {
            for point in [from, to] {
                assert!((point.length() - 10.0).abs() < 0.2, "{point:?}");
            }
        }
        // Each border end meets exactly one other segment: a closed loop.
        let key = |point: &DVec2| {
            (
                (point.x * 1e6).round() as i64,
                (point.y * 1e6).round() as i64,
            )
        };
        let mut ends = std::collections::HashMap::new();
        for [from, to] in &border {
            *ends.entry(key(from)).or_insert(0) += 1;
            *ends.entry(key(to)).or_insert(0) += 1;
        }
        assert!(ends.values().all(|count| *count == 2));
    }
}
