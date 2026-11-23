// Star Trader — "Long Haul", the main menu music.
// An original composition for this game. Paste this file into https://strudel.cc
// and press play to hear a demo run through every block at one fixed tempo.
// The offline exporter applies each block's own tempo ramp.
//
// The game does not play one recording. It plays 8-bar blocks, choosing each
// next block at random from the ones the current block lists in `next` (with
// weights), so the music keeps changing over a long session, slowly rising
// from 60 to 120 BPM and settling back, sometimes drifting into a spacey
// interlude or a slow electric-guitar solo. `tools/music/render.mjs` exports each
// block (with its reverb tail) to assets/music/long-haul/ and writes the
// transitions to manifest.json; see tools/music/README.md.
//
// Every block plays the same 8-chord cycle, one chord per bar, in D minor:
// i - VI - III - VII, then i - VI - iv - V (sus, resolving on the next block).
// A block's tempo eases from `from` to `to` BPM over its 8 bars, and a block
// may only lead to blocks that start at the tempo it ends on.
//
// Only Strudel's built-in synthesizers, FM and noise are used: the guitar is
// synthesized with a plucked FM voice; only its solo uses pitch bends. There
// are no external samples, so the export is self-contained and works offline.
setcps(80 / 60 / 4)

// Every layer shares one reverb (Strudel keeps one per orbit, rebuilt
// whenever its settings change), so all use one room size, `hall`: the decay
// in seconds. Layers differ only in how much they send to it (`room`).
const hall = 4

// Strings that JavaScript edits use single quotes: Strudel parses double-quoted
// and backtick strings as patterns.
const chords = "<Dm9 Bb^7 F^7 C6 Dm9 Bb^7 Gm7 A7sus>"
const roots = "<d2 bb1 f1 c2 d2 bb1 g1 a1>"
// One octave above the bass: a warm, syncopated electric-piano pulse.
const liftRoots = "<d3 bb2 f2 c3 d3 bb2 g2 a2>"
// One octave up: "a4" -> "a5", "bb4" -> "bb5".
const octaveUp = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) + 1))

// Melodies: eight bars each, one per chord.
const melodyA = '[a4@2 d5 e5] [f5@3 [e5 d5]] [c5@2 a4 c5] g4 [a4@2 d5 e5] [f5@2 g5 f5] [e5@2 d5 bb4] a4'
const melodyB = '[d5@2 e5 f5] [d5@3 a4] [c5@2 e5 c5] [g4@2 a4 c5] [e5@2 f5 a5] [f5@3 d5] [bb4@2 d5 f5] [e5@3 d5]'
// A countermelody answering the lead in the gaps between its phrases.
const counterLine = '[~ ~ [f4 e4] [d4 a3]] [~ ~ [d4 f4] a4] [~ ~ [a4 g4] [f4 e4]] [[e4 g4] [a4 c5] [g4 e4] d4] [~ ~ [f4 a4] [c5 a4]] [~ ~ [d5 c5] [bb4 a4]] [~ ~ [g4 bb4] [d5 bb4]] [[a4 g4] [e4 d4] [e4 g4] a4]'
// A 16-bar solo in two blocks. Strummed chords carry it, with the melody as
// each chord's top note and rests between phrases. Three short note-by-note
// runs move at 1.8 times the eighth-note rate: eight notes over 5/9 of a bar.
const soloLineA = '[~ [d5,f5] ~ [f5,a5]] [[f5,bb5,d6]@3 ~] ' +
  '[[c5,f5,a5] ~ ~ [c5,f5,a5] ~ ~ [e5,a5,c6] ~] [[e5,g5,c6]@2 ~ ~] ' +
  '[~ [d5,f5,a5] ~ [f5,a5,d6]] [[bb5,d6,f6]@3 ~] ' +
  '[[[d5,g5,bb5] ~ ~ ~]@4 [g5 bb5 d6 g6 f6 d6 bb5 g5]@5] [[a5,d6,e6]@2 [g5,e6] ~]'
