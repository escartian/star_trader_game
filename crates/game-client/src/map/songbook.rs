//! The main menu's music card: Long Haul's parts on a transition map
//! (`songbook_graph`). Playback remains owned by audio.
use super::songbook_graph::{self, Highlights};
use crate::theme::{CYAN, GREEN, metal};
use egui::RichText;

pub(super) struct Part {
    pub id: &'static str,
    pub section: &'static str,
    pub title: &'static str,
    pub tempo: &'static str,
    pub detail: &'static str,
}

pub(super) const PARTS: &[Part] = &[
    Part {
        id: "drift_a",
        section: "Drift",
        title: "Drift A",
        tempo: "60 BPM",
        detail: "Keys, ensemble, sequence and bells",
    },
    Part {
        id: "drift_b",
        section: "Drift",
        title: "Drift B",
        tempo: "60 BPM",
        detail: "Bass and first guitar phrases",
    },
    Part {
        id: "rise_70",
        section: "Ascent",
        title: "Rise to 70",
        tempo: "60–70 BPM",
        detail: "Sequence and shaker enter",
    },
    Part {
        id: "rise_80",
        section: "Ascent",
        title: "Rise to 80",
        tempo: "70–80 BPM",
        detail: "Arpeggio and hats grow",
    },
    Part {
        id: "depart_a",
        section: "Departure",
        title: "Depart A",
        tempo: "80 BPM",
        detail: "Guitar, organ and horn enter",
    },
    Part {
        id: "depart_b",
        section: "Departure",
        title: "Depart B",
        tempo: "80 BPM",
        detail: "Wider arpeggio and woodwind reply",
    },
    Part {
        id: "rise_90",
        section: "Departure",
        title: "Rise to 90",
        tempo: "80–90 BPM",
        detail: "Guitar and percussion build",
    },
    Part {
        id: "rise_100",
        section: "Departure",
        title: "Rise to 100",
        tempo: "90–100 BPM",
        detail: "Drums join the ascent",
    },
    Part {
        id: "cruise_a",
        section: "Cruise",
        title: "Cruise A",
        tempo: "100 BPM",
        detail: "Melody, guitar, organ and winds",
    },
    Part {
        id: "cruise_b",
        section: "Cruise",
        title: "Cruise B",
        tempo: "100 BPM",
        detail: "Alternate melody and guitar strums",
    },
    Part {
        id: "solo",
        section: "Cruise",
        title: "Guitar Solo",
        tempo: "100 BPM",
        detail: "16 bars of bends, strummed chords and three slurred runs",
    },
    Part {
        id: "rise_110",
        section: "Cruise",
        title: "Rise to 110",
        tempo: "100–110 BPM",
        detail: "The groove gathers speed",
    },
    Part {
        id: "rise_120",
        section: "Arrival",
        title: "Rise to 120",
        tempo: "110–120 BPM",
        detail: "Organ and horns build with the melody",
    },
    Part {
        id: "arrive_a",
        section: "Arrival",
        title: "Arrive A",
        tempo: "120 BPM",
        detail: "Organ, horns, woodwinds and drums",
    },
    Part {
        id: "arrive_b",
        section: "Arrival",
        title: "Arrive B",
        tempo: "120 BPM",
        detail: "Alternate melody with woodwind replies",
    },
    Part {
        id: "fanfare",
        section: "Arrival",
        title: "Orbital Fanfare",
        tempo: "120 BPM",
        detail: "Brass theme, fast strings and light percussion",
    },
    Part {
        id: "fall_110",
        section: "Return",
        title: "Fall to 110",
        tempo: "120–110 BPM",
        detail: "The organ and horns begin to ease",
    },
    Part {
        id: "fall_100",
        section: "Return",
        title: "Fall to 100",
        tempo: "110–100 BPM",
        detail: "Cruise theme returns",
    },
    Part {
        id: "fall_90",
        section: "Return",
        title: "Fall to 90",
        tempo: "100–90 BPM",
        detail: "Drums leave, guitar softens",
    },
    Part {
        id: "fall_80",
        section: "Return",
        title: "Fall to 80",
        tempo: "90–80 BPM",
        detail: "Bells return",
    },
    Part {
        id: "wind",
        section: "Return",
        title: "Wind Down",
        tempo: "80 BPM",
        detail: "Sequence and arpeggio linger",
    },
    Part {
        id: "fall_70",
        section: "Return",
        title: "Fall to 70",
        tempo: "80–70 BPM",
        detail: "Hats fade away",
    },
    Part {
        id: "fall_60",
        section: "Return",
        title: "Fall to 60",
        tempo: "70–60 BPM",
        detail: "Bells lead back to drift",
    },
    Part {
        id: "space",
        section: "Space",
        title: "Space",
        tempo: "60 BPM",
        detail: "Halo pad, beeps and boops, a nod to the hook",
    },
    Part {
        id: "space_rise_80",
        section: "Space",
        title: "Space Rise to 80",
        tempo: "60–80 BPM",
        detail: "The hook gives way to the riff and rhythm box",
    },
    Part {
        id: "space_80",
        section: "Space",
        title: "Space 80",
        tempo: "80 BPM",
        detail: "A French-house riff over a rhythm box and disco bass",
    },
    Part {
        id: "space_rise_100",
        section: "Space",
        title: "Space Rise to 100",
        tempo: "80–100 BPM",
        detail: "Drums, open hats and chopped chords join the riff",
    },
    Part {
        id: "space_100",
        section: "Space",
        title: "Space 100",
        tempo: "100 BPM",
        detail: "The riff with drums, open hats and chopped chords",
    },
    Part {
        id: "space_rise_120",
        section: "Space",
        title: "Space Rise to 120",
        tempo: "100–120 BPM",
        detail: "The riff hands over to the hook at double time",
    },
    Part {
        id: "space_120",
        section: "Space",
        title: "Space 120",
        tempo: "120 BPM",
        detail: "The hook at double time, with the full house groove",
    },
    Part {
        id: "space_fall_100",
        section: "Space",
        title: "Space Fall to 100",
        tempo: "120–100 BPM",
        detail: "The hook hands back to the riff",
    },
    Part {
        id: "space_fall_80",
        section: "Space",
        title: "Space Fall to 80",
        tempo: "100–80 BPM",
        detail: "Drums give way to the rhythm box",
    },
    Part {
        id: "space_fall_60",
        section: "Space",
        title: "Space Fall to 60",
        tempo: "80–60 BPM",
        detail: "The riff fades into the hook's quiet nod",
    },
];

