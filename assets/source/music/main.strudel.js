// Star Trader music — the main track: drift, ascent, departure, cruise (with
// the guitar solo), arrival (with the Orbital Fanfare) and the return. Its
// blocks follow the library's 8-chord cycle, one chord per bar. Rendered with
// library.strudel.js; see tools/music/README.md.

// The alternate cruise and arrival melody, eight bars, one per chord.
const melodyB = '[d5@2 e5 f5] [d5@3 a4] [c5@2 e5 c5] [g4@2 a4 c5] [e5@2 f5 a5] [f5@3 d5] [bb4@2 d5 f5] [e5@3 d5]'
// A 16-bar solo: a singing single-note line with six bends, strummed chords
// and double-stops at the phrase peaks, rests to breathe, and three short runs
// at 1.8 times the eighth-note rate (eight notes over 5/9 of a bar). The second
// half climbs higher; its final A resolves into the next block's D minor.
const solo = '[~ a4 c5 d5] [f5^2@3 ~] [e5^1@2 c5 a4] [[g4,c5,e5]@2 ~ ~] ' +
  '[~ d5 f5 a5] [d6^2@2 c6 a5] [[[d5,g5,bb5] ~ ~ ~]@4 [g5 bb5h d6h f6h g6h f6h d6h bb5h]@5] [a5^1@3 ~] ' +
  '[[d5,f5,a5] ~ [d5,f5,a5] ~ ~ [f5,a5,d6] ~ ~] [[bb5,d6]@3 [a5 f5]] ' +
  '[[[c5,f5,a5] ~ ~ ~]@4 [a4 c5h f5h a5h c6h e6h c6h a5h]@5] [c6^2@3 ~] ' +
  '[~ [a5,d6] [c6,e6] [d6,f6]] [f6^2@3 ~] ' +
  '[[[d6,g6] ~ ~ ~]@4 [g6 f6h d6h bb5h a5h g5h f5h d5h]@5] [[a4,d5,e5]@2 [g4,d5] [e4,a4]]'
// A distinct 120 BPM theme: brass-like calls over a fast string-like pulse.
const fanfareLine = '[d5@2 f5 a5] [bb5@2 a5 f5] [a5@2 c6 a5] [g5@2 e5 c5] [a5@2 d6 c6] [bb5@2 d6 f6] [g5@2 bb5 d6] [e6@2 d6 a5]'
// The same answers as two-note strums, foreshadowing and echoing the solo.
const guitarStrums = '[~ [f4,a4] ~ [a4,d5]] [~ [f4,bb4] ~ [d4,f4]] [~ [a4,c5] ~ [c5,f5]] [[e4,g4] ~ [c4,e4] ~] ' +
  '[~ [d4,f4] ~ [f4,a4]] [~ [bb4,d5] ~ [f4,bb4]] [~ [bb4,d5] ~ [g4,bb4]] [[a4,d5] ~ ~ ~]'

const cruise = { keys: 0.13, ensemble: 0.13, arp: 0.13, arpLpf: 2200, lead: 0.19, guitar: 0.23, rhythm: 0.045,
  pump: 0.05, pluck: 0.04,
  organ: 0.08, horns: 0.1, woodwinds: 0.07,
  counter: 0.12, drums: 1, hats: 0.22, shaker: 0.11, lift: 0.08,
  bells: 0.05, stars: 0.05, starsSparse: 0.8 }
const arrive = { keys: 0.1, ensemble: 0.13, arp: 0.13, arpLpf: 3200,
  arpPhaserRate: 0.25, arpPhaserDepth: 0.16, arpPhaserSweep: 450,
  lead: 0.19, guitar: 0.23, rhythm: 0.055, pump: 0.08, pluck: 0.05, organ: 0.12, horns: 0.14, woodwinds: 0.08,
  counter: 0.12, stabs: 0.075, ostinato: 0.055, drums: 1,
  hats: 0.18, shaker: 0.16, ride: 0.055, toms: 0.07, lift: 0.12 }

const soloGuitar = { keys: 0.12, arpLpf: [1600, 2200], guitarDecay: 0.9, guitarSustain: 0.45, guitarRelease: 0.95,
  guitarLpf: 3600, guitarDelay: 0.07, guitarFeedback: 0.1, guitarRoom: 0.2, rhythm: 0.085,
  drums: 1, hats: 0.13, pad: 0.04, pump: 0, pluck: 0 }


