//! The Songbook's transition map. The music's parts are nodes placed by tempo
//! (60 BPM on the left, 120 on the right) inside outlines for their sections,
//! with arrows for the transitions the music may take. Drawing only: the part
//! table and the window live in `songbook`, playback in `audio`.
use super::songbook::{PARTS, Part};
use crate::theme::{AMBER, CYAN, GREEN, PANEL};
use egui::epaint::QuadraticBezierShape;
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::sync::OnceLock;

/// The map's smallest size in points. It stretches to fill a larger window
/// (spreading the parts out) and scrolls in a smaller one.
pub(super) const MIN_CANVAS: Vec2 = Vec2::new(760.0, 540.0);
const NODE_RADIUS: f32 = 10.0;
/// Canvas margin left and right of the 60 and 120 BPM columns, in points,
/// leaving room for arrows that bow outward from the edge columns.
const SIDE_MARGIN: f32 = 90.0;

/// Rows as fractions of the canvas height, from the top.
const ROW_FUNK_RISE: f32 = 0.05;
const ROW_FUNK: f32 = 0.12;
const ROW_FUNK_FALL: f32 = 0.19;
const ROW_EXTRA: f32 = 0.30;
const ROW_CLIMB: f32 = 0.40;
const ROW_ALTERNATE: f32 = 0.49;
const ROW_RETURN: f32 = 0.60;
const ROW_SPACE_RISE: f32 = 0.73;
const ROW_SPACE: f32 = 0.82;
const ROW_SPACE_FALL: f32 = 0.92;

/// Each part's tempo column in BPM (a ramp sits midway between its two
/// tempos) and its row.
const PLACES: &[(&str, f32, f32)] = &[
    ("drift_a", 60.0, ROW_CLIMB),
    ("drift_b", 60.0, ROW_ALTERNATE),
    ("rise_70", 65.0, ROW_CLIMB),
    ("rise_80", 75.0, ROW_CLIMB),
    ("depart_a", 80.0, ROW_CLIMB),
    ("depart_b", 80.0, ROW_ALTERNATE),
    ("rise_90", 85.0, ROW_CLIMB),
    ("rise_100", 95.0, ROW_CLIMB),
    ("cruise_a", 100.0, ROW_CLIMB),
    ("cruise_b", 100.0, ROW_ALTERNATE),
    ("solo", 100.0, ROW_EXTRA),
    ("rise_110", 105.0, ROW_CLIMB),
    ("rise_120", 115.0, ROW_CLIMB),
    ("arrive_a", 120.0, ROW_CLIMB),
    ("arrive_b", 120.0, ROW_ALTERNATE),
    ("fanfare", 120.0, ROW_EXTRA),
    ("fall_110", 115.0, ROW_RETURN),
    ("fall_100", 105.0, ROW_RETURN),
    ("fall_90", 95.0, ROW_RETURN),
    ("fall_80", 85.0, ROW_RETURN),
    ("wind", 80.0, ROW_RETURN),
    ("fall_70", 75.0, ROW_RETURN),
    ("fall_60", 65.0, ROW_RETURN),
    ("space", 60.0, ROW_SPACE),
    ("space_80", 80.0, ROW_SPACE),
    ("space_100", 100.0, ROW_SPACE),
    ("space_120", 120.0, ROW_SPACE),
    ("space_rise_80", 70.0, ROW_SPACE_RISE),
    ("space_rise_100", 90.0, ROW_SPACE_RISE),
    ("space_rise_120", 110.0, ROW_SPACE_RISE),
    ("space_fall_100", 110.0, ROW_SPACE_FALL),
    ("space_fall_80", 90.0, ROW_SPACE_FALL),
    ("space_fall_60", 70.0, ROW_SPACE_FALL),
    ("funk_60", 60.0, ROW_FUNK),
    ("funk_80", 80.0, ROW_FUNK),
    ("funk_100", 100.0, ROW_FUNK),
    ("funk_120", 120.0, ROW_FUNK),
    ("funk_rise_80", 70.0, ROW_FUNK_RISE),
    ("funk_rise_100", 90.0, ROW_FUNK_RISE),
    ("funk_rise_120", 110.0, ROW_FUNK_RISE),
    ("funk_fall_100", 110.0, ROW_FUNK_FALL),
    ("funk_fall_80", 90.0, ROW_FUNK_FALL),
    ("funk_fall_60", 70.0, ROW_FUNK_FALL),
];

fn section_color(section: &str) -> Color32 {
    match section {
        "Drift" => Color32::from_rgb(120, 170, 255),
        "Ascent" => CYAN,
        "Departure" => Color32::from_rgb(110, 230, 170),
        "Cruise" => GREEN,
        "Arrival" => AMBER,
        "Return" => Color32::from_rgb(230, 140, 200),
        "Funk" => Color32::from_rgb(255, 120, 90),
        _ => Color32::from_rgb(175, 145, 255),
    }
}

/// One possible move from a part, with its chance of being chosen.
pub(super) struct Transition {
    pub from: &'static str,
    pub to: &'static str,
    pub chance: f32,
}

