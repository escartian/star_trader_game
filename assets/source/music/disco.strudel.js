// Star Trader music — the Space Disco pieces at 60, 80, 100 and 120 BPM and the
// 20 BPM ramps between them: the main theme reworked as space funk with a
// space-jazz colour (extended chords, Rhodes, vibraphone, muted trumpet and a
// swung ride) and a synth-disco sheen (supersaw pad, pumping chords, a
// filtered pluck, laser harp, zaps and beeps). Rendered
// with library.strudel.js; see tools/music/README.md.

// A funk vamp in D dorian with space-jazz extensions, two bars per chord:
// Dm11, G13 (its major IV), Dm11, then Bbmaj9 and A7sus, ending on A to
// resolve into every other block.
const discoHarmony = { chords: "<Dm11 Dm11 G13 G13 Dm11 Dm11 Bb^9 A7sus>", roots: "<d2 d2 g1 g1 d2 d2 bb1 a1>",
  rootNumbers: "<38 38 31 31 38 38 34 33>" }

// The Space Disco pieces have two themes, like the space pieces. Theme C, the main
// theme's melody (melodyA, one bar per chord), plays at 60 and 120 BPM with the
// same note values, so 120 is exactly double time: on a talkbox lead, doubled
// on clavinet at 120. Theme D, a syncopated clavinet riff, plays at 80 and 100.
const themeC = '<' + melodyA + '>'
// Theme D: sixteenth-note steps, one bar per chord. Over G9 it uses B natural,
// the dorian colour of funk.
const discoRiff = '<' +
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

// Space jazz: vibraphone arpeggios through the extended chords (ninths,
// elevenths and thirteenths), one bar per chord, eighth notes.
const vibesLine = '<' +
  '[a4 c5 e5 g5 a5 g5 e5 c5] [d5 e5 g5 c6 a5 g5 e5 d5] ' +
  '[b4 d5 e5 a5 b5 a5 e5 d5] [f5 a5 b5 e6 d6 b5 a5 f5] ' +
  '[a4 c5 e5 g5 a5 g5 e5 c5] [d5 e5 g5 c6 a5 g5 e5 d5] ' +
  '[d5 f5 a5 c6 d6 c6 a5 f5] [e5 g5 a5 d6 e6 d6 a5 g5]>'
// A muted trumpet answering in the second bar of each chord, in its gaps.
const trumpetLine = '<~ [~ ~ [a5 g5] [e5 d5]] ~ [~ ~ [d5 e5] [f5 a5]] ' +
  '~ [~ ~ [c6 a5] [g5 e5]] ~ [~ ~ [d5 e5] a5]>'

// Horn lines answering the melody, one bar per chord: pickups at the end of a
// bar and replies where the melody holds a long note.
const hornAnswers = '<~ [~ ~ ~ ~ ~ ~ [a4 c5] d5] ~ [~ [b4 d5] ~ [f5 e5] ~ d5 ~ ~] ' +
  '~ [~ ~ ~ ~ ~ ~ [f5 g5] a5] ~ [~ [g5 e5] ~ [d5 e5] ~ a4 ~ ~]>'

