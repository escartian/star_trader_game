use super::*;
use std::collections::BTreeSet;

#[test]
fn entry_ramp_changes_both_stereo_channels_together() {
    let mut ramp = EntryRamp {
        start_gain: 0.4,
        total_samples: 8,
        samples_left: 8,
    };
    let gains: Vec<_> = (0..8).map(|_| ramp.gain()).collect();
    for index in (0..8).step_by(2) {
        assert_eq!(gains[index], gains[index + 1]);
    }
    assert!(gains.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(gains[0], 0.4);
    assert_eq!(ramp.gain(), 1.0);
}

#[test]
fn published_part_follows_the_sample_boundary() {
    let mut source = MusicGraph::new().unwrap();
    let handle = source.part_handle();
    assert_eq!(part_name(handle.load(Ordering::Acquire)), Some("drift_a"));
    let possible = source.graph.blocks["drift_a"].next.clone();
    source.clock.samples_left = 1;
    source.next();
    assert_eq!(part_name(handle.load(Ordering::Acquire)), Some("drift_a"));
    source.next();
    let next = part_name(handle.load(Ordering::Acquire)).unwrap();
    assert!(possible.contains_key(next));
}

#[test]
fn announced_next_part_is_the_one_that_plays() {
    let mut source = MusicGraph::new().unwrap();
    let now = source.part_handle();
    let up_next = source.up_next_handle();
    for _ in 0..5 {
        let announced = up_next.load(Ordering::Acquire);
        let current = part_name(now.load(Ordering::Acquire)).unwrap();
        let possible = &source.graph.blocks[current].next;
        assert!(possible.contains_key(part_name(announced).unwrap()));
        source.clock.samples_left = 0;
        source.next();
        assert_eq!(now.load(Ordering::Acquire), announced);
    }
    // A Songbook jump announces a successor of the chosen part.
    source.jump_to(part_index("solo").unwrap());
    let next = part_name(up_next.load(Ordering::Acquire)).unwrap();
    assert!(source.graph.blocks["solo"].next.contains_key(next));
}

#[test]
fn queued_part_plays_next_at_the_boundary() {
    let mut source = MusicGraph::new().unwrap();
    let now = source.part_handle();
    let up_next = source.up_next_handle();
    // A part at another tempo than drift_a's 60 BPM: queueing ignores tempo.
    let fanfare = part_index("fanfare").unwrap();
    source.queue_handle().store(fanfare, Ordering::Release);
    source.next(); // Left channel: the request is taken here.
    assert_eq!(
        up_next.load(Ordering::Acquire),
        fanfare,
        "announced at once"
    );
    assert_eq!(part_name(now.load(Ordering::Acquire)), Some("drift_a"));
    source.clock.samples_left = 0;
    source.next();
    assert_eq!(now.load(Ordering::Acquire), fanfare);
    let next = part_name(up_next.load(Ordering::Acquire)).unwrap();
    assert!(source.graph.blocks["fanfare"].next.contains_key(next));
}

#[test]
fn songbook_selection_crossfades_on_a_stereo_frame() {
    let mut source = MusicGraph::new().unwrap();
    let selection = source.selection_handle();
    let now = source.part_handle();
    let mut old = decoder("drift_a.ogg").unwrap();
    let arrive = part_index("arrive_a").unwrap();
    selection.store(arrive, Ordering::Release);
    assert_eq!(
        source.next(),
        old.next(),
        "the fade starts with the old part"
    );
    assert_eq!(source.current_name, "arrive_a");
    assert_eq!(now.load(Ordering::Acquire), arrive);

    let space = part_index("space").unwrap();
    selection.store(space, Ordering::Release);
    source.next(); // Right channel: do not switch halfway through a frame.
    assert_eq!(source.current_name, "arrive_a");
    source.next(); // Next left channel.
    assert_eq!(source.current_name, "space");
    assert_eq!(now.load(Ordering::Acquire), space);
    for _ in 0..SELECTION_FADE_SAMPLES {
        source.next();
    }
    assert!(source.selection_fade.is_none());
    assert!(source.tail.is_none(), "old audio is dropped after the fade");
}

#[test]
fn graph_has_valid_tempos_and_decodable_audio() {
    let graph = Manifest::embedded().unwrap();
    let rms = graph.blocks.values().map(|block| block.rms);
    let quietest = rms.clone().fold(f32::INFINITY, f32::min);
    let loudest = rms.fold(0.0, f32::max);
    let spread_db = 20.0 * (loudest / quietest).log10();
    assert!(
        spread_db < 3.0,
        "active block RMS spread is {spread_db:.1} dB"
    );
    let mut reached = BTreeSet::from([graph.start.as_str()]);
    let mut pending = vec![graph.start.as_str()];
    while let Some(name) = pending.pop() {
        for next in graph.blocks[name].next.keys() {
            if reached.insert(next.as_str()) {
                pending.push(next.as_str());
            }
        }
    }
    assert_eq!(
        reached.len(),
        graph.blocks.len(),
        "every block is reachable"
    );
    for (name, block) in &graph.blocks {
        let source = decoder(&block.file).unwrap();
        assert_eq!(
            (source.channels(), source.sample_rate()),
            (2, 48_000),
            "{name}"
        );
        // Vorbis decoders may expose a short final packet pad past the
        // declared granule length; it comes after the nine-second tail.
        let expected = (block.frames + 9 * 48_000) * 2;
        let decoded = source.count();
        assert!(
            decoded >= expected && decoded <= expected + 2048,
            "{name}: {decoded} samples"
        );
    }
}

#[test]
fn joins_on_the_exact_stereo_sample() {
    let mut clock = JoinClock::new(3);
    for _ in 0..6 {
        assert!(!clock.boundary());
        clock.tick();
    }
    assert!(clock.boundary());
    let mut sequencer = MusicGraph::new().unwrap();
    let mut old = decoder(&sequencer.graph.blocks["drift_a"].file).unwrap();
    sequencer.clock = JoinClock::new(1);
    assert_eq!(sequencer.next(), old.next());
    assert_eq!(sequencer.current_name, "drift_a");
    assert_eq!(sequencer.next(), old.next());
    assert_eq!(sequencer.current_name, "drift_a");
    let joined = sequencer.next().unwrap();
    assert_ne!(sequencer.current_name, "drift_a");
    let mut new = decoder(&sequencer.graph.blocks[&sequencer.current_name].file).unwrap();
    assert_eq!(
        joined,
        old.next().unwrap() + new.next().unwrap() * sequencer.ramp.start_gain
    );
}