/// The transitions in the embedded music manifest, with weights as chances.
pub(super) fn transitions() -> &'static [Transition] {
    static TRANSITIONS: OnceLock<Vec<Transition>> = OnceLock::new();
    TRANSITIONS.get_or_init(|| {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../../../assets/music/manifest.json"))
                .unwrap_or_default();
        let known = |name: &str| {
            PARTS
                .iter()
                .find(|part| part.id == name)
                .map(|part| part.id)
        };
        let mut list = Vec::new();
        for (name, block) in manifest["blocks"].as_object().into_iter().flatten() {
            let Some(from) = known(name) else { continue };
            let next = block["next"].as_object();
            let total: f64 = next
                .into_iter()
                .flatten()
                .filter_map(|(_, w)| w.as_f64())
                .sum();
            for (target, weight) in next.into_iter().flatten() {
                if let (Some(to), Some(weight)) = (known(target), weight.as_f64()) {
                    let chance = (weight / total) as f32;
                    list.push(Transition { from, to, chance });
                }
            }
        }
        list
    })
}

/// A part's centre on the map's `canvas`.
fn place(id: &str, canvas: Rect) -> Option<Pos2> {
    let (_, bpm, row) = PLACES.iter().find(|(place, ..)| *place == id)?;
    let span = canvas.width() - 2.0 * SIDE_MARGIN;
    Some(
        canvas.min
            + Vec2::new(
                SIDE_MARGIN + (bpm - 60.0) / 60.0 * span,
                row * canvas.height(),
            ),
    )
}

/// What the map shows besides the parts themselves.
pub(super) struct Highlights {
    pub current: Option<&'static str>,
    pub next: Option<&'static str>,
    pub time: f64,
}

/// What the viewer did on the map this frame.
#[derive(Default)]
pub(super) struct MapInput {
    pub hovered: Option<&'static Part>,
    pub chosen: Option<&'static str>,
}

/// Paints the map at the cursor, `size` points large (at least `MIN_CANVAS`),
/// and returns the hovered and clicked parts.
pub(super) fn show(ui: &mut Ui, highlights: &Highlights, size: Vec2) -> MapInput {
    let (response, painter) = ui.allocate_painter(size.max(MIN_CANVAS), Sense::click());
    let canvas = response.rect;
    let hovered = response.hover_pos().and_then(|pointer| {
        PARTS.iter().find(|part| {
            place(part.id, canvas).is_some_and(|at| at.distance(pointer) <= NODE_RADIUS + 5.0)
        })
    });
    let focus = hovered.map(|part| part.id).or(highlights.current);
    paint_sections(&painter, canvas);
    let chances = paint_transitions(&painter, canvas, focus);
    paint_parts(&painter, canvas, highlights, hovered.map(|part| part.id));
    for (at, chance) in chances {
        tag(&painter, at, Align2::CENTER_CENTER, &chance, GREEN);
    }
    let chosen = if response.clicked() {
        hovered.map(|part| part.id)
    } else {
        None
    };
    if hovered.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    MapInput { hovered, chosen }
}

fn paint_sections(painter: &Painter, canvas: Rect) {
    let mut sections: Vec<&str> = Vec::new();
    for part in PARTS {
        if !sections.contains(&part.section) {
            sections.push(part.section);
        }
    }
    for section in sections {
        let centres = PARTS
            .iter()
            .filter(|part| part.section == section)
            .filter_map(|part| place(part.id, canvas));
        let Some(bounds) = centres
            .map(|at| Rect::from_center_size(at, Vec2::ZERO))
            .reduce(Rect::union)
        else {
            continue;
        };
        // Room for the node labels below and the section name above.
        let outline = Rect::from_min_max(
            bounds.min - Vec2::new(25.0, 24.0),
            bounds.max + Vec2::new(25.0, 26.0),
        );
        let color = section_color(section);
        let radius = (outline.height().min(outline.width()) / 2.0).min(40.0);
        painter.rect_filled(outline, radius, color.gamma_multiply(0.06));
        painter.rect_stroke(
            outline,
            radius,
            Stroke::new(1.0_f32, color.gamma_multiply(0.55)),
            StrokeKind::Inside,
        );
        // The name sits on the outline's top edge, clear of the parts inside.
        let name = outline.left_top() + Vec2::new(radius.max(12.0), 0.0);
        tag(
            painter,
            name,
            Align2::LEFT_CENTER,
            &section.to_uppercase(),
            color,
        );
    }
}

/// Paints every transition; returns the focused part's chance labels and
/// where they go, for painting above the parts.
fn paint_transitions(painter: &Painter, canvas: Rect, focus: Option<&str>) -> Vec<(Pos2, String)> {
    let mut chances = Vec::new();
    for transition in transitions() {
        let (Some(from), Some(to)) = (place(transition.from, canvas), place(transition.to, canvas))
        else {
            continue;
        };
        let leaves_focus = focus == Some(transition.from);
        let enters_focus = focus == Some(transition.to);
        let (stroke, head) = if leaves_focus {
            (Stroke::new(1.2 + 3.0 * transition.chance, GREEN), 7.0)
        } else if enters_focus {
            (Stroke::new(1.2_f32, CYAN.gamma_multiply(0.7)), 5.0)
        } else {
            (
                Stroke::new(1.0_f32, Color32::from_gray(150).gamma_multiply(0.22)),
                4.0,
            )
        };
        let curve = arrow(painter, from, to, stroke, head);
        if leaves_focus {
            chances.push((curve, format!("{:.0}%", transition.chance * 100.0)));
        }
    }
    chances
}

