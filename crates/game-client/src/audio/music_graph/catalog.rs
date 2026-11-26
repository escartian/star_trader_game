//! The music's blocks: their files (embedded on desktop, fetched in the
//! browser by `library`), names, the transition manifest, and the order the
//! browser fetches them in. The sequencer that plays them is `music_graph`.
use rodio::Decoder;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Cursor;

pub(super) type BlockDecoder = Decoder<Cursor<&'static [u8]>>;

const MANIFEST: &str = include_str!("../../../../../assets/music/manifest.json");
/// The block files in this order (`NAMES`), and on desktop their embedded
/// bytes (`EMBEDDED`, from `assets/music/<name>.ogg`).
macro_rules! blocks {
    ($($name:ident),* $(,)?) => {
        pub(super) const NAMES: &[&str] = &[$(concat!(stringify!($name), ".ogg")),*];
        #[cfg(not(target_arch = "wasm32"))]
        const EMBEDDED: &[&[u8]] = &[$(
            include_bytes!(concat!("../../../../../assets/music/", stringify!($name), ".ogg")),
        )*];
    };
}

blocks![
    drift_a,
    drift_b,
    space,
    rise_70,
    rise_80,
    space_80,
    depart_a,
    depart_b,
    rise_90,
    rise_100,
    cruise_a,
    cruise_b,
    space_100,
    solo,
    rise_110,
    rise_120,
    arrive_a,
    arrive_b,
    space_120,
    space_rise_80,
    space_rise_100,
    space_rise_120,
    space_fall_100,
    space_fall_80,
    space_fall_60,
    disco_60,
    disco_rise_80,
    disco_80,
    disco_rise_100,
    disco_100,
    disco_rise_120,
    disco_120,
    disco_fall_100,
    disco_fall_80,
    disco_fall_60,
    night_60,
    night_rise_80,
    night_80,
    night_rise_100,
    night_100,
    night_rise_120,
    night_120,
    night_fall_100,
    night_fall_80,
    night_fall_60,
    fanfare,
    fall_110,
    fall_100,
    fall_90,
    fall_80,
    wind,
    fall_70,
    fall_60,
];

/// Stable names for the UI; the audio thread publishes an index into `NAMES`.
pub(in crate::audio) fn part_name(index: usize) -> Option<&'static str> {
    NAMES.get(index).and_then(|file| file.strip_suffix(".ogg"))
}

pub(in crate::audio) fn part_index(name: &str) -> Option<usize> {
    NAMES
        .iter()
        .position(|file| file.strip_suffix(".ogg") == Some(name))
}

pub(super) fn audio_index(file: &str) -> usize {
    NAMES
        .iter()
        .position(|name| *name == file)
        .expect("validated manifest references known audio")
}

/// A block's Ogg bytes, once available (always, on desktop).
pub(super) fn block_bytes(index: usize) -> Option<&'static [u8]> {
    #[cfg(not(target_arch = "wasm32"))]
    return EMBEDDED.get(index).copied();
    #[cfg(target_arch = "wasm32")]
    return crate::audio::library::get(index);
}

/// A block's file name (`drift_a.ogg`) by `NAMES` index.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(in crate::audio) fn file_name(index: usize) -> Option<&'static str> {
    NAMES.get(index).copied()
}

fn loaded(file: &str) -> bool {
    block_bytes(audio_index(file)).is_some()
}

/// The block files in the order to fetch them (`NAMES` indices): slowest
/// tempo first, since the music opens slow (the start block, `drift_a` at
/// 60 bpm, leads), then by rising tempo; within one tempo, outward from the
/// start along the transitions, likelier successors first.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(in crate::audio) fn load_order() -> Result<Vec<usize>, String> {
    let graph = Manifest::embedded()?;
    let mut order = vec![graph.start.clone()];
    let mut index = 0;
    while index < order.len() {
        let mut next: Vec<(&String, &u32)> = graph.blocks[&order[index]].next.iter().collect();
        next.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (name, _) in next {
            if !order.contains(name) {
                order.push(name.clone());
            }
        }
        index += 1;
    }
    for name in graph.blocks.keys() {
        if !order.contains(name) {
            order.push(name.clone());
        }
    }
    // Slowest first: every block at the opening tempo, then rising by the
    // fastest tempo each reaches (the sort keeps the order above within one).
    order.sort_by_key(|name| {
        let block = &graph.blocks[name];
        block.from_bpm.max(block.to_bpm)
    });
    Ok(order
        .iter()
        .map(|name| audio_index(&graph.blocks[name].file))
        .collect())
}

#[derive(Deserialize)]
pub(super) struct Manifest {
    sample_rate: u32,
    pub(super) start: String,
    pub(super) blocks: BTreeMap<String, Block>,
}

#[derive(Deserialize)]
pub(super) struct Block {
    pub(super) file: String,
    pub(super) frames: usize,
    /// Leveled active RMS before the shared peak gain; only ratios are used.
    pub(super) rms: f32,
    pub(super) from_bpm: u32,
    pub(super) to_bpm: u32,
    pub(super) next: BTreeMap<String, u32>,
}

impl Manifest {
    pub(super) fn embedded() -> Result<Self, String> {
        let graph: Self = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
        graph.validate()?;
        Ok(graph)
    }

    fn validate(&self) -> Result<(), String> {
        if self.sample_rate != 48_000 || !self.blocks.contains_key(&self.start) {
            return Err("invalid music sample rate or start block".into());
        }
        if self.blocks.len() != NAMES.len() {
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
            if NAMES.iter().all(|file| *file != block.file) {
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

    /// A weighted draw among the successors already loaded. While none is,
    /// a loaded block at the same tempo plays instead, else this one repeats.
    pub(super) fn choose_next<'a>(&'a self, name: &'a str) -> &'a str {
        let next: Vec<(&String, &u32)> = self.blocks[name]
            .next
            .iter()
            .filter(|(next, _)| loaded(&self.blocks[*next].file))
            .collect();
        let total: u32 = next.iter().map(|(_, weight)| **weight).sum();
        if total == 0 {
            let tempo = self.blocks[name].to_bpm;
            return self
                .blocks
                .iter()
                .find(|(_, block)| {
                    block.from_bpm == tempo && block.to_bpm == tempo && loaded(&block.file)
                })
                .map_or(name, |(other, _)| other.as_str());
        }
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

pub(super) fn decoder(file: &str) -> Result<BlockDecoder, String> {
    let index = NAMES
        .iter()
        .position(|name| *name == file)
        .ok_or_else(|| format!("unknown music file {file}"))?;
    let bytes = block_bytes(index).ok_or_else(|| format!("{file} is still loading"))?;
    Decoder::new(Cursor::new(bytes)).map_err(|error| error.to_string())
}
