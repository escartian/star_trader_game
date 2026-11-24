//! Music playback. The map decides which track should play and how loud; this
//! module owns the audio device. Desktop builds play through rodio; the browser
//! build (and any machine without an output device) stays silent.
//!
//! Volumes are slider percentages (0-100). Loudness is roughly logarithmic, so a
//! slider fraction `f` maps to the linear amplitude gain `f²`: half the slider is
//! about -12 dB rather than -6 dB, which sounds closer to "half as loud".

#[cfg(not(target_arch = "wasm32"))]
mod music_graph;

/// Seconds for music to fade in after starting and out before stopping.
const FADE_SECONDS: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    MainTheme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Click,
    /// Picking a star, planet or fleet on a map: the click, pitched up.
    Select,
    Laser,
    Shield,
    Hull,
    Explosion,
}

mod cues;
pub use cues::{mark_click, mark_select, quiet_clicks, take_cue};

/// Linear amplitude gain for a master and a music slider, each 0-100 percent.
pub fn music_gain(master_percent: u8, music_percent: u8, muted: bool) -> f32 {
    if muted {
        return 0.0;
    }
    let fraction = |percent: u8| f32::from(percent.min(100)) / 100.0;
    (fraction(master_percent) * fraction(music_percent)).powi(2)
}

pub struct Music {
    output: Option<output::Output>,
    /// The track wanted now; the playing one fades out first if it differs.
    wanted: Option<Track>,
    playing: Option<Track>,
    /// 0 (silent) to 1 (full target gain); ramps by `FADE_SECONDS`.
    fade: f32,
    gain: f32,
    effects_gain: f32,
    /// UI clock in seconds at the last update, for fade timing.
    last_time: Option<f64>,
}

impl Music {
    /// No device: every call is bookkeeping only (tests, captures, the browser).
    pub fn silent() -> Self {
        Self {
            output: None,
            wanted: None,
            playing: None,
            fade: 0.0,
            gain: 0.0,
            effects_gain: 0.0,
            last_time: None,
        }
    }

    /// Opens the default output device; falls back to silence with a reason.
    pub fn open() -> (Self, Option<String>) {
        let mut music = Self::silent();
        match output::Output::open() {
            Ok(output) => music.output = Some(output),
            Err(error) => return (music, Some(format!("Music is off: {error}"))),
        }
        (music, None)
    }

    #[cfg(test)]
    pub fn playing(&self) -> Option<Track> {
        self.playing
    }

    /// Steps fades toward `wanted` at `gain`. `time` is the UI clock in
    /// seconds. Returns true while a fade is still running (keep repainting).
    pub fn update(&mut self, wanted: Option<Track>, gain: f32, time: f64) -> bool {
        let step = self
            .last_time
            .map_or(0.0, |last| ((time - last).max(0.0) as f32) / FADE_SECONDS);
        self.last_time = Some(time);
        self.wanted = wanted;
        self.gain = gain.clamp(0.0, 1.0);
        if self.playing != self.wanted {
            // Fade the old track out (or finish instantly if nothing plays).
            self.fade = if self.playing.is_some() {
                (self.fade - step).max(0.0)
            } else {
                0.0
            };
            if self.fade == 0.0 {
                self.playing = self.wanted;
                if let Some(output) = &mut self.output {
                    output.play(self.playing);
                }
            }
        } else if self.playing.is_some() {
            self.fade = (self.fade + step).min(1.0);
        }
        if let Some(output) = &self.output {
            output.set_gain(self.gain * self.fade);
        }
        let settled_level = if self.playing.is_some() { 1.0 } else { 0.0 };
        self.playing != self.wanted || self.fade != settled_level
    }

    pub fn set_effects_gain(&mut self, gain: f32) {
        self.effects_gain = gain.clamp(0.0, 1.0) * 0.4;
        if let Some(output) = &mut self.output {
            output.set_effects_gain(self.effects_gain);
        }
    }

    pub fn play_effect(&mut self, effect: Effect) {
        if self.effects_gain > 0.0
            && let Some(output) = &mut self.output
        {
            output.play_effect(effect, self.effects_gain);
        }
    }