// The second half climbs higher; its final A resolves into the next block's D minor.
const soloLineB = '[[d5,f5,a5] ~ [d5,f5,a5] [f5,a5,d6] ~ [a5,d6,f6] [a5,d6,e6]@2] [[bb5,d6,f6]@3 ~] ' +
  '[[[c5,f5,a5] ~ ~ ~]@4 [a4 c5 f5 a5 c6 a5 f5 c5]@5] [[e5,g5,c6]@3 ~] ' +
  '[~ [d5,f5] [f5,a5] [a5,d6]] [[d6,f6]@3 ~] ' +
  '[[[d5,g5,bb5] ~ ~ ~]@4 [g5 bb5 d6 f6 g6 f6 d6 bb5]@5] [[a5,e6]@2 [a5,d6] [e5,a5]]'
// Bend depth in semitones for each solo event: whole- and half-step bends
// into the held chords, never inside a run.
const soloBendsA = '<0 2 0 0 0 1 0 [2@2 0 0]>'
const soloBendsB = '<0 2 0 1 0 2 0 [1@2 0 0]>'
// Rhythm guitar under every bar, voiced below the solo's runs: off-beat
// chord stabs under the quick runs, then a ringing chord and a re-strike under
// each held note. Each bar follows `chords`; A7sus keeps its suspended fourth.
const soloRhythmChords = '<' +
  '[~ [d3,a3,d4] ~ [d3,a3,f4]] [[bb2,f3,bb3,d4]@2 ~ [bb2,f3,d4]] ' +
  '[~ [f3,c4,f4] ~ [f3,c4,e4]] [[c3,g3,c4,e4]@2 ~ [c3,g3,e4]] ' +
  '[~ [d3,a3,d4] ~ [d3,a3,f4]] [[bb2,f3,bb3,d4]@2 ~ [bb2,f3,a3]] ' +
  '[~ [g3,d4,g4] ~ [g3,d4,f4]] [[a2,e3,a3,d4]@2 [a2,e3,g3,d4] [a2,e3,a3]]' +
  '>'
// A distinct 120 BPM theme: brass-like calls over a fast string-like pulse.
const fanfareLine = '[d5@2 f5 a5] [bb5@2 a5 f5] [a5@2 c6 a5] [g5@2 e5 c5] [a5@2 d6 c6] [bb5@2 d6 f6] [g5@2 bb5 d6] [e6@2 d6 a5]'
// Short phrases answer the synth melody; rests keep the lead from being busy.
const guitarLine = '[~ d4 ~ a4] [~ f4 ~ a4] [~ c5 ~ a4] [g4 ~ e4 ~] [~ d4 ~ f4] [~ a4 ~ c5] [~ bb4 ~ g4] [a4 ~ ~ ~]'
// Short orchestral-style answers; both use notes from the current chord.
const hornLine = '[~ d4@2 ~] [~ f4@2 ~] [~ c5@2 ~] [~ g4@2 ~] [~ f4@2 ~] [~ d4@2 ~] [~ bb3@2 ~] [~ e4@2 ~]'
const woodwindLine = '[~ ~ a4 g4] [~ ~ f4 d4] [~ ~ c5 a4] [~ ~ g4 e4] [~ ~ a4 f4] [~ ~ d5 c5] [~ ~ bb4 g4] [~ ~ e5 d5]'
const arpUp = '0 2 4 6 7 6 4 2 1 3 5 7 8 7 5 3'
const arpWide = '0 4 7 4 1 5 8 5 2 6 9 6 3 7 10 7'
// An asymmetric eight-note sequence voiced from each bar's chord. The rests
// leave room for the bells and for the more regular arpeggio that arrives later.
const spaceSequence = '0 ~ 4 ~ 2 ~ 5 ~ 1 ~ 4 ~ 3 ~ 6 ~'

const BARS = [0, 1, 2, 3, 4, 5, 6, 7]
// Eight per-bar values as a pattern string: '<a b c d e f g h>'.
const perBar = (values) => '<' + values.join(' ') + '>'
// A setting is a number, or [start, end] eased across the 8 bars.
const level = (value) =>
  Array.isArray(value) ? perBar(BARS.map((bar) => (value[0] + ((value[1] - value[0]) * bar) / 7).toFixed(4))) : value
const on = (value) => (Array.isArray(value) ? value[0] > 0 || value[1] > 0 : value > 0)
// Drums can play throughout (1), come in halfway ('in') or stop halfway ('out').
const drumMask = (drums) => (drums === 'in' ? '<0 0 0 0 1 1 1 1>' : drums === 'out' ? '<1 1 1 1 0 0 0 0>' : '1')

