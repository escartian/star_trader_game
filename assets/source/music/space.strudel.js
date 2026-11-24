// Star Trader music — the space pieces at 60, 80, 100 and 120 BPM and the
// 20 BPM ramps between them. Rendered with library.strudel.js; see
// tools/music/README.md.

// The space pieces: two bars per chord, ending on A to resolve like the others.
const spaceHarmony = { chords: "<Dm9 Dm9 Bb^7 Bb^7 Gm9 Gm9 A7sus A7sus>", roots: "<d2 d2 bb1 bb1 g1 g1 a1 a1>",
  rootNumbers: "<38 38 34 34 31 31 33 33>" }
// The space pieces have two themes. Theme A, the hook, plays at 60 and 120 BPM
// with the same note values, so 120 is exactly double time; theme B, a
// French-house riff, plays at 80 and 100 BPM. The ramps between them crossfade.
//
// The hook (theme A): a constellation of wide leaps, fifths and ninths, one
// two-bar phrase per chord with the same shape each time, rising a little with
// each chord. Eight eighth-note steps a bar, played staccato on a glassy FM
// bell-synth. At 120 an echo canon an eighth late fills its rests.
const hookPhrases = [
  ['a5 ~ e6 ~ d6 ~ ~ a5', 'c6 ~ ~ e6 ~ f6 e6 ~'], // Dm9
  ['f5 ~ c6 ~ bb5 ~ ~ f5', 'a5 ~ ~ c6 ~ d6 c6 ~'], // Bb^7
  ['d6 ~ a6 ~ g6 ~ ~ d6', 'f6 ~ ~ a6 ~ bb6 a6 ~'], // Gm9
  ['e6 ~ a6 ~ g6 ~ ~ e6', 'd6 ~ e6 ~ g6 ~ a6@2'], // A7sus, resolving to D
]
const hookNotes = '<' + hookPhrases.flatMap((phrase) => phrase.map((bar) => '[' + bar + ']')).join(' ') + '>'
// The riff (theme B): a staccato, syncopated sixteenth-note figure high on each
// chord, in the manner of filtered French house. One phrase per chord, two bars.
const riffPhrases = [
  ['~ a5 d6 ~ e6 ~ d6 a5 ~ f6 ~ e6 d6 ~ a5 ~', '~ a5 d6 ~ e6 ~ f6 ~ a6 ~ ~ g6 f6 ~ e6 ~'], // Dm9
  ['~ f5 bb5 ~ d6 ~ bb5 f5 ~ a5 ~ d6 c6 ~ a5 ~', '~ f5 bb5 ~ d6 ~ f6 ~ a6 ~ ~ f6 d6 ~ c6 ~'], // Bb^7
  ['~ g5 bb5 ~ d6 ~ bb5 g5 ~ a5 ~ d6 c6 ~ bb5 ~', '~ g5 bb5 ~ d6 ~ f6 ~ a6 ~ ~ g6 f6 ~ d6 ~'], // Gm9
  ['~ e5 a5 ~ d6 ~ a5 e5 ~ g5 ~ d6 e6 ~ g6 ~', '~ a5 d6 ~ e6 ~ g6 ~ a6 ~ g6 ~ e6 d6 ~ ~'], // A7sus
]
const riffNotes = '<' + riffPhrases.flatMap((phrase) => phrase.map((bar) => '[' + bar + ']')).join(' ') + '>'
// The space pieces' settings, shared by the pieces and the ramps between them.
const spaceAt = {
  // Theme A: the hook as a quiet staccato nod, with a faint echo.
  60: {
    keys: 0.12, ensemble: 0.2, bass: 0.26, space: 0.19, sequence: 0.1, pad: 0.1, halo: 0.12,
    crystal: 0.08, beeps: 0.1, hookNotes, hook: 0.09, hookEcho: 0.04, laser: 0.06, bells: 0.1, stars: 0.07,
    starsSparse: 0.6
  },
  // Theme B: the riff over the rhythm box and a disco octave bass.
  80: {
    keys: 0.08, ensemble: 0.12, bass: 0, seqbass: 0.1, seqSteps: discoSteps, halo: 0.09,
    pad: 0.08, sequence: 0.06, crystal: 0.06, beeps: 0.08, machine: 0.5, riffNotes, riff: 0.09, harp: 0.05,
    bells: 0.08, stars: 0.06
  },
  // Theme B with drums, open hats, chopped chords and the choir.
  100: {
    keys: 0.06, ensemble: 0.1, bass: 0.2, seqbass: 0.12, seqSteps: discoSteps, halo: 0.06,
    choir: 0.07, pad: 0.05, crystal: 0.04, beeps: 0.06, drums: 1, hats: 0.12, openhat: 0.03,
    toms: 0.05, riffNotes, riff: 0.13, chop: [0.03, 0.06], harp: 0.05
  },
  // Theme A at double time, with the full house groove and the echo canon.
  120: {
    keys: 0.05, ensemble: 0.12, bass: 0.4, seqbass: 0.13, seqSteps: sixteenthSteps, choir: 0.08,
    pump: 0.06, chop: 0.06, arp: 0.1, arpLpf: 3200, arpPhaserRate: 0.25, arpPhaserDepth: 0.16,
    arpPhaserSweep: 450, zaps: 0.06, beeps: 0.04, drums: 1, hats: 0.14, openhat: 0.04,
    shaker: 0.12, ride: 0.05, toms: 0.07, hookNotes, hook: 0.16, hookLow: 0.11, hookEcho: 0.08, pad: 0.05,
    laser: 0.07
  },
}
// A 20 BPM ramp between two space pieces, with an orchestral harp playing its
// glissandi throughout to mark the chord changes and link the two themes.
const spaceRamp = (from, to, next) => familyRamp(spaceAt, spaceHarmony, from, to, next, { harp: 0.09 })

family('space', {
  // 60 to 120 BPM: the space pieces.
  space: block({ from: 60, ...spaceHarmony, ...spaceAt[60],
    next: { space_rise_80: 1 } }),
  space_80: block({ from: 80, ...spaceHarmony, ...spaceAt[80],
    next: { space_rise_100: 1, space_fall_60: 1 } }),
  space_100: block({ from: 100, ...spaceHarmony, ...spaceAt[100],
    next: { space_rise_120: 1, space_fall_80: 1 } }),
  space_120: block({ from: 120, ...spaceHarmony, ...spaceAt[120],
    next: { space_fall_100: 1 } }),
  // Ramps between the space pieces, 20 BPM each way.
  space_rise_80: spaceRamp(60, 80, { space_80: 1 }),
  space_rise_100: spaceRamp(80, 100, { space_100: 1 }),
  space_rise_120: spaceRamp(100, 120, { space_120: 1 }),
  space_fall_100: spaceRamp(120, 100, { space_100: 1 }),
  space_fall_80: spaceRamp(100, 80, { space_80: 1 }),
  space_fall_60: spaceRamp(80, 60, { space: 1 }),
}, [
  'space', 'space_rise_80', 'space_80', 'space_rise_100', 'space_100', 'space_rise_120',
  'space_120', 'space_fall_100', 'space_100', 'space_fall_80', 'space_80', 'space_fall_60',
  'space'
])
