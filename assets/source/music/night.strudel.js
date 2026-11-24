// Star Trader music — the Night Drift pieces at 60, 80, 100 and 120 BPM and the
// 20 BPM ramps between them: dark Memphis and drift phonk in space, with
// cowbell melodies, sliding 808s, trap hats and a half-time phonk kit.
// Rendered with library.strudel.js; see tools/music/README.md.

// A dark minor cycle, two bars per chord: Dm, Bb, Gm and A7, whose C sharp
// (harmonic minor) pulls hard back to D minor.
const nightHarmony = { chords: "<Dm Dm Bb Bb Gm Gm A7 A7>", roots: "<d2 d2 bb1 bb1 g1 g1 a1 a1>",
  rootNumbers: "<38 38 34 34 31 31 33 33>" }

// The Night Drift pieces have two themes, like the other families. Theme E, a busy
// cowbell melody in sixteenth-note steps, plays at 60 and 120 BPM with the same
// note values, so 120 is exactly double time. Theme F, sparser and darker,
// plays on the distorted synth lead at 80 and 100 BPM. Both use one bar per
// step of the harmony; over A7 they use D flat, the C sharp of the chord.
const themeE = '<' +
  '[[d6 ~ d6 ~] [a5 ~ d6 ~] [f6 ~ e6 ~] [d6 ~ a5 ~]] ' +
  '[[d6 ~ d6 ~] [a5 ~ d6 ~] [f6 e6 d6 ~] [c6 ~ a5 ~]] ' +
  '[[bb5 ~ bb5 ~] [f5 ~ bb5 ~] [d6 ~ c6 ~] [bb5 ~ f5 ~]] ' +
  '[[bb5 ~ bb5 ~] [f5 ~ bb5 ~] [d6 c6 bb5 ~] [a5 ~ f5 ~]] ' +
  '[[g5 ~ g5 ~] [d5 ~ g5 ~] [bb5 ~ a5 ~] [g5 ~ d5 ~]] ' +
  '[[g5 ~ g5 ~] [d5 ~ g5 ~] [bb5 a5 g5 ~] [f5 ~ d5 ~]] ' +
  '[[a5 ~ a5 ~] [e5 ~ a5 ~] [db6 ~ e6 ~] [a5 ~ e5 ~]] ' +
  '[[a5 ~ a5 ~] [e5 ~ a5 ~] [g5 ~ e5 ~] [db6 ~ ~ ~]]>'
const themeF = '<' +
  '[[d5 ~ ~ f5] [~ ~ a5 ~] [~ g5 ~ f5] [e5 ~ d5 ~]] ' +
  '[[d5 ~ ~ f5] [~ ~ a5 ~] [c6 ~ a5 ~] [g5 ~ ~ ~]] ' +
  '[[d5 ~ ~ f5] [~ ~ bb5 ~] [~ a5 ~ f5] [d5 ~ c5 ~]] ' +
  '[[d5 ~ ~ f5] [~ ~ bb5 ~] [d6 ~ bb5 ~] [a5 ~ ~ ~]] ' +
  '[[bb4 ~ ~ d5] [~ ~ g5 ~] [~ f5 ~ d5] [bb4 ~ a4 ~]] ' +
  '[[bb4 ~ ~ d5] [~ ~ g5 ~] [bb5 ~ g5 ~] [f5 ~ ~ ~]] ' +
  '[[a4 ~ ~ db5] [~ ~ e5 ~] [~ g5 ~ e5] [db5 ~ a4 ~]] ' +
  '[[a4 ~ ~ db5] [~ ~ e5 ~] [a5 ~ g5 ~] [e5 ~ ~ ~]]>'

// A picked bass guitar groove under the 808, two bars per chord.
const nightBassLine = '<' +
  '[[d2 ~ ~ d2] [~ ~ f2 ~] [a2 ~ ~ g2] [~ f2 ~ ~]] [[d2 ~ ~ d2] [~ ~ f2 ~] [c3 ~ a2 ~] [g2 ~ e2 ~]] ' +
  '[[bb1 ~ ~ bb1] [~ ~ d2 ~] [f2 ~ ~ e2] [~ d2 ~ ~]] [[bb1 ~ ~ bb1] [~ ~ d2 ~] [a2 ~ f2 ~] [d2 ~ c2 ~]] ' +
  '[[g1 ~ ~ g1] [~ ~ bb1 ~] [d2 ~ ~ c2] [~ bb1 ~ ~]] [[g1 ~ ~ g1] [~ ~ bb1 ~] [f2 ~ d2 ~] [bb1 ~ a1 ~]] ' +
  '[[a1 ~ ~ a1] [~ ~ db2 ~] [e2 ~ ~ g2] [~ e2 ~ ~]] [[a1 ~ ~ a1] [~ ~ db2 ~] [g2 ~ e2 ~] [db2 ~ a1 ~]]>'

