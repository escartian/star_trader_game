// Star Trader music — the phonk pieces at 60, 80, 100 and 120 BPM and the
// 20 BPM ramps between them: dark Memphis and drift phonk in space, with
// cowbell melodies, sliding 808s, trap hats and a half-time phonk kit.
// Rendered with library.strudel.js; see tools/music/README.md.

// A dark minor cycle, two bars per chord: Dm, Bb, Gm and A7, whose C sharp
// (harmonic minor) pulls hard back to D minor.
const phonkHarmony = { chords: "<Dm Dm Bb Bb Gm Gm A7 A7>", roots: "<d2 d2 bb1 bb1 g1 g1 a1 a1>",
  rootNumbers: "<38 38 34 34 31 31 33 33>" }

// The phonk pieces have two themes, like the other families. Theme E, a busy
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

// The phonk pieces' settings, shared by the pieces and the ramps between them.
const phonkAt = {
  // Theme E, slow and eerie: the cowbell over long sliding 808s, a dark pad,
  // a lowpassed choir and crackle, with the kit and hats held back.
  60: {
    ensemble: 0, bass: 0, cowbell: 0.08, cowbellNotes: themeE, bass808: 0.22, slide808: true,
    pad: 0.07, choir: 0.05, halo: 0.06, crackle: 0.05, phonkKit: 0.35, trap: 0.035
  },
  // Theme F on the dark lead, with the kit, trap hats and a half-time clap.
  80: {
    ensemble: 0, bass: 0, dark: 0.1, darkNotes: themeF, bass808: 0.26, slide808: true,
    phonkKit: 0.55, trap: 0.05, clap: 0.07, clapPattern: "~ ~ pink ~", pad: 0.05, choir: 0.05,
    crackle: 0.04
  },
  // Theme F, heavier: the cowbell doubles the lead an octave up.
  100: {
    ensemble: 0, bass: 0, dark: 0.12, darkNotes: themeF, cowbell: 0.05,
    cowbellNotes: octaveUp(themeF), bass808: 0.3, slide808: true, phonkKit: 0.7, trap: 0.07,
    clap: 0.09, clapPattern: "~ ~ pink ~", choir: 0.06, chop: 0.03, crackle: 0.03
  },
  // Theme E at double time: drift phonk, cowbell up front over the full kit.
  120: {
    ensemble: 0, bass: 0, cowbell: 0.13, cowbellNotes: themeE, dark: 0.05,
    darkNotes: octaveDown(themeE), bass808: 0.32, slide808: true, phonkKit: 0.8, trap: 0.08,
    clap: 0.11, clapPattern: "~ ~ pink ~", choir: 0.06, zaps: 0.03, crackle: 0.03
  },
}
const phonkRamp = (from, to, next) => familyRamp(phonkAt, phonkHarmony, from, to, next)

family('phonk', {
  phonk_60: block({ from: 60, ...phonkHarmony, ...phonkAt[60], next: { phonk_rise_80: 2 } }),
  phonk_80: block({ from: 80, ...phonkHarmony, ...phonkAt[80], next: { phonk_rise_100: 2, phonk_fall_60: 1 } }),
  phonk_100: block({ from: 100, ...phonkHarmony, ...phonkAt[100], next: { phonk_rise_120: 2, phonk_fall_80: 1 } }),
  phonk_120: block({ from: 120, ...phonkHarmony, ...phonkAt[120], next: { phonk_fall_100: 2 } }),
  // Ramps between the phonk pieces, 20 BPM each way.
  phonk_rise_80: phonkRamp(60, 80, { phonk_80: 1 }),
  phonk_rise_100: phonkRamp(80, 100, { phonk_100: 1 }),
  phonk_rise_120: phonkRamp(100, 120, { phonk_120: 1 }),
  phonk_fall_100: phonkRamp(120, 100, { phonk_100: 1 }),
  phonk_fall_80: phonkRamp(100, 80, { phonk_80: 1 }),
  phonk_fall_60: phonkRamp(80, 60, { phonk_60: 1 }),
}, [
  'phonk_60', 'phonk_rise_80', 'phonk_80', 'phonk_rise_100', 'phonk_100', 'phonk_rise_120',
  'phonk_120', 'phonk_fall_100', 'phonk_100', 'phonk_fall_80', 'phonk_80', 'phonk_fall_60',
  'phonk_60',
])