pub(super) fn title(id: &str) -> Option<&'static str> {
    PARTS
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.title)
}

fn part(id: &str) -> Option<&'static Part> {
    PARTS.iter().find(|part| part.id == id)
}

/// The static ID of a Long Haul part, if `name` is one.
#[cfg(any(test, feature = "dev-tools"))]
pub(super) fn id(name: &str) -> Option<&'static str> {
    part(name).map(|part| part.id)
}

/// The Songbook's two views. Transient, like the window itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum View {
    /// Every part in section order, as a list to pick from.
    #[default]
    List,
    /// The transition map (`songbook_graph`).
    Map,
}

/// Shows the Songbook if `open`; returns a part the viewer chose to play.
pub(super) fn show(
    ctx: &egui::Context,
    open: &mut bool,
    view: &mut View,
    current: Option<&'static str>,
    next: Option<&'static str>,
) -> Option<&'static str> {
    if !*open {
        return None;
    }
    // The map is for desktop screens; phones (full-screen sheets) list parts.
    let screen = ctx.screen_rect();
    let wide = screen.width() >= metal::FULL_SCREEN_BELOW;
    let mut showing = if wide { *view } else { View::List };
    // The map's "up next" ring pulses, so it repaints more often.
    let repaint = if showing == View::Map { 50 } else { 250 };
    ctx.request_repaint_after(std::time::Duration::from_millis(repaint));
    // The map card fills most of the screen, up to 1200 points wide.
    let width = if showing == View::Map {
        (screen.width() - 80.0).clamp(800.0, 1200.0)
    } else {
        440.0
    };
    let mut chosen = None;
    metal::window(ctx, "Songbook · Long Haul", open, width, |ui| {
        let content_top = ui.cursor().top();
        let panel_width = (ui.available_width() - 24.0).max(0.0);
        metal::frame(ui, 235, |ui| {
            ui.set_width(panel_width);
            if showing == View::Map {
                now_playing_line(ui, current, next);
            } else {
                now_playing(ui, current, next);
            }
        });
        if wide {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut showing, View::List, "List");
                ui.selectable_value(&mut showing, View::Map, "Map");
            });
            *view = showing;
        }
        ui.add_space(4.0);
        chosen = match showing {
            View::List => list(ui, current),
            View::Map => map(ui, current, next, width, content_top),
        };
    });
    chosen
}