// The Space Disco pieces' settings, shared by the pieces and the ramps between them.
const discoAt = {
  // Theme C, slow and sultry space jazz: talkbox lead, Rhodes, vibraphone and
  // a muted trumpet over slap bass, a swung ride and the laser harp.
  60: {
    oohs: 0.075, moog: 0.06, glue: 0.5,
    keys: 0.13, ensemble: 0.15, bass: 0, slap: 0.16, slapNotes: slapLine, talk: 0.12,
    talkNotes: themeC, laser: 0.06, scratch: 0.04, groove: 0.3, swingRide: 0.025, pad: 0.09,
    halo: 0.075, vibes: 0.07, vibesNotes: vibesLine, trumpet: 0.08, trumpetNotes: trumpetLine,
    bells: 0.04, beeps: 0.03,
    pluck: 0.025
  },
  // Theme D: the clavinet riff with slap bass, chicken scratch and horn stabs,
  // the trumpet answering and vibraphone above.
  80: {
    oohs: 0.06, moog: 0.08, glue: 0.6, brassLine: 0.075, brassLineNotes: hornAnswers,
    keys: 0.1, ensemble: 0.12, bass: 0, slap: 0.18, slapNotes: slapLine, clav: 0.12,
    clavNotes: discoRiff, scratch: 0.06, brass: 0.07, groove: 0.455, hats: 0.06, swingRide: 0.02,
    organ: 0.06, laser: 0.03, pad: 0.06, vibes: 0.05, vibesNotes: vibesLine, trumpet: 0.08,
    trumpetNotes: trumpetLine,
    pump: 0.03,
    pluck: 0.035,
    openhat: 0.012
  },
  // Theme D, fuller: organ, congas and tambourine join the band.
  100: {
    oohs: 0.075, moog: 0.1, glue: 0.8, brassLine: 0.09, brassLineNotes: hornAnswers, openhat: 0.015,
    keys: 0.09, ensemble: 0.12, bass: 0, slap: 0.17, slapNotes: slapLine, clav: 0.12,
    clavNotes: discoRiff, scratch: 0.06, brass: 0.09, organ: 0.07, groove: 0.52, hats: 0.06,
    swingRide: 0.02, congas: 0.08, tambourine: 0.04, laser: 0.03, vibes: 0.04,
    vibesNotes: vibesLine, trumpet: 0.07, trumpetNotes: trumpetLine,
    pad: 0.05,
    pump: 0.045,
    pluck: 0.04,
    zaps: 0.02,
    beeps: 0.02
  },
  // Theme C at double time: talkbox and clavinet in unison over the full band,
  // horns answering, a held choir and a Moog bass under the slap bass, congas,
  // tambourine, open hats and the swung ride, glued by more reverb and a
  // slapback echo, with the laser harp above.
  120: {
    oohs: 0.09, moog: 0.12, glue: 1, brassLine: 0.12, brassLineNotes: hornAnswers, openhat: 0.02,
    keys: 0.09, ensemble: 0.09, bass: 0, slap: 0.17, slapNotes: slapLine, talk: 0.11,
    talkNotes: themeC, clav: 0.08, clavNotes: themeC, scratch: 0.07, brass: 0.1, organ: 0.06,
    groove: 0.585, hats: 0.06, swingRide: 0.025, congas: 0.12, tambourine: 0.06, laser: 0.08,
    vibes: 0.03, vibesNotes: vibesLine, trumpet: 0.06, trumpetNotes: trumpetLine,
    pad: 0.05,
    pump: 0.055,
    pluck: 0.045,
    zaps: 0.025,
    beeps: 0.02
  },
}
const discoRamp = (from, to, next) => familyRamp(discoAt, discoHarmony, from, to, next)

family('disco', {
  disco_60: block({ from: 60, ...discoHarmony, ...discoAt[60], next: { disco_rise_80: 2 } }),
  disco_80: block({ from: 80, ...discoHarmony, ...discoAt[80], next: { disco_rise_100: 2, disco_fall_60: 1 } }),
  disco_100: block({ from: 100, ...discoHarmony, ...discoAt[100], next: { disco_rise_120: 2, disco_fall_80: 1 } }),
  disco_120: block({ from: 120, ...discoHarmony, ...discoAt[120], next: { disco_fall_100: 2 } }),
  // Ramps between the Space Disco pieces, 20 BPM each way.
  disco_rise_80: discoRamp(60, 80, { disco_80: 1 }),
  disco_rise_100: discoRamp(80, 100, { disco_100: 1 }),
  disco_rise_120: discoRamp(100, 120, { disco_120: 1 }),
  disco_fall_100: discoRamp(120, 100, { disco_100: 1 }),
  disco_fall_80: discoRamp(100, 80, { disco_80: 1 }),
  disco_fall_60: discoRamp(80, 60, { disco_60: 1 }),
}, [
  'disco_60', 'disco_rise_80', 'disco_80', 'disco_rise_100', 'disco_100', 'disco_rise_120', 'disco_120',
  'disco_fall_100', 'disco_100', 'disco_fall_80', 'disco_80', 'disco_fall_60', 'disco_60',
])