/// Small text on a dark pill, readable over lines and outlines.
fn tag(painter: &Painter, at: Pos2, align: Align2, text: &str, color: Color32) {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(10.0), color);
    let rect = align
        .anchor_size(at, galley.size())
        .expand2(Vec2::new(4.0, 1.0));
    painter.rect_filled(rect, 6.0, PANEL.gamma_multiply(0.92));
    painter.galley(rect.min + Vec2::new(4.0, 1.0), galley, color);
}

/// Draws a curved arrow between two node centres and returns its midpoint.
/// Arrows bow to their left, so the two directions of a pair stay apart.
fn arrow(painter: &Painter, from: Pos2, to: Pos2, stroke: Stroke, head: f32) -> Pos2 {
    let along = to - from;
    let normal = Vec2::new(along.y, -along.x).normalized();
    // Longer arrows bow further, so they pass around the parts between.
    let control = from + along * 0.5 + normal * (along.length() * 0.3).max(10.0);
    let start = from + (control - from).normalized() * (NODE_RADIUS + 2.0);
    let tangent = (to - control).normalized();
    let end = to - tangent * (NODE_RADIUS + 3.0);
    painter.add(QuadraticBezierShape::from_points_stroke(
        [start, control, end],
        false,
        Color32::TRANSPARENT,
        stroke,
    ));
    let side = Vec2::new(-tangent.y, tangent.x) * head * 0.55;
    let back = end - tangent * head;
    painter.add(egui::Shape::convex_polygon(
        vec![end, back + side, back - side],
        stroke.color,
        Stroke::NONE,
    ));
    // The midpoint of a quadratic Bézier curve.
    Pos2::new(
        0.25 * start.x + 0.5 * control.x + 0.25 * end.x,
        0.25 * start.y + 0.5 * control.y + 0.25 * end.y,
    )
}

fn paint_parts(painter: &Painter, canvas: Rect, highlights: &Highlights, hovered: Option<&str>) {
    for part in PARTS {
        let Some(at) = place(part.id, canvas) else {
            continue;
        };
        let color = section_color(part.section);
        let current = highlights.current == Some(part.id);
        if highlights.next == Some(part.id) {
            // A ring that pulses about once a second around the part that plays next.
            let pulse = (highlights.time * std::f64::consts::TAU).sin() as f32 * 0.5 + 0.5;
            painter.circle_stroke(
                at,
                NODE_RADIUS + 3.0 + 3.0 * pulse,
                Stroke::new(1.5_f32, CYAN.gamma_multiply(0.5 + 0.5 * pulse)),
            );
        }
        let fill = if current { GREEN } else { PANEL };
        let width: f32 = if hovered == Some(part.id) { 2.5 } else { 1.5 };
        painter.circle(
            at,
            NODE_RADIUS,
            fill,
            Stroke::new(width, if current { GREEN } else { color }),
        );
        painter.text(
            at + Vec2::new(0.0, NODE_RADIUS + 3.0),
            Align2::CENTER_TOP,
            short_title(part.title),
            FontId::proportional(11.0),
            if current {
                GREEN
            } else {
                Color32::from_gray(215)
            },
        );
    }
}

/// Compact node labels: "Rise to 110" -> "Rise 110".
fn short_title(title: &str) -> String {
    match title {
        "Guitar Solo" => "Solo".into(),
        "Orbital Fanfare" => "Fanfare".into(),
        _ => title.replace(" to ", " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_part_has_one_place_and_none_overlap() {
        let placed: BTreeSet<_> = PLACES.iter().map(|(id, ..)| *id).collect();
        let parts: BTreeSet<_> = PARTS.iter().map(|part| part.id).collect();
        assert_eq!(placed.len(), PLACES.len(), "each part is placed once");
        assert_eq!(placed, parts);
        let centres: Vec<_> = PLACES
            .iter()
            .filter_map(|(id, ..)| place(id, Rect::from_min_size(Pos2::ZERO, MIN_CANVAS)))
            .collect();
        for (index, a) in centres.iter().enumerate() {
            for b in &centres[index + 1..] {
                assert!(
                    a.distance(*b) > 4.0 * NODE_RADIUS,
                    "nodes at {a:?} and {b:?} overlap"
                );
            }
        }
    }

    #[test]
    fn transitions_cover_every_part_with_chances_summing_to_one() {
        for part in PARTS {
            let total: f32 = transitions()
                .iter()
                .filter(|transition| transition.from == part.id)
                .map(|transition| transition.chance)
                .sum();
            assert!(
                (total - 1.0).abs() < 1e-4,
                "{} has chances summing to {total}",
                part.id
            );
        }
    }
}
