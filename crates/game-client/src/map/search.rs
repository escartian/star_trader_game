//! Finding a star system or a body (planet, moon, station) by name from the
//! galaxy map, for galaxies too large to scan by eye. Presentation only: a
//! pick selects and centres the system, as a map click would, but plots no
//! course.
use crate::theme::icons::Icon;
use crate::theme::{CYAN, PANEL};
use egui::{Id, Key, Rect, RichText, TextEdit, Ui};
use game_core::{StarSystem, SystemId};

/// How many results the box lists at most.
const MAX_RESULTS: usize = 8;

/// One match: the system to show and how the result reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Found {
    pub system: SystemId,
    pub label: String,
}

/// Systems and bodies whose names contain `query` (ignoring case). Names that
/// start with it come first, then systems before bodies, then by name.
pub(super) fn find(systems: &[StarSystem], query: &str) -> Vec<Found> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    // (starts with the query, is a body, name, result)
    let mut found: Vec<(bool, bool, String, Found)> = Vec::new();
    for system in systems {
        let name = system.name.to_lowercase();
        if name.contains(&query) {
            let label = format!("★ {}", system.name);
            found.push((
                !name.starts_with(&query),
                false,
                name,
                Found {
                    system: system.id,
                    label,
                },
            ));
        }
        for body in &system.bodies {
            let body_name = body.name.to_lowercase();
            if body_name.contains(&query) {
                let label = format!(
                    "{} · {} in {}",
                    body.name,
                    kind_name(body.kind),
                    system.name
                );
                found.push((
                    !body_name.starts_with(&query),
                    true,
                    body_name,
                    Found {
                        system: system.id,
                        label,
                    },
                ));
            }
        }
    }
    found.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
    found
        .into_iter()
        .map(|(.., hit)| hit)
        .take(MAX_RESULTS)
        .collect()
}

/// "RockyPlanet" -> "rocky planet".
fn kind_name(kind: game_core::BodyKind) -> String {
    let debug = format!("{kind:?}");
    let mut words = String::new();
    for (index, letter) in debug.chars().enumerate() {
        if letter.is_uppercase() && index > 0 {
            words.push(' ');
        }
        words.extend(letter.to_lowercase());
    }
    words
}

/// A Search button at the galaxy map's top-left corner; tapping it (or
/// Ctrl/Cmd+F) opens the search box with its results below while there is a
/// query. It is drawn in a child of the map's `ui`, after the map, so it sits
/// on top and takes its own clicks. Escape, a pick or the button closes it.
/// Returns a picked system.
pub(super) fn search_box(ui: &mut Ui, map: Rect, systems: &[StarSystem]) -> Option<SystemId> {
    let query_id = Id::new("galaxy-search-query");
    let open_id = Id::new("galaxy-search-open");
    let mut query = ui
        .data(|data| data.get_temp::<String>(query_id))
        .unwrap_or_default();
    let mut open = ui
        .data(|data| data.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let shortcut = ui.input(|i| i.modifiers.command && i.key_pressed(Key::F));
    let mut focus = shortcut && !open;
    open |= shortcut;
    let mut picked = None;
    let corner = Rect::from_min_size(map.min + egui::vec2(8.0, 8.0), egui::vec2(296.0, 400.0));
    let mut overlay = ui.new_child(egui::UiBuilder::new().max_rect(corner.intersect(map)));
    let button = crate::theme::icons::labeled_toggle(&mut overlay, Icon::Focus, "Search", open)
        .on_hover_text("Find a star or planet by name (Ctrl+F)");
    if button.clicked() {
        open = !open;
        focus = open;
    }
    if open {
        egui::Frame::new()
            .fill(PANEL.gamma_multiply(0.92))
            .corner_radius(6.0)
            .inner_margin(6.0)
            .show(&mut overlay, |ui| {
                let edit = ui.add(
                    TextEdit::singleline(&mut query)
                        .hint_text("Star or planet name")
                        .desired_width(260.0),
                );
                if focus {
                    edit.request_focus();
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    open = false;
                }
                let found = find(systems, &query);
                if !query.trim().is_empty() && found.is_empty() {
                    ui.label(RichText::new("No star or planet by that name").color(CYAN));
                }
                for hit in found {
                    if ui.selectable_label(false, hit.label).clicked() {
                        picked = Some(hit.system);
                    }
                }
            });
    }
    if picked.is_some() || !open {
        query.clear();
        open = false;
    }
    ui.data_mut(|data| {
        data.insert_temp(query_id, query);
        data.insert_temp(open_id, open);
    });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_systems_and_bodies_by_name_ignoring_case() {
        let systems = game_core::demo_galaxy();
        let system = &systems[1];
        let body = &system.bodies[0];
        let by_system = find(&systems, &system.name.to_uppercase());
        assert_eq!(by_system[0].system, system.id, "{by_system:?}");
        assert!(by_system[0].label.contains(&system.name));
        let by_body = find(&systems, &body.name.to_lowercase());
        assert!(
            by_body
                .iter()
                .any(|hit| hit.system == system.id && hit.label.starts_with(&body.name)),
            "{by_body:?}"
        );
        assert!(find(&systems, "   ").is_empty());
        assert!(find(&systems, "no such place at all").is_empty());
    }

    #[test]
    fn lists_at_most_eight_results() {
        let systems = game_core::demo_galaxy();
        // Every name contains a vowel, so a single vowel matches many.
        assert!(find(&systems, "a").len() <= MAX_RESULTS);
    }
}