function block(settings) {
  const o = { keys: 0, ensemble: 0.16, bass: 0.55, arp: 0, arpNotes: arpUp, arpLpf: 1400,
    arpPhaserRate: 0.12, arpPhaserDepth: 0.12, arpPhaserSweep: 600,
    sequence: 0, lead: 0, leadNotes: melodyA,
    guitar: 0, guitarNotes: guitarLine, guitarBends: '',
    guitarDecay: 0.28, guitarRelease: 0.28, guitarLpf: 2400,
    guitarDelay: 0.18, guitarFeedback: 0.22, guitarRoom: 0.35,
    organ: 0, horns: 0, woodwinds: 0,
    counter: 0, stabs: 0, ostinato: 0, drums: 0, hats: 0, shaker: 0, lift: 0,
    ride: 0, toms: 0,
    bells: 0, stars: 0, starsSparse: 0.5, space: 0, ...settings }
  const from = o.from
  const to = o.to ?? o.from
  const bpm = BARS.map((bar) => from + ((to - from) * bar) / 8)
  // A dotted eighth (three sixteenths) in seconds at each bar's tempo.
  const echo = perBar(bpm.map((beats) => (45 / beats).toFixed(4)))
  const layers = []
  if (on(o.keys)) {
    // Short, off-beat FM keyboard chords replace the old held sawtooth pad.
    // The gaps let bass, guitar and percussion breathe between chord changes.
    layers.push(chord(chords).voicing()
      .struct("x ~ ~ [~ x]")
      .s("sine").fmi(1.8).fmh(2)
      .decay(0.38).sustain(0).release(0.12)
      .lpf(2400)
      .room(0.42).size(hall)
      .gain(level(o.keys)))
  }
  if (on(o.ensemble)) {
    // A moving midrange harmony fills the gaps without returning to a drone:
    // two short string-machine-like swells per bar, with clear rests.
    layers.push(chord(chords).voicing()
      .struct("x ~ x ~")
      .s("sawtooth")
      .attack(0.1).decay(0.55).sustain(0.12).release(0.24)
      .lpf(1450).lpq(0.8)
      .pan(0.58)
      .room(0.35).size(hall)
      .gain(level(o.ensemble)))
  }
  if (on(o.bass)) {
    layers.push(note(roots)
      .struct("x ~ x x ~ x ~ x")
      .s("sawtooth")
      .decay(0.22).sustain(0.35).release(0.1)
      .lpf(380).lpenv(2.5).lpdecay(0.12)
      .room(0.2).size(hall)
      .gain(level(o.bass)))
  }
  if (on(o.arp)) {
    layers.push(n(o.arpNotes)
      .chord(chords).voicing()
      .s("square")
      .decay(0.14).sustain(0)
      .lpf(level(o.arpLpf))
      .delay(0.35).delaytime(echo).delayfeedback(0.42)
      .phaser(o.arpPhaserRate).phaserdepth(o.arpPhaserDepth).phasersweep(o.arpPhaserSweep)
      .pan(sine.range(0.3, 0.7).slow(4))
      .room(0.6).size(hall)
      .gain(level(o.arp)))
  }
  if (on(o.sequence)) {
    // A restrained, moving analog-style sequence. It threads through the
    // space interlude and early ascent, then yields to the square arpeggio.
    layers.push(n(spaceSequence)
      .chord(chords).voicing()
      .s("triangle")
      .decay(0.19).sustain(0).release(0.08)
      .lpf(sine.range(850, 2300).slow(8))
      .phaser(0.18).phaserdepth(0.1).phasersweep(600)
      .delay(0.26).delaytime(echo).delayfeedback(0.36)
      .pan(sine.range(0.2, 0.8).slow(8))
      .room(0.58).size(hall)
      .gain(level(o.sequence)))
  }
  if (on(o.lift)) {
    // Root accents between bass notes; FM gives the ascending sections a
    // distinct electric-piano voice without competing with the lead melody.
    layers.push(note(liftRoots)
      .struct("~ x ~ [~ x] ~ x ~ x")
      .s("sine").fmi(1.4).fmh(2)
      .decay(0.22).sustain(0).release(0.08)
      .lpf(2300)
      .pan(0.28)
      .room(0.4).size(hall)
      .gain(level(o.lift)))
  }
  if (on(o.lead)) {
    layers.push(note('<' + o.leadNotes + '>')
      .s("sawtooth")
      .attack(0.04).decay(0.3).sustain(0.6).release(0.7)
      .lpf(2200).lpq(1)
      .delay(0.3).delaytime(echo).delayfeedback(0.35)
      .room(0.75).size(hall)
      .gain(level(o.lead)))
  }
  if (on(o.guitar)) {
    // A dry FM/triangle pluck replaces the wavy distorted saw. Only the solo
    // bends into selected notes (`guitarBends`); the other phrases hold pitch.
    let guitar = note('<' + o.guitarNotes + '>')
      .s("triangle").fmi(2).fmh(2.5)
      .attack(0.005).decay(o.guitarDecay).sustain(0.25).release(o.guitarRelease)
      .shape(0.08)
      .hpf(220).lpf(o.guitarLpf)
      .delay(o.guitarDelay).delaytime(echo).delayfeedback(o.guitarFeedback)
      .pan(0.42)
      .room(o.guitarRoom).size(hall)
      .gain(level(o.guitar))
    if (o.guitarBends) guitar = guitar.penv(o.guitarBends).pattack(0.17).pdecay(0.04)
    layers.push(guitar)
  }
  if (o.soloChords) {
    layers.push(note(soloRhythmChords)
      .s("triangle").fmi(1.4).fmh(2)
      .attack(0.01).decay(0.4).sustain(0.15).release(0.25)
      .shape(0.06).hpf(90).lpf(1700)
      .pan(0.32).room(0.16).size(hall)
      .gain(0.085))
  }
  if (on(o.organ)) {
    // Dry chordal organ accents add weight without a held low pad.
    layers.push(chord(chords).voicing()
      .struct("x ~ ~ [~ x]")
      .s("sine").fmi(0.7).fmh(2)
      .attack(0.06).decay(0.25).sustain(0.65).release(0.28)
      .lpf(1800)
      .pan(0.52).room(0.42).size(hall)
      .gain(level(o.organ)))
  }
  if (on(o.horns)) {
    // A warm synth-brass answer, with no phaser or vibrato.
    layers.push(note('<' + hornLine + '>')
      .s("sawtooth")
      .attack(0.12).decay(0.3).sustain(0.4).release(0.35)
      .lpf(1250).lpq(1.2)
      .pan(0.32).room(0.48).size(hall)
      .gain(level(o.horns)))
  }
  if (on(o.woodwinds)) {
    // A lighter high-register reply, also stable in pitch.
    layers.push(note('<' + woodwindLine + '>')
      .s("triangle").fmi(0.7).fmh(2)
      .attack(0.06).decay(0.4).sustain(0.32).release(0.25)
      .lpf(2300)
      .pan(0.68).room(0.4).size(hall)
      .gain(level(o.woodwinds)))
  }
  if (on(o.counter)) {
    layers.push(note('<' + counterLine + '>')
      .s("triangle")
      .decay(0.3).sustain(0.25).release(0.3)
      .lpf(2600)
      .delay(0.3).delaytime(echo).delayfeedback(0.3)
      .pan(0.35)
      .room(0.65).size(hall)
      .gain(level(o.counter)))
  }
  if (on(o.stabs)) {
    // Off-beat chord stabs that drive the fastest blocks.
    layers.push(chord(chords).voicing()
      .struct("~ x ~ [~ x] ~ x ~ x")
      .s("square")
      .decay(0.09).sustain(0)
      .lpf(2400)
      .pan(0.6)
      .room(0.5).size(hall)
      .gain(level(o.stabs)))
  }
  if (on(o.ostinato)) {
    // A clipped string-machine rhythm under the 120 BPM theme.
    layers.push(chord(chords).voicing()
      .struct("x ~ x x ~ x x ~")
      .s("sawtooth")
      .attack(0.02).decay(0.13).sustain(0).release(0.07)
      .lpf(1900).pan(0.55)
      .room(0.3).size(hall)
      .gain(level(o.ostinato)))
  }
  if (o.drums) {
    const mask = drumMask(o.drums)
    // Four even kicks; the former last-step pickup sounded like an accidental double hit.
    layers.push(note("g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ ~")
      .s("sine")
      .decay(0.28).sustain(0)
      .penv(30).pdecay(0.05)
      .room(0.15).size(hall)
      .gain(0.6)
      .mask(mask))
    layers.push(s("~ pink ~ pink")
      .decay(0.16).sustain(0)
      .bandf(1700).bandq(0.8)
      .room(0.45).size(hall)
      .gain(0.34)
      .mask(mask))
  }
  if (on(o.hats)) {
    layers.push(s("[~ white]*4")
      .decay(0.035).sustain(0)
      .hpf(8500)
      .pan(0.62)
      .room(0.3).size(hall)
      .gain(level(o.hats)))
  }
  if (on(o.shaker)) {
    // A dry high-mid pulse, distinct from the hats and lower snare.
    // Even spacing avoids a stray double hit in the rising sections.
    layers.push(s("~ pink ~ pink ~ pink ~ pink")
      .decay(0.065).sustain(0)
      .bandf(5100).bandq(1.4)
      .pan(0.72)
      .room(0.2).size(hall)
      .gain(level(o.shaker)))
  }
  if (on(o.ride)) {
    // Thin metallic offbeats; much softer than the main hats.
    layers.push(s("~ white ~ white")
      .decay(0.12).sustain(0)
      .bandf(7200).bandq(3)
      .pan(0.38).room(0.22).size(hall)
      .gain(level(o.ride)))
  }
  if (on(o.toms)) {
    // Two tuned accents per bar, without a kick pickup or double beat.
    layers.push(note("~ ~ d3 ~ ~ ~ a2 ~")
      .s("sine").penv(8).pdecay(0.07)
      .decay(0.16).sustain(0)
      .lpf(500).pan(0.6)
      .room(0.18).size(hall)
      .gain(level(o.toms)))
  }
  if (on(o.bells)) {
    // FM bells: sparse (about one every two bars), high and echoing.
    layers.push(n("<[0 ~ ~ ~] ~ [~ ~ 7 ~] ~ [4 ~ ~ ~] ~ [~ ~ ~ 2] ~>")
      .scale("D5:minor")
      .s("sine").fmi(2.2).fmh(3.5)
      .decay(1.2).sustain(0)
      .delay(0.4).delaytime(echo).delayfeedback(0.35)
      .pan(rand.range(0.15, 0.85))
      .room(0.8).size(hall)
      .gain(level(o.bells)))
  }
  if (on(o.stars)) {
    // Twinkling stars: random notes from the scale, some dropped.
    layers.push(n(irand(10).segment(16))
      .scale("D5:minor")
      .degradeBy(o.starsSparse)
      .s("triangle")
      .decay(0.12).sustain(0)
      .hpf(900).lpf(2600)
      .delay(0.4).delaytime(echo).delayfeedback(0.45)
      .pan(rand)
      .room(0.7).size(hall)
      .gain(level(o.stars)))
  }
  if (on(o.space)) {
    // Space interlude: spaced electric-key notes and short radio-noise ticks.
    layers.push(note("d3 ~ a3 ~ f4 ~ a3 ~")
      .s("sine").fmi(2.4).fmh(3)
      .decay(0.55).sustain(0).release(0.16)
      .lpf(1800)
      .delay(0.35).delaytime(echo).delayfeedback(0.3)
      .room(0.55).size(hall)
      .gain(level(o.space)))
    layers.push(s("pink ~ ~ ~ ~ pink ~ ~")
      .decay(0.14).sustain(0).release(0.06)
      .bandf(sine.range(300, 3200).slow(3)).bandq(7)
      .pan(sine.range(0.2, 0.8).slow(5))
      .room(0.6).size(hall)
      .gain(level(o.space)))
  }
  return { from, to, bpm, next: o.next, pattern: stack(...layers) }
}