// A bowed bass: two long notes per chord, root then fifth, in the contrabass
// range.
const bowedLine = '<d2 [d2@3 a1] bb1 [bb1@3 f1] g1 [g1@3 d2] a1 [a1@3 e1]>'

// The Night Drift pieces' settings, shared by the pieces and the ramps between them.
// The kit sits under the music; rimshots, toms, shaker and open hats vary it.
const nightAt = {
  // Theme E, slow and eerie: the cowbell over long sliding 808s and the bass
  // guitar, a dark pad, a lowpassed choir and crackle, the kit held back.
  60: {
    bowed: 0.1, bowedNotes: bowedLine, brass: 0.04, ensemble: 0, bass: 0, cowbell: 0.08, cowbellNotes: themeE, bass808: 0.234, slide808: true,
    bassGuitar: 0.195, bassGuitarNotes: nightBassLine, pad: 0.07, choir: 0.05, halo: 0.06,
    crackle: 0.05, phonkKit: 0.175, trap: 0.017, rim: 0.087, shaker: 0.028
  },
  // Theme F on the dark lead, with the kit, trap hats, rimshots, toms and a
  // half-time clap.
  80: {
    bowed: 0.1, bowedNotes: bowedLine, brass: 0.06, ensemble: 0, bass: 0, dark: 0.1, darkNotes: themeF, bass808: 0.286, slide808: true,
    bassGuitar: 0.234, bassGuitarNotes: nightBassLine, phonkKit: 0.266, trap: 0.025, clap: 0.035,
    clapPattern: "~ ~ pink ~", rim: 0.105, toms: 0.028, shaker: 0.035, pad: 0.05, choir: 0.05,
    crackle: 0.04
  },
  // Theme F, heavier: the cowbell doubles the lead an octave up.
  100: {
    bowed: 0.09, bowedNotes: bowedLine, brass: 0.07, ensemble: 0, bass: 0, dark: 0.12, darkNotes: themeF, cowbell: 0.05,
    cowbellNotes: octaveUp(themeF), bass808: 0.325, slide808: true, bassGuitar: 0.234,
    bassGuitarNotes: nightBassLine, phonkKit: 0.343, trap: 0.035, clap: 0.044,
    clapPattern: "~ ~ pink ~", rim: 0.122, toms: 0.035, shaker: 0.035, openhat: 0.014, choir: 0.06,
    chop: 0.03, crackle: 0.03
  },
  // Theme E at double time: drift phonk, cowbell up front over the full kit.
  120: {
    bowed: 0.08, bowedNotes: bowedLine, brass: 0.07, ensemble: 0, bass: 0, cowbell: 0.13, cowbellNotes: themeE, dark: 0.05,
    darkNotes: octaveDown(themeE), bass808: 0.338, slide808: true, bassGuitar: 0.208,
    bassGuitarNotes: nightBassLine, phonkKit: 0.392, trap: 0.039, clap: 0.054,
    clapPattern: "~ ~ pink ~", rim: 0.122, toms: 0.042, shaker: 0.042, openhat: 0.021, choir: 0.06,
    zaps: 0.03, crackle: 0.03
  },
}
const nightRamp = (from, to, next) => familyRamp(nightAt, nightHarmony, from, to, next)

family('night', {
  night_60: block({ from: 60, ...nightHarmony, ...nightAt[60], next: { night_rise_80: 2 } }),
  night_80: block({ from: 80, ...nightHarmony, ...nightAt[80], next: { night_rise_100: 2, night_fall_60: 1 } }),
  night_100: block({ from: 100, ...nightHarmony, ...nightAt[100], next: { night_rise_120: 2, night_fall_80: 1 } }),
  night_120: block({ from: 120, ...nightHarmony, ...nightAt[120], next: { night_fall_100: 2 } }),
  // Ramps between the Night Drift pieces, 20 BPM each way.
  night_rise_80: nightRamp(60, 80, { night_80: 1 }),
  night_rise_100: nightRamp(80, 100, { night_100: 1 }),
  night_rise_120: nightRamp(100, 120, { night_120: 1 }),
  night_fall_100: nightRamp(120, 100, { night_100: 1 }),
  night_fall_80: nightRamp(100, 80, { night_80: 1 }),
  night_fall_60: nightRamp(80, 60, { night_60: 1 }),
}, [
  'night_60', 'night_rise_80', 'night_80', 'night_rise_100', 'night_100', 'night_rise_120',
  'night_120', 'night_fall_100', 'night_100', 'night_fall_80', 'night_80', 'night_fall_60',
  'night_60',
])
