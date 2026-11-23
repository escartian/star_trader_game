// Star Trader music — the funk pieces at 60, 80, 100 and 120 BPM and the
// 20 BPM ramps between them: the main theme reworked as space funk that turns
// to phonk as the tempo rises. Rendered with library.strudel.js; see
// tools/music/README.md.

// A funk vamp in D dorian, two bars per chord: Dm9, G9 (its major IV), Dm9,
// then Bb^7 and A7sus, ending on A to resolve into every other block.
const funkHarmony = { chords: "<Dm9 Dm9 G9 G9 Dm9 Dm9 Bb^7 A7sus>", roots: "<d2 d2 g1 g1 d2 d2 bb1 a1>",
  rootNumbers: "<38 38 31 31 38 38 34 33>" }

// The funk pieces have two themes, like the space pieces. Theme C, the main
// theme's melody (melodyA, one bar per chord), plays at 60 and 120 BPM with the
// same note values, so 120 is exactly double time: on a talkbox lead at 60,
// and on a phonk cowbell an octave up at 120. Theme D, a syncopated clavinet
// riff, plays at 80 and 100 BPM; at 100 a cowbell doubles it an octave up.
const themeC = '<' + melodyA + '>'
// Theme D: sixteenth-note steps, one bar per chord. Over G9 it uses B natural,
// the dorian colour of funk.
const funkRiff = '<' +
  '[[d5 ~ f5 d5] [~ a4 c5 ~] [d5 ~ ~ f5] [~ e5 d5 ~]] ' +
  '[[d5 ~ f5 d5] [~ a4 c5 ~] [f5 ~ e5 d5] [c5 ~ a4 ~]] ' +
  '[[b4 ~ d5 b4] [~ f4 a4 ~] [b4 ~ ~ d5] [~ c5 b4 ~]] ' +
  '[[b4 ~ d5 b4] [~ f4 a4 ~] [d5 ~ c5 b4] [a4 ~ f4 ~]] ' +
  '[[d5 ~ f5 d5] [~ a4 c5 ~] [d5 ~ ~ f5] [~ e5 d5 ~]] ' +
  '[[d5 ~ f5 d5] [~ a4 c5 ~] [f5 ~ e5 d5] [c5 ~ a4 ~]] ' +
  '[[bb4 ~ d5 bb4] [~ f4 a4 ~] [bb4 ~ ~ d5] [~ c5 bb4 ~]] ' +
  '[[a4 ~ d5 a4] [~ e4 g4 ~] [a4 ~ g4 e4] [d5 ~ e5 ~]]>'
// Slap bass, one bar per chord: thumbed roots, popped octaves and walk-ups.
const slapLine = '<' +
  '[[d2 ~ d3 ~] [~ d2 ~ f2] [~ g2 ~ d2] [a2 ~ c3 d3]] ' +
  '[[d2 ~ d3 ~] [~ d2 ~ f2] [~ a2 ~ c3] [d3 ~ c3 a2]] ' +
  '[[g1 ~ g2 ~] [~ g1 ~ b1] [~ c2 ~ g1] [d2 ~ f2 g2]] ' +
  '[[g1 ~ g2 ~] [~ g1 ~ b1] [~ d2 ~ f2] [g2 ~ f2 d2]] ' +
  '[[d2 ~ d3 ~] [~ d2 ~ f2] [~ g2 ~ d2] [a2 ~ c3 d3]] ' +
  '[[d2 ~ d3 ~] [~ d2 ~ f2] [~ a2 ~ c3] [d3 ~ c3 a2]] ' +
  '[[bb1 ~ bb2 ~] [~ bb1 ~ d2] [~ f2 ~ bb1] [c2 ~ d2 f2]] ' +
  '[[a1 ~ a2 ~] [~ a1 ~ e2] [~ g2 ~ a1] [e2 ~ g2 a2]]>'