const cruise = { keys: 0.13, ensemble: 0.13, arp: 0.13, arpLpf: 2200, lead: 0.19, guitar: 0.23,
  organ: 0.08, horns: 0.1, woodwinds: 0.07,
  counter: 0.12, drums: 1, hats: 0.22, shaker: 0.11, lift: 0.08,
  bells: 0.05, stars: 0.05, starsSparse: 0.8 }
const arrive = { keys: 0.1, ensemble: 0.13, arp: 0.13, arpLpf: 3200,
  arpPhaserRate: 0.25, arpPhaserDepth: 0.16, arpPhaserSweep: 450,
  lead: 0.19, guitar: 0.23, organ: 0.12, horns: 0.14, woodwinds: 0.08,
  counter: 0.12, stabs: 0.075, ostinato: 0.055, drums: 1,
  hats: 0.18, shaker: 0.16, ride: 0.055, toms: 0.07, lift: 0.12 }

// Shared by both halves of the solo.
const soloGuitar = { keys: 0.12, arpLpf: [1600, 2200], guitarDecay: 0.72, guitarRelease: 0.95,
  guitarLpf: 3600, guitarDelay: 0.07, guitarFeedback: 0.1, guitarRoom: 0.2, soloChords: true,
  drums: 1, hats: 0.13 }

