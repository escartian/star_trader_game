//! Embedded Long Haul blocks and a sample aligned, weighted sequencer.
//! The manifest's `frames` excludes the nine second reverb tail in each Ogg.
//! All files are stereo 48 kHz; a transition occurs before sample `frames * 2`.

use rodio::{Decoder, Source};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

type BlockDecoder = Decoder<Cursor<&'static [u8]>>;

const MANIFEST: &str = include_str!("../../../../assets/music/long-haul/manifest.json");
const AUDIO: &[(&str, &[u8])] = &[
    (
        "drift_a.ogg",
        include_bytes!("../../../../assets/music/long-haul/drift_a.ogg"),
    ),
    (
        "drift_b.ogg",
        include_bytes!("../../../../assets/music/long-haul/drift_b.ogg"),
    ),
    (
        "space.ogg",
        include_bytes!("../../../../assets/music/long-haul/space.ogg"),
    ),
    (
        "rise_70.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_70.ogg"),
    ),
    (
        "rise_80.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_80.ogg"),
    ),
    (
        "depart_a.ogg",
        include_bytes!("../../../../assets/music/long-haul/depart_a.ogg"),
    ),
    (
        "depart_b.ogg",
        include_bytes!("../../../../assets/music/long-haul/depart_b.ogg"),
    ),
    (
        "rise_90.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_90.ogg"),
    ),
    (
        "rise_100.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_100.ogg"),
    ),
    (
        "cruise_a.ogg",
        include_bytes!("../../../../assets/music/long-haul/cruise_a.ogg"),
    ),
    (
        "cruise_b.ogg",
        include_bytes!("../../../../assets/music/long-haul/cruise_b.ogg"),
    ),
    (
        "solo.ogg",
        include_bytes!("../../../../assets/music/long-haul/solo.ogg"),
    ),
    (
        "rise_110.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_110.ogg"),
    ),
    (
        "rise_120.ogg",
        include_bytes!("../../../../assets/music/long-haul/rise_120.ogg"),
    ),
    (
        "arrive_a.ogg",
        include_bytes!("../../../../assets/music/long-haul/arrive_a.ogg"),
    ),
    (
        "arrive_b.ogg",
        include_bytes!("../../../../assets/music/long-haul/arrive_b.ogg"),
    ),
    (
        "fanfare.ogg",
        include_bytes!("../../../../assets/music/long-haul/fanfare.ogg"),
    ),
    (
        "fall_110.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_110.ogg"),
    ),
    (
        "fall_100.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_100.ogg"),
    ),
    (
        "fall_90.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_90.ogg"),
    ),
    (
        "fall_80.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_80.ogg"),
    ),
    (
        "wind.ogg",
        include_bytes!("../../../../assets/music/long-haul/wind.ogg"),
    ),
    (
        "fall_70.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_70.ogg"),
    ),
    (
        "fall_60.ogg",
        include_bytes!("../../../../assets/music/long-haul/fall_60.ogg"),
    ),
];

const NO_SELECTION: usize = usize::MAX;
/// Two seconds for an explicitly chosen part to replace the current one.
const SELECTION_FADE_SAMPLES: usize = 48_000 * 2 * 2;

/// Stable names for the UI; the audio thread publishes an index into `AUDIO`.
pub(super) fn part_name(index: usize) -> Option<&'static str> {
    AUDIO
        .get(index)
        .and_then(|(file, _)| file.strip_suffix(".ogg"))
}

pub(super) fn part_index(name: &str) -> Option<usize> {
    AUDIO
        .iter()
        .position(|(file, _)| file.strip_suffix(".ogg") == Some(name))
}

fn audio_index(file: &str) -> usize {
    AUDIO
        .iter()
        .position(|(name, _)| *name == file)
        .expect("validated manifest references embedded audio")
}

#[derive(Deserialize)]
struct Manifest {
    sample_rate: u32,
    start: String,
    blocks: BTreeMap<String, Block>,
}