// The funk pieces' settings, shared by the pieces and the ramps between them.
const funkAt = {
  // Theme C, slow and sultry: talkbox lead, laser harp, slap bass and a light
  // groove, with a little space colour.
  60: {
    keys: 0.105, ensemble: 0.15, bass: 0, slap: 0.16, slapNotes: slapLine, talk: 0.12,
    talkNotes: themeC, laser: 0.06, scratch: 0.04, groove: 0.325, hats: 0.052, pad: 0.09,
    halo: 0.075, bells: 0.05, beeps: 0.04
  },
  // Theme D: the clavinet riff with slap bass, chicken scratch and horn stabs.
  80: {
    keys: 0.075, ensemble: 0.12, bass: 0, slap: 0.18, slapNotes: slapLine, clav: 0.12,
    clavNotes: funkRiff, scratch: 0.06, brass: 0.07, groove: 0.455, hats: 0.078, organ: 0.06,
    laser: 0.03, pad: 0.06
  },
  // Theme D turning phonk: an 808 under the slap bass, trap hats, claps and a
  // cowbell doubling the riff.
  100: {
    ensemble: 0.12, bass: 0, slap: 0.13, slapNotes: slapLine, bass808: 0.22, clav: 0.12,
    clavNotes: funkRiff, cowbell: 0.05, cowbellNotes: octaveUp(funkRiff), scratch: 0.05,
    brass: 0.08, groove: 0.52, trap: 0.046, clap: 0.065, pump: 0.04, laser: 0.04
  },
  // Theme C at double time, phonk and funk together: cowbell melody, distorted
  // 808, trap hat rolls and a half-time clap, under the talkbox and laser harp,
  // with the slap bass, chicken scratch, horn stabs, electric-piano comping,
  // congas and tambourine keeping it funky.
  120: {
    ensemble: 0.09, bass: 0, bass808: 0.28, slap: 0.12, slapNotes: slapLine, cowbell: 0.12,
    scratch: 0.06, brass: 0.08, keys: 0.06, congas: 0.12, tambourine: 0.05,
    cowbellNotes: octaveUp(themeC), talk: 0.08, talkNotes: themeC, laser: 0.07, groove: 0.585,
    trap: 0.058, clap: 0.091, clapPattern: "~ ~ pink ~", chop: 0.04, choir: 0.075, zaps: 0.04
  },
}
const funkRamp = (from, to, next) => familyRamp(funkAt, funkHarmony, from, to, next)

family('funk', {
  funk_60: block({ from: 60, ...funkHarmony, ...funkAt[60],
    next: { drift_a: 1, space: 1, funk_rise_80: 2 } }),
  funk_80: block({ from: 80, ...funkHarmony, ...funkAt[80],
    next: { depart_a: 1, rise_90: 1, space_80: 1, funk_rise_100: 2, funk_fall_60: 1 } }),
  funk_100: block({ from: 100, ...funkHarmony, ...funkAt[100],
    next: { cruise_a: 1, rise_110: 1, space_100: 1, funk_rise_120: 2, funk_fall_80: 1 } }),
  funk_120: block({ from: 120, ...funkHarmony, ...funkAt[120],
    next: { arrive_a: 1, fall_110: 1, space_120: 1, funk_fall_100: 2 } }),
  // Ramps between the funk pieces, 20 BPM each way.
  funk_rise_80: funkRamp(60, 80, { funk_80: 1 }),
  funk_rise_100: funkRamp(80, 100, { funk_100: 1 }),
  funk_rise_120: funkRamp(100, 120, { funk_120: 1 }),
  funk_fall_100: funkRamp(120, 100, { funk_100: 1 }),
  funk_fall_80: funkRamp(100, 80, { funk_80: 1 }),
  funk_fall_60: funkRamp(80, 60, { funk_60: 1 }),
}, [
  'funk_60', 'funk_rise_80', 'funk_80', 'funk_rise_100', 'funk_100', 'funk_rise_120', 'funk_120',
  'funk_fall_100', 'funk_100', 'funk_fall_80', 'funk_80', 'funk_fall_60', 'funk_60',
])