window.musicStart = 'drift_a'
family('main', {
  // 60 BPM: drifting.
  drift_a: block({ from: 60, keys: 0.24, ensemble: 0.24, sequence: 0.1, pad: 0.1,
    bells: 0.16, stars: 0.1, starsSparse: 0.45,
    next: { drift_b: 2, rise_70: 2 } }),
  drift_b: block({ from: 60, keys: 0.22, ensemble: 0.23, bass: 0.5, guitar: 0.16, pad: 0.09, viola: 0.12,
    sequence: 0.09, bells: 0.1, stars: 0.08,
    next: { drift_a: 1, rise_70: 3 } }),
  rise_70: block({ from: 60, to: 70, keys: [0.22, 0.2], ensemble: [0.23, 0.2], guitar: [0.08, 0.13], pad: [0.1, 0.09],
    arp: [0, 0.07], sequence: [0.1, 0.09],
    shaker: [0, 0.08], lift: [0, 0.07], bells: 0.14, stars: 0.1,
    next: { rise_80: 2 } }),
  rise_80: block({ from: 70, to: 80, keys: [0.2, 0.18], ensemble: [0.2, 0.16], guitar: [0.13, 0.17], pad: [0.09, 0.07], pluck: [0, 0.04],
    arp: [0.07, 0.13], sequence: [0.09, 0], hats: [0, 0.22],
    shaker: [0.08, 0.1], lift: [0.07, 0.09], bells: 0.12, stars: 0.1,
    next: { depart_a: 1, depart_b: 1 } }),
  // 80 BPM: departing.
  depart_a: block({ from: 80, keys: 0.18, arp: 0.13, guitar: 0.19, pad: 0.06, viola: 0.1, organ: 0.04, horns: 0.05,
    hats: 0.22, shaker: 0.09, lift: 0.07,
    bells: 0.13, stars: 0.1,
    next: { depart_b: 2, rise_90: 2 } }),
  depart_b: block({ from: 80, keys: 0.16, arp: 0.13, arpNotes: arpWide, guitar: 0.17, pad: 0.06, viola: 0.08,
    organ: 0.05, horns: 0.05, woodwinds: 0.06, counter: 0.12, hats: 0.22,
    shaker: 0.09, lift: 0.07, bells: 0.1,
    stars: 0.08, next: { depart_a: 1, rise_90: 2 } }),
  rise_90: block({ from: 80, to: 90, keys: [0.18, 0.14], arp: 0.13, arpLpf: [1400, 1800], guitar: [0.17, 0.22],
    guitarNotes: guitarStrums, rhythm: [0, 0.025], pad: [0.06, 0.03], pluck: [0.04, 0.06], viola: 0.08,
    organ: [0.05, 0.07], horns: [0.05, 0.08], woodwinds: [0.04, 0.06], hats: 0.22,
    shaker: [0.08, 0.11], lift: [0.08, 0.1],
    bells: 0.1, stars: 0.08, next: { rise_100: 2 } }),
  rise_100: block({ from: 90, to: 100, keys: [0.14, 0.1], arp: 0.13, arpLpf: [1800, 2200], guitar: 0.22, rhythm: [0.025, 0.045], pluck: 0.06, pump: [0, 0.05],
    organ: [0.07, 0.08], horns: [0.08, 0.1], woodwinds: [0.06, 0.07], drums: 'in', hats: 0.22,
    shaker: [0.11, 0.14], lift: [0.1, 0.12],
    bells: [0.12, 0.05], stars: 0.07, next: { cruise_a: 2, cruise_b: 1 } }),
  // 100 BPM: cruising, with the melodies and the solo.
  cruise_a: block({ from: 100, ...cruise, next: { cruise_b: 2, solo: 1, rise_110: 1 } }),
  cruise_b: block({ from: 100, ...cruise, leadNotes: melodyB, guitarNotes: guitarStrums, viola: 0.08, next: { cruise_a: 1, solo: 2, rise_110: 1 } }),
  solo: block({ from: 100, bars: 16, ...soloGuitar, ensemble: [0.09, 0.14], arp: [0.05, 0.11],
    guitar: [0.22, 0.3], guitarNotes: solo, guitarBends: true,
    organ: [0.04, 0.11], horns: [0, 0.1], viola: [0, 0.12],
    next: { cruise_a: 1, rise_110: 2, fall_90: 1 } }),
  rise_110: block({ from: 100, to: 110, ...cruise, arpLpf: [2200, 2700], stabs: [0, 0.04],
    guitarNotes: guitarStrums, rhythm: [0.045, 0.055], pump: [0.05, 0.07], pluck: 0.06,
    organ: [0.08, 0.1], horns: [0.1, 0.12], woodwinds: [0.07, 0.08],
    shaker: [0.12, 0.15], lift: [0.1, 0.12], bells: 0, stars: 0,
    next: { rise_120: 1 } }),
  rise_120: block({ from: 110, to: 120, ...arrive, stabs: [0.04, 0.075],
    organ: [0.1, 0.12], horns: [0.12, 0.14], woodwinds: 0.08,
    ostinato: [0, 0.055], ride: [0, 0.055], toms: [0, 0.07],
    shaker: [0.15, 0.16], lift: [0.12, 0.14], leadNotes: octaveUp(melodyA),
    next: { arrive_a: 1, arrive_b: 1, fanfare: 1 } }),
  // 120 BPM: arriving, the peak.
  arrive_a: block({ from: 120, ...arrive, leadNotes: octaveUp(melodyA), next: { arrive_b: 2, fanfare: 1, fall_110: 2 } }),
  arrive_b: block({ from: 120, ...arrive, leadNotes: octaveUp(melodyB), guitarNotes: guitarStrums, next: { arrive_a: 1, fanfare: 1, fall_110: 2 } }),
  fanfare: block({ from: 120, ...arrive, lead: 0.21, leadNotes: fanfareLine,
    horns: 0.18, organ: 0.14, woodwinds: 0.05, guitar: 0.11, counter: 0, pump: 0.07,
    arp: 0.1, stabs: 0.05, ostinato: 0.07,
    next: { arrive_a: 1, arrive_b: 1, fall_110: 2 } }),
  fall_110: block({ from: 120, to: 110, ...arrive, stabs: [0.075, 0.04], pump: [0.08, 0.05], pluck: [0.05, 0.03],
    shaker: [0.16, 0], ostinato: [0.055, 0], ride: [0.055, 0], toms: [0.07, 0],
    lift: [0.12, 0], leadNotes: octaveUp(melodyB),
    next: { fall_100: 1 } }),
  fall_100: block({ from: 110, to: 100, ...cruise, keys: [0.1, 0.13], arpLpf: [3200, 2200], stabs: [0.04, 0],
    guitarNotes: guitarStrums, pump: [0.05, 0.02], pluck: [0.03, 0],
    shaker: 0, lift: 0,
    bells: [0, 0.05], next: { solo: 1, cruise_a: 1, fall_90: 2 } }),
  // Back down through 90, 80, 70 to 60.
  fall_90: block({ from: 100, to: 90, keys: [0.13, 0.16], arp: 0.13, guitar: [0.21, 0.14], rhythm: [0.045, 0], pump: [0.02, 0], counter: [0.16, 0.08], pad: [0, 0.04],
    organ: [0.08, 0.03], horns: [0.1, 0], woodwinds: [0.07, 0],
    drums: 'out', hats: 0.22, bells: 0.08, stars: 0.08, next: { fall_80: 1 } }),
  fall_80: block({ from: 90, to: 80, keys: [0.16, 0.18], arp: 0.13, arpLpf: [1800, 1400], guitar: [0.14, 0.12], counter: [0.08, 0], pad: [0.04, 0.07], viola: 0.1,
    hats: 0.22, bells: [0.08, 0.13], stars: 0.1, next: { wind: 1 } }),
  wind: block({ from: 80, keys: 0.18, arp: 0.13, guitar: 0.12, pad: 0.07, viola: 0.11, hats: 0.22,
    sequence: 0.035, bells: 0.13, stars: 0.1,
    next: { fall_70: 2, depart_a: 1 } }),
  fall_70: block({ from: 80, to: 70, keys: [0.18, 0.2], arp: [0.13, 0.07], guitar: [0.12, 0], sequence: [0.035, 0.06], pad: 0.08, viola: [0.1, 0.06],
    hats: [0.22, 0],
    bells: 0.14, stars: 0.1, next: { fall_60: 1 } }),
  fall_60: block({ from: 70, to: 60, keys: [0.2, 0.22], arp: [0.07, 0], pad: [0.08, 0.1],
    sequence: [0.06, 0.045],
    bells: 0.16, stars: 0.1, next: { drift_a: 1 } }),
}, [
  'drift_a', 'drift_b', 'space', 'drift_a', 'rise_70', 'rise_80', 'space_80', 'depart_a',
  'depart_b', 'rise_90', 'rise_100', 'cruise_a', 'solo', 'space_100', 'cruise_a', 'cruise_b',
  'rise_110', 'rise_120', 'arrive_a', 'fanfare', 'space_120', 'arrive_b', 'fall_110', 'fall_100',
  'fall_90', 'fall_80', 'wind', 'fall_70', 'fall_60'
])