    /// The block the audio thread is playing now, if desktop music is active.
    pub fn current_block(&self) -> Option<&'static str> {
        self.output.as_ref().and_then(output::Output::current_block)
    }

    /// The block already chosen to follow the current one.
    pub fn next_block(&self) -> Option<&'static str> {
        self.output.as_ref().and_then(output::Output::next_block)
    }

    /// Requests an immediate, short crossfade to a Songbook part.
    pub fn select_part(&self, name: &str) -> bool {
        self.output
            .as_ref()
            .is_some_and(|output| output.select_part(name))
    }

    /// Queues a Songbook part to play when the current block ends.
    pub fn queue_part(&self, name: &str) -> bool {
        self.output
            .as_ref()
            .is_some_and(|output| output.queue_part(name))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod output {
    use super::{
        Effect, Track,
        music_graph::{self, MusicGraph},
    };
    use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
    use std::io::Cursor;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    pub(super) fn effect_bytes(effect: Effect) -> &'static [u8] {
        match effect {
            Effect::Click | Effect::Select => {
                include_bytes!("../../../assets/effects/sounds/click3.ogg")
            }
            Effect::Laser => include_bytes!("../../../assets/effects/sounds/laserLarge_000.ogg"),
            Effect::Shield => include_bytes!("../../../assets/effects/sounds/forceField_000.ogg"),
            Effect::Hull => include_bytes!("../../../assets/effects/sounds/impactMetal_001.ogg"),
            Effect::Explosion => {
                include_bytes!("../../../assets/effects/sounds/explosionCrunch_000.ogg")
            }
        }
    }

    pub struct Output {
        // Field order matters: the sink must drop before its stream.
        sink: Option<Sink>,
        part: Option<Arc<AtomicUsize>>,
        up_next: Option<Arc<AtomicUsize>>,
        selection: Option<Arc<AtomicUsize>>,
        queue: Option<Arc<AtomicUsize>>,
        effects: Vec<Sink>,
        stream: OutputStream,
    }

    impl Output {
        pub fn open() -> Result<Self, String> {
            let mut stream =
                OutputStreamBuilder::open_default_stream().map_err(|error| error.to_string())?;
            // rodio otherwise prints a notice to stderr when the game exits.
            stream.log_on_drop(false);
            Ok(Self {
                sink: None,
                part: None,
                up_next: None,
                selection: None,
                queue: None,
                effects: Vec::new(),
                stream,
            })
        }

        pub fn play(&mut self, track: Option<Track>) {
            self.sink = None;
            self.part = None;
            self.up_next = None;
            self.selection = None;
            self.queue = None;
            let Some(Track::MainTheme) = track else {
                return;
            };
            match MusicGraph::new() {
                Ok(source) => {
                    self.part = Some(source.part_handle());
                    self.up_next = Some(source.up_next_handle());
                    self.selection = Some(source.selection_handle());
                    self.queue = Some(source.queue_handle());
                    let sink = Sink::connect_new(self.stream.mixer());
                    sink.set_volume(0.0);
                    sink.append(source);
                    self.sink = Some(sink);
                }
                Err(error) => eprintln!("The music could not be started: {error}"),
            }
        }

        pub fn set_gain(&self, gain: f32) {
            if let Some(sink) = &self.sink {
                sink.set_volume(gain);
            }
        }

        pub fn current_block(&self) -> Option<&'static str> {
            let index = self.part.as_ref()?.load(Ordering::Acquire);
            music_graph::part_name(index)
        }

        pub fn next_block(&self) -> Option<&'static str> {
            let index = self.up_next.as_ref()?.load(Ordering::Acquire);
            music_graph::part_name(index)
        }

        pub fn select_part(&self, name: &str) -> bool {
            let Some(index) = music_graph::part_index(name) else {
                return false;
            };
            let Some(selection) = &self.selection else {
                return false;
            };
            selection.store(index, Ordering::Release);
            true
        }

        pub fn queue_part(&self, name: &str) -> bool {
            let (Some(index), Some(queue)) = (music_graph::part_index(name), &self.queue) else {
                return false;
            };
            queue.store(index, Ordering::Release);
            true
        }

        pub fn set_effects_gain(&mut self, gain: f32) {
            self.effects.retain(|sink| !sink.empty());
            for sink in &self.effects {
                sink.set_volume(gain);
            }
        }

        pub fn play_effect(&mut self, effect: Effect, gain: f32) {
            match Decoder::new(Cursor::new(effect_bytes(effect))) {
                Ok(source) => {
                    let sink = Sink::connect_new(self.stream.mixer());
                    sink.set_volume(gain);
                    // A selection plays the click a fifth higher, so it reads
                    // as a distinct, brighter blip.
                    if effect == Effect::Select {
                        sink.set_speed(1.5);
                    }
                    sink.append(source);
                    self.effects.push(sink);
                }
                Err(error) => eprintln!("Effect {effect:?} could not be played: {error}"),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod output {
    use super::{Effect, Track};

    /// The browser build has no music yet (see ROADMAP.md).
    pub struct Output;

    impl Output {
        pub fn open() -> Result<Self, String> {
            Err("the browser build has no audio yet".into())
        }

        pub fn play(&mut self, _: Option<Track>) {}

        pub fn set_gain(&self, _: f32) {}

        pub fn current_block(&self) -> Option<&'static str> {
            None
        }

        pub fn next_block(&self) -> Option<&'static str> {
            None
        }

        pub fn select_part(&self, _: &str) -> bool {
            false
        }

        pub fn queue_part(&self, _: &str) -> bool {
            false
        }

        pub fn set_effects_gain(&mut self, _: f32) {}

        pub fn play_effect(&mut self, _: Effect, _: f32) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn every_embedded_effect_decodes() {
        for effect in [
            Effect::Click,
            Effect::Select,
            Effect::Laser,
            Effect::Shield,
            Effect::Hull,
            Effect::Explosion,
        ] {
            let source = rodio::Decoder::new(std::io::Cursor::new(output::effect_bytes(effect)));
            assert!(source.is_ok(), "{effect:?}");
        }
    }

    #[test]
    fn slider_percentages_map_to_squared_gain() {
        assert_eq!(music_gain(100, 100, false), 1.0);
        assert_eq!(music_gain(100, 50, false), 0.25);
        assert_eq!(music_gain(0, 100, false), 0.0);
        assert_eq!(music_gain(100, 100, true), 0.0);
        assert_eq!(music_gain(255, 100, false), 1.0, "percent is capped");
    }

    #[test]
    fn a_new_track_fades_in_and_stopping_fades_out_first() {
        let mut music = Music::silent();
        assert!(music.update(Some(Track::MainTheme), 1.0, 0.0));
        assert_eq!(music.playing(), Some(Track::MainTheme));
        assert!(
            music.update(Some(Track::MainTheme), 1.0, 0.6),
            "still fading in"
        );
        assert!(!music.update(Some(Track::MainTheme), 1.0, 2.0), "fully in");
        assert!(music.update(None, 1.0, 2.6));
        assert_eq!(music.playing(), Some(Track::MainTheme), "fading out");
        assert!(!music.update(None, 1.0, 4.0));
        assert_eq!(music.playing(), None);
    }
}