#[derive(Deserialize)]
struct Block {
    file: String,
    frames: usize,
    /// Leveled active RMS before the shared peak gain; only ratios are used.
    rms: f32,
    from_bpm: u32,
    to_bpm: u32,
    next: BTreeMap<String, u32>,
}

impl Manifest {
    fn embedded() -> Result<Self, String> {
        let graph: Self = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
        graph.validate()?;
        Ok(graph)
    }

    fn validate(&self) -> Result<(), String> {
        if self.sample_rate != 48_000 || !self.blocks.contains_key(&self.start) {
            return Err("invalid music sample rate or start block".into());
        }
        if self.blocks.len() != AUDIO.len() {
            return Err("music block count does not match embedded files".into());
        }
        for (name, block) in &self.blocks {
            if block.frames == 0
                || !block.rms.is_finite()
                || block.rms <= 0.0
                || block.next.is_empty()
                || block.from_bpm == 0
                || block.to_bpm == 0
            {
                return Err(format!("invalid music block {name}"));
            }
            if AUDIO.iter().all(|(file, _)| *file != block.file) {
                return Err(format!("missing embedded audio for {name}"));
            }
            for (next, weight) in &block.next {
                let Some(target) = self.blocks.get(next) else {
                    return Err(format!("unknown successor {next} for {name}"));
                };
                if *weight == 0 || block.to_bpm != target.from_bpm {
                    return Err(format!("invalid tempo transition {name} to {next}"));
                }
            }
        }
        Ok(())
    }

    fn choose_next<'a>(&'a self, name: &str) -> &'a str {
        let next = &self.blocks[name].next;
        let total: u32 = next.values().sum();
        let mut draw = fastrand::u32(0..total);
        for (name, weight) in next {
            if draw < *weight {
                return name;
            }
            draw -= weight;
        }
        unreachable!("positive transition weights sum to total")
    }
}

fn decoder(file: &str) -> Result<BlockDecoder, String> {
    let bytes = AUDIO
        .iter()
        .find(|(name, _)| *name == file)
        .ok_or_else(|| format!("missing embedded music file {file}"))?
        .1;
    Decoder::new(Cursor::new(bytes)).map_err(|error| error.to_string())
}

/// Counts interleaved samples, so both stereo channels cross at one frame edge.
struct JoinClock {
    samples_left: usize,
}

/// Raises a louder incoming part over its first bar. Quieter parts keep their
/// own level; the old part's tail naturally fades underneath them.
struct EntryRamp {
    start_gain: f32,
    total_samples: usize,
    samples_left: usize,
}

impl EntryRamp {
    fn flat() -> Self {
        Self {
            start_gain: 1.0,
            total_samples: 0,
            samples_left: 0,
        }
    }

    fn between(previous: &Block, next: &Block, sample_rate: u32) -> Self {
        let start_gain = (previous.rms / next.rms).clamp(0.35, 1.0);
        if start_gain == 1.0 {
            return Self::flat();
        }
        // Four beats at the incoming tempo; two samples per stereo frame.
        let frames = (f64::from(sample_rate) * 240.0 / f64::from(next.from_bpm)).round() as usize;
        Self {
            start_gain,
            total_samples: frames * 2,
            samples_left: frames * 2,
        }
    }

    fn gain(&mut self) -> f32 {
        if self.samples_left == 0 {
            return 1.0;
        }
        // Both channels of a frame share one gain, preserving the stereo image.
        let elapsed_frames = (self.total_samples - self.samples_left) / 2;
        let t = elapsed_frames as f32 / (self.total_samples / 2) as f32;
        let eased = t * t * (3.0 - 2.0 * t);
        self.samples_left -= 1;
        self.start_gain + (1.0 - self.start_gain) * eased
    }
}

/// A manual Songbook jump may cross tempos. Fade the previous music out while
/// the chosen part fades in, then discard its old decoder and reverb tail.
struct SelectionFade {
    samples_left: usize,
    older_tail: Option<BlockDecoder>,
}

