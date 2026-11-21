//! Music playback. The map decides which track should play and how loud; this
//! module owns the audio device. Desktop builds play through rodio; the browser
//! build (and any machine without an output device) stays silent.
//!
//! Volumes are slider percentages (0-100). Loudness is roughly logarithmic, so a
//! slider fraction `f` maps to the linear amplitude gain `f²`: half the slider is
//! about -12 dB rather than -6 dB, which sounds closer to "half as loud".

/// The original main menu theme, exported from
/// `assets/source/music/main-theme.strudel.js` by `tools/music/render.mjs`. It
/// is a seamless 133.6 s loop of 48 bars whose tempo eases 60-120-60 BPM.
#[cfg(not(target_arch = "wasm32"))]
const MAIN_THEME: &[u8] = include_bytes!("../../../assets/music/main-theme.ogg");

/// Seconds for music to fade in after starting and out before stopping.
const FADE_SECONDS: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    MainTheme,
}

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
}

#[cfg(not(target_arch = "wasm32"))]
mod output {
    use super::{MAIN_THEME, Track};
    use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
    use std::io::Cursor;

    pub struct Output {
        // Field order matters: the sink must drop before its stream.
        sink: Option<Sink>,
        stream: OutputStream,
    }

    impl Output {
        pub fn open() -> Result<Self, String> {
            let mut stream =
                OutputStreamBuilder::open_default_stream().map_err(|error| error.to_string())?;
            // rodio otherwise prints a notice to stderr when the game exits.
            stream.log_on_drop(false);
            Ok(Self { sink: None, stream })
        }

        pub fn play(&mut self, track: Option<Track>) {
            self.sink = None;
            let Some(Track::MainTheme) = track else {
                return;
            };
            match Decoder::new_looped(Cursor::new(MAIN_THEME)) {
                Ok(source) => {
                    let sink = Sink::connect_new(self.stream.mixer());
                    sink.set_volume(0.0);
                    sink.append(source);
                    self.sink = Some(sink);
                }
                Err(error) => eprintln!("Main theme could not be decoded: {error}"),
            }
        }

        pub fn set_gain(&self, gain: f32) {
            if let Some(sink) = &self.sink {
                sink.set_volume(gain);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod output {
    use super::Track;

    /// The browser build has no music yet (see ROADMAP.md).
    pub struct Output;

    impl Output {
        pub fn open() -> Result<Self, String> {
            Err("the browser build has no audio yet".into())
        }

        pub fn play(&mut self, _: Option<Track>) {}

        pub fn set_gain(&self, _: f32) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_embedded_theme_decodes_as_the_48_bar_tempo_arch() {
        use rodio::Source;
        let theme = rodio::Decoder::new(std::io::Cursor::new(MAIN_THEME)).unwrap();
        assert_eq!((theme.channels(), theme.sample_rate()), (2, 48_000));
        let samples = theme.count();
        let seconds = samples as f64 / 2.0 / 48_000.0;
        // Six 8-bar sections at 60, 80, 100, 120, 100 and 80 BPM: each holds
        // for 4 bars, then eases toward the next over 4 (the source's tempoMap).
        let tempos = [60.0, 80.0, 100.0, 120.0, 100.0, 80.0];
        let expected: f64 = (0..6)
            .flat_map(|section| {
                let (bpm, next) = (tempos[section], tempos[(section + 1) % 6]);
                [0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0].map(|step| bpm + (next - bpm) * step / 5.0)
            })
            .map(|bpm: f64| 4.0 * 60.0 / bpm)
            .sum();
        assert!((seconds - expected).abs() < 0.05, "{seconds} s");
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
