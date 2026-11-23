//! Small music reference card on the main menu. Playback remains owned by audio.
use crate::theme::{CYAN, GREEN, metal};
use egui::RichText;

struct Part {
    id: &'static str,
    section: &'static str,
    title: &'static str,
    tempo: &'static str,
    detail: &'static str,
}

const PARTS: &[Part] = &[
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
        id: "space",
        section: "Drift",
        title: "Space",
        tempo: "60 BPM",
        detail: "Soft bass, ensemble and radio ticks",
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
        detail: "16 bars of bends, strummed chords and three short runs",
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
];

pub(super) fn title(id: &str) -> Option<&'static str> {
    PARTS
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.title)
}

pub(super) fn show(
    ctx: &egui::Context,
    open: &mut bool,
    current: Option<&str>,
) -> Option<&'static str> {
    if !*open {
        return None;
    }
    if current.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
    let mut chosen = None;
    metal::window(ctx, "Songbook · Long Haul", open, 440.0, |ui| {
        let panel_width = (ui.available_width() - 24.0).max(0.0);
        metal::frame(ui, 235, |ui| {
            ui.set_width(panel_width);
            ui.label(RichText::new("NOW PLAYING").color(CYAN).strong());
            if let Some(part) = current.and_then(|id| PARTS.iter().find(|part| part.id == id)) {
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
        });
        ui.add_space(8.0);
        ui.label("Select a part to play it now. The track then continues through its possible next parts.");
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
    });
    chosen
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