impl SelectionFade {
    fn blend(&mut self) -> f32 {
        let elapsed_frames = (SELECTION_FADE_SAMPLES - self.samples_left) / 2;
        let t = elapsed_frames as f32 / (SELECTION_FADE_SAMPLES / 2) as f32;
        self.samples_left -= 1;
        t * t * (3.0 - 2.0 * t)
    }
}

impl JoinClock {
    fn new(frames: usize) -> Self {
        Self {
            samples_left: frames * 2,
        }
    }

    fn boundary(&self) -> bool {
        self.samples_left == 0
    }

    fn tick(&mut self) {
        self.samples_left -= 1;
    }
}

pub struct LongHaul {
    graph: Manifest,
    current_name: String,
    current: BlockDecoder,
    tail: Option<BlockDecoder>,
    clock: JoinClock,
    ramp: EntryRamp,
    selection_fade: Option<SelectionFade>,
    now_playing: Arc<AtomicUsize>,
    selected_part: Arc<AtomicUsize>,
}

impl LongHaul {
    pub fn new() -> Result<Self, String> {
        let graph = Manifest::embedded()?;
        let name = graph.start.clone();
        let block = &graph.blocks[&name];
        let current = decoder(&block.file)?;
        let clock = JoinClock::new(block.frames);
        let now_playing = Arc::new(AtomicUsize::new(audio_index(&block.file)));
        Ok(Self {
            graph,
            current_name: name,
            current,
            tail: None,
            clock,
            ramp: EntryRamp::flat(),
            selection_fade: None,
            now_playing,
            selected_part: Arc::new(AtomicUsize::new(NO_SELECTION)),
        })
    }

    pub(super) fn part_handle(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.now_playing)
    }

    pub(super) fn selection_handle(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.selected_part)
    }

    fn jump_to(&mut self, index: usize) {
        let Some(name) = part_name(index) else { return };
        let Some(block) = self.graph.blocks.get(name) else {
            return;
        };
        let Ok(chosen) = decoder(&block.file) else {
            return;
        };
        let old_current = std::mem::replace(&mut self.current, chosen);
        self.selection_fade = Some(SelectionFade {
            samples_left: SELECTION_FADE_SAMPLES,
            older_tail: self.tail.take(),
        });
        self.tail = Some(old_current);
        self.current_name = name.to_owned();
        self.clock = JoinClock::new(block.frames);
        self.ramp = EntryRamp::flat();
        self.now_playing.store(index, Ordering::Release);
    }

    fn advance(&mut self) -> Result<(), String> {
        let next = self.graph.choose_next(&self.current_name).to_owned();
        let block = &self.graph.blocks[&next];
        self.ramp = EntryRamp::between(&self.graph.blocks[&self.current_name], block, 48_000);
        let decoder = decoder(&block.file)?;
        self.tail = Some(std::mem::replace(&mut self.current, decoder));
        self.current_name = next;
        self.clock = JoinClock::new(block.frames);
        self.now_playing
            .store(audio_index(&block.file), Ordering::Release);
        Ok(())
    }
}

impl Iterator for LongHaul {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        // Consume requests only before the left channel of a stereo frame.
        if self.clock.samples_left.is_multiple_of(2) {
            let chosen = self.selected_part.swap(NO_SELECTION, Ordering::AcqRel);
            if chosen != NO_SELECTION {
                self.jump_to(chosen);
            }
        }
        if self.clock.boundary() && self.advance().is_err() {
            return None;
        }
        self.clock.tick();
        let dry = self.current.next().unwrap_or(0.0);
        let wet = self.tail.as_mut().and_then(Iterator::next).unwrap_or(0.0);
        if let Some(fade) = &mut self.selection_fade {
            let older = fade
                .older_tail
                .as_mut()
                .and_then(Iterator::next)
                .unwrap_or(0.0);
            let blend = fade.blend();
            let mixed = dry * blend + (wet + older) * (1.0 - blend);
            if fade.samples_left == 0 {
                self.selection_fade = None;
                self.tail = None;
            }
            Some(mixed)
        } else {
            Some(dry * self.ramp.gain() + wet)
        }
    }
}

impl Source for LongHaul {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        48_000
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests;