window.musicStart = 'drift_a'
window.blocks = {
  // 60 BPM: drifting.
  drift_a: block({ from: 60, keys: 0.24, ensemble: 0.24, sequence: 0.1,
    bells: 0.16, stars: 0.1, starsSparse: 0.45,
    next: { drift_b: 2, space: 1, rise_70: 2 } }),
  drift_b: block({ from: 60, keys: 0.22, ensemble: 0.23, bass: 0.5, guitar: 0.16,
    sequence: 0.09, bells: 0.1, stars: 0.08,
    next: { drift_a: 1, space: 1, rise_70: 3 } }),
  space: block({ from: 60, keys: 0.17, ensemble: 0.29, bass: 0.28, space: 0.19, sequence: 0.12,
    bells: 0.13, stars: 0.07, starsSparse: 0.6,
    next: { drift_a: 2, drift_b: 1 } }),
  rise_70: block({ from: 60, to: 70, keys: [0.22, 0.2], ensemble: [0.23, 0.2],
    arp: [0, 0.07], sequence: [0.1, 0.09],
    shaker: [0, 0.08], lift: [0, 0.07], bells: 0.14, stars: 0.1,
    next: { rise_80: 2 } }),
  rise_80: block({ from: 70, to: 80, keys: [0.2, 0.18], ensemble: [0.2, 0.16],
    arp: [0.07, 0.13], sequence: [0.09, 0], hats: [0, 0.22],
    shaker: [0.08, 0.1], lift: [0.07, 0.09], bells: 0.12, stars: 0.1,
    next: { depart_a: 1, depart_b: 1 } }),
  // 80 BPM: departing.
  depart_a: block({ from: 80, keys: 0.18, arp: 0.13, guitar: 0.19, organ: 0.04, horns: 0.05,
    hats: 0.22, shaker: 0.09, lift: 0.07,
    bells: 0.13, stars: 0.1,
    next: { depart_b: 2, rise_90: 2 } }),
  depart_b: block({ from: 80, keys: 0.16, arp: 0.13, arpNotes: arpWide, guitar: 0.17,
    organ: 0.05, horns: 0.05, woodwinds: 0.06, counter: 0.12, hats: 0.22,
    shaker: 0.09, lift: 0.07, bells: 0.1,
    stars: 0.08, next: { depart_a: 1, rise_90: 2 } }),
  rise_90: block({ from: 80, to: 90, keys: [0.18, 0.14], arp: 0.13, arpLpf: [1400, 1800], guitar: [0.16, 0.22],
    organ: [0.05, 0.07], horns: [0.05, 0.08], woodwinds: [0.04, 0.06], hats: 0.22,
    shaker: [0.08, 0.11], lift: [0.08, 0.1],
    bells: 0.1, stars: 0.08, next: { rise_100: 2 } }),
  rise_100: block({ from: 90, to: 100, keys: [0.14, 0.1], arp: 0.13, arpLpf: [1800, 2200], guitar: 0.22,
    organ: [0.07, 0.08], horns: [0.08, 0.1], woodwinds: [0.06, 0.07], drums: 'in', hats: 0.22,
    shaker: [0.11, 0.14], lift: [0.1, 0.12],
    bells: [0.12, 0.05], stars: 0.07, next: { cruise_a: 2, cruise_b: 1 } }),
  // 100 BPM: cruising, with the melodies and the solo.
  cruise_a: block({ from: 100, ...cruise, next: { cruise_b: 2, solo: 1, rise_110: 1 } }),
  cruise_b: block({ from: 100, ...cruise, leadNotes: melodyB, next: { cruise_a: 1, solo: 2, rise_110: 1 } }),
  solo: block({ from: 100, ...soloGuitar, ensemble: [0.09, 0.12], arp: [0.05, 0.08],
    guitar: [0.22, 0.26], guitarNotes: soloLineA, guitarBends: soloBendsA,
    organ: [0.04, 0.08], horns: [0, 0.05], next: { solo_b: 1 } }),
  solo_b: block({ from: 100, ...soloGuitar, ensemble: [0.12, 0.14], arp: [0.08, 0.11],
    guitar: [0.26, 0.3], guitarNotes: soloLineB, guitarBends: soloBendsB,
    organ: [0.08, 0.11], horns: [0.05, 0.1],
    next: { cruise_a: 1, rise_110: 2, fall_90: 1 } }),
  rise_110: block({ from: 100, to: 110, ...cruise, arpLpf: [2200, 2700], stabs: [0, 0.04],
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
  arrive_b: block({ from: 120, ...arrive, leadNotes: octaveUp(melodyB), next: { arrive_a: 1, fanfare: 1, fall_110: 2 } }),
  fanfare: block({ from: 120, ...arrive, lead: 0.21, leadNotes: fanfareLine,
    horns: 0.18, organ: 0.14, woodwinds: 0.05, guitar: 0.11, counter: 0,
    arp: 0.1, stabs: 0.05, ostinato: 0.07,
    next: { arrive_a: 1, arrive_b: 1, fall_110: 2 } }),
  fall_110: block({ from: 120, to: 110, ...arrive, stabs: [0.075, 0.04],
    shaker: [0.16, 0], ostinato: [0.055, 0], ride: [0.055, 0], toms: [0.07, 0],
    lift: [0.12, 0], leadNotes: octaveUp(melodyB),
    next: { fall_100: 1 } }),
  fall_100: block({ from: 110, to: 100, ...cruise, keys: [0.1, 0.13], arpLpf: [3200, 2200], stabs: [0.04, 0],
    shaker: 0, lift: 0,
    bells: [0, 0.05], next: { solo: 1, cruise_a: 1, fall_90: 2 } }),
  // Back down through 90, 80, 70 to 60.
  fall_90: block({ from: 100, to: 90, keys: [0.13, 0.16], arp: 0.13, guitar: [0.21, 0.12], counter: [0.16, 0.08],
    organ: [0.08, 0.03], horns: [0.1, 0], woodwinds: [0.07, 0],
    drums: 'out', hats: 0.22, bells: 0.08, stars: 0.08, next: { fall_80: 1 } }),
  fall_80: block({ from: 90, to: 80, keys: [0.16, 0.18], arp: 0.13, arpLpf: [1800, 1400], guitar: [0.12, 0], counter: [0.08, 0],
    hats: 0.22, bells: [0.08, 0.13], stars: 0.1, next: { wind: 1 } }),
  wind: block({ from: 80, keys: 0.18, arp: 0.13, hats: 0.22,
    sequence: 0.035, bells: 0.13, stars: 0.1,
    next: { fall_70: 2, depart_a: 1 } }),
  fall_70: block({ from: 80, to: 70, keys: [0.18, 0.2], arp: [0.13, 0.07], sequence: [0.035, 0.06],
    hats: [0.22, 0],
    bells: 0.14, stars: 0.1, next: { fall_60: 1 } }),
  fall_60: block({ from: 70, to: 60, keys: [0.2, 0.22], arp: [0.07, 0],
    sequence: [0.06, 0.045],
    bells: 0.16, stars: 0.1, next: { drift_a: 1, space: 1 } }),
}

// strudel.cc: one pass through every block.
const demo = ['drift_a', 'drift_b', 'space', 'drift_a', 'rise_70', 'rise_80', 'depart_a', 'depart_b', 'rise_90',
  'rise_100', 'cruise_a', 'solo', 'solo_b', 'cruise_a', 'cruise_b', 'rise_110', 'rise_120', 'arrive_a', 'fanfare', 'arrive_b',
  'fall_110', 'fall_100', 'fall_90', 'fall_80', 'wind', 'fall_70', 'fall_60']
arrange(...demo.map((name) => [8, window.blocks[name].pattern]))