/// The map's height so the card needs no scrolling: `metal::window` shows a
/// card's content in a scroll area 70% of the screen tall. Leaves room for
/// the content above the map (since `content_top`) and `below` points under it.
fn map_height(ui: &egui::Ui, content_top: f32, below: f32) -> f32 {
    let budget = ui.ctx().screen_rect().height() * 0.7;
    let above = ui.cursor().top() - content_top;
    (budget - above - below).max(songbook_graph::MIN_CANVAS.y)
}

fn now_playing(ui: &mut egui::Ui, current: Option<&str>, next: Option<&str>) {
    ui.label(RichText::new("NOW PLAYING").color(CYAN).strong());
    if let Some(part) = current.and_then(part) {
        ui.label(
            RichText::new(format!("{} · {}", part.title, part.tempo))
                .color(GREEN)
                .size(20.0),
        );
        ui.label(part.detail);
        ui.small(format!("Part ID: {}", part.id));
    } else {
        ui.label("Music is stopped or unavailable.");
    }
    if let Some(part) = next.and_then(part) {
        ui.small(format!("Up next: {} · {}", part.title, part.tempo));
    }
}

/// A one-line version of `now_playing`, leaving the map more room.
fn now_playing_line(ui: &mut egui::Ui, current: Option<&str>, next: Option<&str>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("NOW PLAYING").color(CYAN).strong());
        match current.and_then(part) {
            Some(part) => ui.label(
                RichText::new(format!("{} · {}", part.title, part.tempo))
                    .color(GREEN)
                    .size(18.0),
            ),
            None => ui.label("Music is stopped or unavailable."),
        };
        if let Some(part) = next.and_then(part) {
            ui.add_space(12.0);
            ui.label(RichText::new("UP NEXT").color(CYAN).strong());
            ui.label(format!("{} · {}", part.title, part.tempo));
        }
    });
}

fn list(ui: &mut egui::Ui, current: Option<&str>) -> Option<&'static str> {
    ui.label(
        "Select a part to play it now. The track then continues through its possible next parts.",
    );
    let mut chosen = None;
    let mut section = "";
    for part in PARTS {
        if part.section != section {
            section = part.section;
            ui.add_space(8.0);
            ui.label(RichText::new(section.to_uppercase()).color(CYAN).strong());
        }
        let text = format!("{} · {} — {}", part.title, part.tempo, part.detail);
        let label = RichText::new(text).color(if current == Some(part.id) {
            GREEN
        } else {
            ui.visuals().text_color()
        });
        if ui
            .selectable_label(current == Some(part.id), label)
            .clicked()
        {
            chosen = Some(part.id);
        }
    }
    chosen
}

fn map(
    ui: &mut egui::Ui,
    current: Option<&'static str>,
    next: Option<&'static str>,
    width: f32,
    content_top: f32,
) -> Option<&'static str> {
    let time = ui.input(|input| input.time);
    // The map fills the card down to its two lines of explanation; on a
    // screen too small for its minimum size, the card's content scrolls.
    // Sized from the requested card width and the screen, never from the
    // space left in the window: egui shrinks that while a window is dragged
    // towards the screen edge, which would shrink the whole card.
    // The card's frame pads its content by 12 points on each side.
    let size = egui::vec2(width - 24.0, map_height(ui, content_top, 56.0));
    let highlights = Highlights {
        current,
        next,
        time,
    };
    let input = songbook_graph::show(ui, &highlights, size);
    // Describes the hovered part, else the current one, else the map.
    match input.hovered.or_else(|| current.and_then(part)) {
        Some(part) => ui.label(format!("{} · {} — {}", part.title, part.tempo, part.detail)),
        None => ui.label("Each circle is a part, grouped by section and placed by tempo."),
    };
    ui.small(
        "Hover a part to see where it can go next (green, with chances) and what leads to \
         it (cyan). Click a part to play it; the music then follows its arrows.",
    );
    input.chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_embedded_block_has_one_songbook_entry() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../assets/music/long-haul/manifest.json"
        ))
        .unwrap();
        let names: BTreeSet<_> = manifest["blocks"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let entries: BTreeSet<_> = PARTS.iter().map(|part| part.id).collect();
        assert_eq!(entries.len(), PARTS.len(), "part IDs must be unique");
        assert_eq!(entries, names);
    }
}
