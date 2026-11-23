// Star Trader — "Long Haul", the main menu music.
// An original composition for this game. Paste this file into https://strudel.cc
// and press play to hear a demo run through every block at one fixed tempo.
// The offline exporter applies each block's own tempo ramp.
//
// The game does not play one recording. It plays blocks of bars (8 unless a
// block sets `bars`, such as the 16-bar solo), choosing each
// next block at random from the ones the current block lists in `next` (with
// weights), so the music keeps changing over a long session, slowly rising
// from 60 to 120 BPM and settling back, sometimes drifting into a spacey
// interlude or a slow electric-guitar solo. `tools/music/render.mjs` exports each
// block (with its reverb tail) to assets/music/long-haul/ and writes the
// transitions to manifest.json; see tools/music/README.md.
//
// Most blocks play the same 8-chord cycle, one chord per bar, in D minor:
// i - VI - III - VII, then i - VI - iv - V (sus, resolving on the next block).
// The space pieces (60, 80, 100 and 120 BPM, mostly inspired by space-opera
// albums) hold each chord for two bars instead: i - VI - iv - V. Each faster
// one adds instruments and percussion, so the motion builds with the tempo.
// A block's tempo eases from `from` to `to` BPM over its bars, and a block
// may only lead to blocks that start at the tempo it ends on.
//
// The sound mixes Strudel's own synthesizers (a Jarre-style phased supersaw
// pad, pumping supersaw chords and a filtered sawtooth pluck, plus the drums
// and noise effects) with General MIDI instruments (FluidR3_GM, MIT licence),
// including a bowed viola line. Only the guitar
// solo uses pitch bends. The exporter serves the instrument data from
// assets/source/music/soundfonts/, so renders work offline. On strudel.cc the
// same sounds load from Strudel's own soundfont host.
setcps(80 / 60 / 4)

// Every layer shares one reverb (Strudel keeps one per orbit, rebuilt
// whenever its settings change), so all use one room size, `hall`: the decay
// in seconds. Layers differ only in how much they send to it (`room`).
const hall = 4
// The space layers send to a second, longer reverb on orbit 2 (seconds).
const deep = 7

// Strudel's General MIDI instruments, each in its FluidR3_GM version (MIT
// licence; the `:n` index selects it). `boost` (a postgain) brings each sample
// to the RMS level the earlier synth voice had at the same gain, measured
// against a fixed reference tone, so the levels in the blocks keep their balance.
const instruments = {
  keys: { sound: "gm_epiano1:1", boost: 1.3 },
  ensemble: { sound: "gm_synth_strings_1:3", boost: 1.6 },
  bass: { sound: "gm_synth_bass_1:1", boost: 0.85 },
  arp: { sound: "gm_lead_1_square:2", boost: 1.25 },
  sequence: { sound: "gm_lead_4_chiff:2", boost: 2 },
  lift: { sound: "gm_epiano2:1", boost: 1.7 },
  lead: { sound: "gm_lead_2_sawtooth:3", boost: 1.2 },
  guitar: { sound: "gm_electric_guitar_clean:2", boost: 3 },
  organ: { sound: "gm_drawbar_organ:3", boost: 3.5 },
  horns: { sound: "gm_french_horn:1", boost: 1.15 },
  woodwinds: { sound: "gm_flute:2", boost: 1.4 },
  counter: { sound: "gm_vibraphone:3", boost: 1.6 },
  stabs: { sound: "gm_synth_brass_1:1", boost: 1.8 },
  ostinato: { sound: "gm_string_ensemble_1:2", boost: 4 },
  toms: { sound: "gm_melodic_tom:3", boost: 2 },
  bells: { sound: "gm_celesta:3", boost: 2.9 },
  stars: { sound: "gm_music_box:3", boost: 1.5 },
  space: { sound: "gm_fx_atmosphere:3", boost: 1.8 },
  viola: { sound: "gm_viola:1", boost: 0.7 },
  // Strudel synthesizers, levelled like the ensemble, stabs and arpeggio.
  pad: { sound: "supersaw", boost: 0.57 },
  pump: { sound: "supersaw", boost: 0.76 },
  pluck: { sound: "sawtooth", boost: 1.75 },
  // The space pieces. Sparse voices (crystal, beeps, zaps) are boosted less
  // than their average level would suggest, so single notes do not jump out.
  halo: { sound: "gm_pad_halo:1", boost: 0.88 },
  choir: { sound: "gm_synth_choir:2", boost: 0.68 },
  crystal: { sound: "gm_fx_crystal:4", boost: 2 },
  beeps: { sound: "square", boost: 1 },
  boops: { sound: "triangle", boost: 1 },
  hook: { sound: "supersaw", boost: 2.15 },
  hookLow: { sound: "square", boost: 1.75 },
  hookEcho: { sound: "sawtooth", boost: 1.75 },
  seqbass: { sound: "sawtooth", boost: 1.25 },
  zaps: { sound: "sawtooth", boost: 2.5 },
}
const play = (pattern, name) => pattern.s(instruments[name].sound).postgain(instruments[name].boost)

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
// Guitar notation. A chord `[a,b,c]` is strummed: each later string starts
// 1/STRUM of the chord's span after the one before. `d6^2` bends up into d6
// from two semitones below; `bendsOf` turns those marks into a `penv` pattern
// with one value per note or chord, matching the notes' structure.
const STRUM = 32
const chordMark = /\[([^\[\]]*,[^\[\]]*)\]/g
const strum = (text) => text.replace(chordMark, (_, inner) => '[' + inner.split(',')
  .map((note, string) => (string ? '[~@' + string + ' ' + note.trim() + '@' + (STRUM - string) + ']' : note.trim())).join(', ') + ']')
const notesOf = (text) => text.replace(/\^\d/g, '')
const bendsOf = (text) => text
  .replace(chordMark, (_, inner) => String(Math.max(0, ...[...inner.matchAll(/\^(\d)/g)].map((m) => Number(m[1])))))
  .replace(/[a-g]b?\d(\^(\d))?/g, (_, mark, bend) => bend ?? '0')
// A 16-bar solo: a singing single-note line with six bends, strummed chords
// and double-stops at the phrase peaks, rests to breathe, and three short runs
// at 1.8 times the eighth-note rate (eight notes over 5/9 of a bar). The second
// half climbs higher; its final A resolves into the next block's D minor.
const solo = '[~ a4 c5 d5] [f5^2@3 ~] [e5^1@2 c5 a4] [[g4,c5,e5]@2 ~ ~] ' +
  '[~ d5 f5 a5] [d6^2@2 c6 a5] [[[d5,g5,bb5] ~ ~ ~]@4 [g5 bb5 d6 f6 g6 f6 d6 bb5]@5] [a5^1@3 ~] ' +
  '[[d5,f5,a5] ~ [d5,f5,a5] ~ ~ [f5,a5,d6] ~ ~] [[bb5,d6]@3 [a5 f5]] ' +
  '[[[c5,f5,a5] ~ ~ ~]@4 [a4 c5 f5 a5 c6 e6 c6 a5]@5] [c6^2@3 ~] ' +
  '[~ [a5,d6] [c6,e6] [d6,f6]] [f6^2@3 ~] ' +
  '[[[d6,g6] ~ ~ ~]@4 [g6 f6 d6 bb5 a5 g5 f5 d5]@5] [[a4,d5,e5]@2 [g4,d5] [e4,a4]]'
// Rhythm guitar, voiced below the lead and the solo's runs: off-beat chord
// stabs in odd bars, then a ringing chord and a re-strike in even bars. It
// enters quietly in cruise so the solo's chordal guitar is not a sudden change.
// Each bar follows `chords`; A7sus keeps its suspended fourth.
const rhythmChords = '<' +
  '[~ [d3,a3,d4] ~ [d3,a3,f4]] [[bb2,f3,bb3,d4]@2 ~ [bb2,f3,d4]] ' +
  '[~ [f3,c4,f4] ~ [f3,c4,e4]] [[c3,g3,c4,e4]@2 ~ [c3,g3,e4]] ' +
  '[~ [d3,a3,d4] ~ [d3,a3,f4]] [[bb2,f3,bb3,d4]@2 ~ [bb2,f3,a3]] ' +
  '[~ [g3,d4,g4] ~ [g3,d4,f4]] [[a2,e3,a3,d4]@2 [a2,e3,g3,d4] [a2,e3,a3]]' +
  '>'
// A distinct 120 BPM theme: brass-like calls over a fast string-like pulse.
const fanfareLine = '[d5@2 f5 a5] [bb5@2 a5 f5] [a5@2 c6 a5] [g5@2 e5 c5] [a5@2 d6 c6] [bb5@2 d6 f6] [g5@2 bb5 d6] [e6@2 d6 a5]'
// Short phrases answer the synth melody; rests keep the lead from being busy.
const guitarLine = '[~ d4 ~ a4] [~ f4 ~ a4] [~ c5 ~ a4] [g4 ~ e4 ~] [~ d4 ~ f4] [~ a4 ~ c5] [~ bb4 ~ g4] [a4 ~ ~ ~]'
// The same answers as two-note strums, foreshadowing and echoing the solo.
const guitarStrums = '[~ [f4,a4] ~ [a4,d5]] [~ [f4,bb4] ~ [d4,f4]] [~ [a4,c5] ~ [c5,f5]] [[e4,g4] ~ [c4,e4] ~] ' +
  '[~ [d4,f4] ~ [f4,a4]] [~ [bb4,d5] ~ [f4,bb4]] [~ [bb4,d5] ~ [g4,bb4]] [[a4,d5] ~ ~ ~]'
// A slow bowed viola line on each chord's guide tones, mostly one note a bar.
const violaLine = '[f4@3 e4] d4 [c4@3 e4] [g4@2 e4@2] [f4@3 a4] [d4@3 f4] [bb3@2 d4@2] [e4@3 d4]'
// A deadmau5-style pluck figure, voiced from each bar's chord.
const pluckNotes = '0 4 7 4 2 4 7 9 0 4 7 4 2 4 6 4'
// Short orchestral-style answers; both use notes from the current chord.
const hornLine = '[~ d4@2 ~] [~ f4@2 ~] [~ c5@2 ~] [~ g4@2 ~] [~ f4@2 ~] [~ d4@2 ~] [~ bb3@2 ~] [~ e4@2 ~]'
const woodwindLine = '[~ ~ a4 g4] [~ ~ f4 d4] [~ ~ c5 a4] [~ ~ g4 e4] [~ ~ a4 f4] [~ ~ d5 c5] [~ ~ bb4 g4] [~ ~ e5 d5]'
const arpUp = '0 2 4 6 7 6 4 2 1 3 5 7 8 7 5 3'
const arpWide = '0 4 7 4 1 5 8 5 2 6 9 6 3 7 10 7'
// An asymmetric eight-note sequence voiced from each bar's chord. The rests
// leave room for the bells and for the more regular arpeggio that arrives later.
const spaceSequence = '0 ~ 4 ~ 2 ~ 5 ~ 1 ~ 4 ~ 3 ~ 6 ~'

// The space pieces: two bars per chord, ending on A to resolve like the others.
const spaceHarmony = { chords: "<Dm9 Dm9 Bb^7 Bb^7 Gm9 Gm9 A7sus A7sus>", roots: "<d2 d2 bb1 bb1 g1 g1 a1 a1>" }
// The sequencer's roots as MIDI numbers (d2, bb1, g1, a1), for adding steps.
const spaceRootNumbers = "<38 38 34 34 31 31 33 33>"
// The space hook: one two-bar phrase per chord, eight eighth-note steps a
// bar. It keeps these note values in every space piece, so the tempo alone
// speeds it up: a quiet nod at 60 BPM, then 80 and 100, and at 120 exactly
// double time, where an echo canon fills its rests.
const hookPhrases = [
  ['d5 ~ a5 ~ f5 e5 ~ d5', 'e5@2 f5 ~ a5@3 ~'], // Dm9
  ['d5 ~ f5 ~ a5 bb5 ~ a5', 'f5@2 d5 ~ c5@3 ~'], // Bb^7
  ['bb4 ~ d5 ~ f5 a5 ~ g5', 'a5@2 bb5 ~ d6@3 ~'], // Gm9
  ['e5 ~ a5 ~ g5 e5 ~ d5', 'e5@2 g5 ~ a5@3 ~'], // A7sus, resolving to D
]
const hookNotes = '<' + hookPhrases.flatMap((phrase) => phrase.map((bar) => '[' + bar + ']')).join(' ') + '>'
// One octave down: "a5" -> "a4".
const octaveDown = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) - 1))
// Computer beeps (square, high) and boops (triangle, low): steady pitches from
// D minor pentatonic in short bursts, like ship telemetry.
const beepNotes = "<[d6 d6 ~ a6] ~ [~ c7 c7 ~] ~ [f6 ~ f6 ~] ~ [~ ~ g6 a6] ~>"
const boopNotes = "<~ [~ ~ d4 ~] ~ [a3 ~ ~ ~] ~ [~ f4 ~ ~] ~ [~ ~ ~ [d4 a3]]>"
// Sequencer steps above the root, in semitones: root, octave and fifth.
const eighthSteps = '[0 12 7 12]*2'
const sixteenthSteps = '[0 12 7 12]*4'

// One value per bar as a pattern string: '<a b c ...>'.
const perBar = (values) => '<' + values.join(' ') + '>'
const on = (value) => (Array.isArray(value) ? value[0] > 0 || value[1] > 0 : value > 0)
// Drums can play throughout (1), come in halfway ('in') or stop halfway ('out').
const drumMask = (drums, bars) => (drums === 'in' || drums === 'out'
  ? perBar([...Array(bars).keys()].map((bar) => Number((bar >= bars / 2) === (drums === 'in'))))
  : '1')

function block(settings) {
  const o = { keys: 0, ensemble: 0.16, bass: 0.55, arp: 0, arpNotes: arpUp, arpLpf: 1400,
    arpPhaserRate: 0.12, arpPhaserDepth: 0.12, arpPhaserSweep: 600,
    sequence: 0, lead: 0, leadNotes: melodyA,
    guitar: 0, guitarNotes: guitarLine, guitarBends: false, rhythm: 0,
    guitarDecay: 0.4, guitarSustain: 0.35, guitarRelease: 0.3, guitarLpf: 2400,
    guitarDelay: 0.18, guitarFeedback: 0.22, guitarRoom: 0.35,
    organ: 0, horns: 0, woodwinds: 0,
    counter: 0, stabs: 0, ostinato: 0, drums: 0, hats: 0, shaker: 0, lift: 0,
    ride: 0, toms: 0,
    bells: 0, stars: 0, starsSparse: 0.5, space: 0,
    pad: 0, pump: 0, pluck: 0, viola: 0,
    chords, roots, halo: 0, choir: 0, crystal: 0, beeps: 0, seqbass: 0, seqSteps: eighthSteps,
    hook: 0, hookLow: 0, hookEcho: 0,
    machine: 0, zaps: 0, ...settings }
  const from = o.from
  const to = o.to ?? o.from
  const bars = [...Array(o.bars ?? 8).keys()]
  const bpm = bars.map((bar) => from + ((to - from) * bar) / bars.length)
  // A setting is a number, or [start, end] eased across the block's bars.
  const level = (value) => (Array.isArray(value)
    ? perBar(bars.map((bar) => (value[0] + ((value[1] - value[0]) * bar) / (bars.length - 1)).toFixed(4)))
    : value)
  // A dotted eighth (three sixteenths) in seconds at each bar's tempo.
  const echo = perBar(bpm.map((beats) => (45 / beats).toFixed(4)))
  const layers = []
  if (on(o.keys)) {
    // Short, off-beat electric-piano chords instead of a held pad.
    // The gaps let bass, guitar and percussion breathe between chord changes.
    layers.push(play(chord(o.chords).voicing(), 'keys')
      .struct("x ~ ~ [~ x]")
      .decay(0.6).sustain(0.15).release(0.2)
      .lpf(3000)
      .room(0.42).size(hall)
      .gain(level(o.keys)))
  }
  if (on(o.ensemble)) {
    // A moving midrange harmony fills the gaps without returning to a drone:
    // two short synth-string swells per bar, with clear rests.
    layers.push(play(chord(o.chords).voicing(), 'ensemble')
      .struct("x ~ x ~")
      .attack(0.1).decay(0.55).sustain(0.3).release(0.3)
      .lpf(1800)
      .pan(0.58)
      .room(0.35).size(hall)
      .gain(level(o.ensemble)))
  }
  if (on(o.bass)) {
    layers.push(play(note(o.roots), 'bass')
      .struct("x ~ x x ~ x ~ x")
      .decay(0.22).sustain(0.35).release(0.1)
      .lpf(700)
      .room(0.2).size(hall)
      .gain(level(o.bass)))
  }
  if (on(o.arp)) {
    layers.push(play(n(o.arpNotes).chord(o.chords).voicing(), 'arp')
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
    layers.push(play(n(spaceSequence).chord(o.chords).voicing(), 'sequence')
      .decay(0.19).sustain(0).release(0.08)
      .lpf(sine.range(850, 2300).slow(8))
      .phaser(0.18).phaserdepth(0.1).phasersweep(600)
      .delay(0.26).delaytime(echo).delayfeedback(0.36)
      .pan(sine.range(0.2, 0.8).slow(8))
      .room(0.58).size(hall)
      .gain(level(o.sequence)))
  }
  if (on(o.lift)) {
    // Root accents between bass notes; a second, FM-style electric piano gives
    // the ascending sections a distinct voice without competing with the lead.
    layers.push(play(note(liftRoots), 'lift')
      .struct("~ x ~ [~ x] ~ x ~ x")
      .decay(0.3).sustain(0).release(0.1)
      .lpf(2300)
      .pan(0.28)
      .room(0.4).size(hall)
      .gain(level(o.lift)))
  }
  if (on(o.lead)) {
    layers.push(play(note('<' + o.leadNotes + '>'), 'lead')
      .attack(0.04).decay(0.3).sustain(0.6).release(0.7)
      .lpf(2600)
      .delay(0.3).delaytime(echo).delayfeedback(0.35)
      .room(0.75).size(hall)
      .gain(level(o.lead)))
  }
  if (on(o.guitar)) {
    // Only the solo bends into selected notes (`guitarBends`); the other
    // phrases hold pitch.
    let guitar = play(note(strum('<' + notesOf(o.guitarNotes) + '>')), 'guitar')
      .attack(0.005).decay(o.guitarDecay).sustain(o.guitarSustain).release(o.guitarRelease)
      .hpf(220).lpf(o.guitarLpf)
      .delay(o.guitarDelay).delaytime(echo).delayfeedback(o.guitarFeedback)
      .pan(0.42)
      .room(o.guitarRoom).size(hall)
      .gain(level(o.guitar))
    // A bend starts `penv` semitones below the written note and rises into it.
    if (o.guitarBends) guitar = guitar.penv('<' + bendsOf(o.guitarNotes) + '>').pattack(0.14)
    layers.push(guitar)
  }
  if (on(o.rhythm)) {
    layers.push(play(note(strum(rhythmChords)), 'guitar')
      .attack(0.01).decay(0.4).sustain(0.2).release(0.25)
      .hpf(90).lpf(1700)
      .pan(0.32).room(0.16).size(hall)
      .gain(level(o.rhythm)))
  }
  if (on(o.pad)) {
    // A space-opera string-machine pad: one swell a bar with a gap before the
    // next chord, through a slow phaser and a resonant filter sweep.
    layers.push(play(chord(o.chords).voicing(), 'pad')
      .struct("x@3 ~")
      .attack(0.7).decay(0.8).sustain(0.5).release(0.9)
      .lpf(sine.range(700, 2100).slow(16)).lpq(4)
      .phaser(0.15).phaserdepth(0.45).phasersweep(900)
      .pan(sine.range(0.35, 0.65).slow(12))
      .room(0.65).size(hall)
      .gain(level(o.pad)))
  }
  if (on(o.pump)) {
    // French-house pumping chords: each beat's chord swells in after the kick,
    // like a sidechained pad, under a slow filter sweep.
    layers.push(play(chord(o.chords).voicing(), 'pump')
      .struct("x x x x")
      .attack(0.13).decay(0.2).sustain(0).release(0.08)
      .lpf(sine.range(900, 3200).slow(8)).lpq(2)
      .pan(0.5).room(0.3).size(hall)
      .gain(level(o.pump)))
  }
  if (on(o.pluck)) {
    // A progressive-house pluck: short sawtooth notes opened by a quick
    // filter envelope, echoing on the dotted eighth.
    layers.push(play(n(pluckNotes).chord(o.chords).voicing(), 'pluck')
      .decay(0.12).sustain(0).release(0.05)
      .lpf(500).lpenv(4).lpdecay(0.09).lpq(6)
      .delay(0.3).delaytime(echo).delayfeedback(0.4)
      .pan(sine.range(0.25, 0.75).slow(2))
      .room(0.45).size(hall)
      .gain(level(o.pluck)))
  }
  if (on(o.viola)) {
    // Bowed viola: slow attack and long release so the line sings.
    layers.push(play(note('<' + violaLine + '>'), 'viola')
      .attack(0.18).decay(0.5).sustain(0.7).release(0.6)
      .lpf(3200)
      .pan(0.62).room(0.5).size(hall)
      .gain(level(o.viola)))
  }
  if (on(o.organ)) {
    // Dry chordal organ accents add weight without a held low pad.
    layers.push(play(chord(o.chords).voicing(), 'organ')
      .struct("x ~ ~ [~ x]")
      .attack(0.06).decay(0.25).sustain(0.65).release(0.28)
      .lpf(2400)
      .pan(0.52).room(0.42).size(hall)
      .gain(level(o.organ)))
  }
  if (on(o.horns)) {
    // A warm French-horn answer, with no phaser or vibrato.
    layers.push(play(note('<' + hornLine + '>'), 'horns')
      .attack(0.08).decay(0.3).sustain(0.6).release(0.35)
      .lpf(2000)
      .pan(0.32).room(0.48).size(hall)
      .gain(level(o.horns)))
  }
  if (on(o.woodwinds)) {
    // A lighter high-register flute reply, also stable in pitch.
    layers.push(play(note('<' + woodwindLine + '>'), 'woodwinds')
      .attack(0.04).decay(0.4).sustain(0.5).release(0.25)
      .lpf(2300)
      .pan(0.68).room(0.4).size(hall)
      .gain(level(o.woodwinds)))
  }
  if (on(o.counter)) {
    layers.push(play(note('<' + counterLine + '>'), 'counter')
      .decay(0.8).sustain(0.1).release(0.3)
      .lpf(2600)
      .delay(0.3).delaytime(echo).delayfeedback(0.3)
      .pan(0.35)
      .room(0.65).size(hall)
      .gain(level(o.counter)))
  }
  if (on(o.stabs)) {
    // Off-beat synth-brass stabs that drive the fastest blocks.
    layers.push(play(chord(o.chords).voicing(), 'stabs')
      .struct("~ x ~ [~ x] ~ x ~ x")
      .decay(0.12).sustain(0)
      .lpf(2400)
      .pan(0.6)
      .room(0.5).size(hall)
      .gain(level(o.stabs)))
  }
  if (on(o.ostinato)) {
    // A clipped string-section rhythm under the 120 BPM theme.
    layers.push(play(chord(o.chords).voicing(), 'ostinato')
      .struct("x ~ x x ~ x x ~")
      .attack(0.02).decay(0.13).sustain(0).release(0.07)
      .lpf(1900).pan(0.55)
      .room(0.3).size(hall)
      .gain(level(o.ostinato)))
  }
  if (o.drums) {
    const mask = drumMask(o.drums, bars.length)
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
    layers.push(play(note("~ ~ d3 ~ ~ ~ a2 ~"), 'toms')
      .decay(0.3).sustain(0)
      .lpf(500).pan(0.6)
      .room(0.18).size(hall)
      .gain(level(o.toms)))
  }
  if (on(o.bells)) {
    // Celesta bells: sparse (about one every two bars), high and echoing.
    layers.push(play(n("<[0 ~ ~ ~] ~ [~ ~ 7 ~] ~ [4 ~ ~ ~] ~ [~ ~ ~ 2] ~>").scale("D5:minor"), 'bells')
      .decay(1.2).sustain(0)
      .delay(0.4).delaytime(echo).delayfeedback(0.35)
      .pan(rand.range(0.15, 0.85))
      .room(0.8).size(hall)
      .gain(level(o.bells)))
  }
  if (on(o.stars)) {
    // Twinkling music-box stars: random notes from the scale, some dropped.
    layers.push(play(n(irand(10).segment(16)).scale("D5:minor"), 'stars')
      .degradeBy(o.starsSparse)
      .decay(0.25).sustain(0)
      .hpf(900).lpf(2600)
      .delay(0.4).delaytime(echo).delayfeedback(0.45)
      .pan(rand)
      .room(0.7).size(hall)
      .gain(level(o.stars)))
  }
  if (on(o.space)) {
    // Space interlude: spaced atmosphere notes in the deep reverb, over a
    // faint, slowly tuned radio static.
    layers.push(play(note("d3 ~ a3 ~ f4 ~ a3 ~"), 'space')
      .decay(1.6).sustain(0).release(0.6)
      .lpf(1800)
      .delay(0.45).delaytime(echo).delayfeedback(0.45)
      .pan(sine.range(0.3, 0.7).slow(6))
      .orbit(2).room(0.75).size(deep)
      .gain(level(o.space)))
    layers.push(s("pink ~ ~ ~ ~ pink ~ ~")
      .decay(0.3).sustain(0).release(0.2)
      .bandf(sine.range(400, 2600).slow(7)).bandq(3)
      .pan(sine.range(0.2, 0.8).slow(5))
      .room(0.6).size(hall)
      .gain(level(o.space)).postgain(0.35))
  }
  if (on(o.halo)) {
    // A slow halo pad that rises and fades with each chord, far in the reverb.
    layers.push(play(chord(o.chords).voicing(), 'halo')
      .attack(1.2).decay(1).sustain(0.6).release(2)
      .lpf(2600)
      .pan(sine.range(0.3, 0.7).slow(10))
      .orbit(2).room(0.8).size(deep)
      .gain(level(o.halo)))
  }
  if (on(o.choir)) {
    // A synth choir swelling once a bar under the faster space pieces.
    layers.push(play(chord(o.chords).voicing(), 'choir')
      .struct("x@3 ~")
      .attack(0.6).decay(0.8).sustain(0.6).release(1)
      .lpf(3000)
      .orbit(2).room(0.6).size(deep)
      .gain(level(o.choir)))
  }
  if (on(o.crystal)) {
    // Glassy shimmer, a note every other half bar or so.
    layers.push(play(n("<[0 ~] ~ [~ 4] ~ [2 ~] ~ [~ 7] ~>").scale("D5:minor"), 'crystal')
      .decay(2).sustain(0).release(1)
      .delay(0.4).delaytime(echo).delayfeedback(0.45)
      .pan(rand.range(0.2, 0.8))
      .orbit(2).room(0.8).size(deep)
      .gain(level(o.crystal)))
  }
  if (on(o.beeps)) {
    // Beeps and boops: short steady tones, no pitch sweeps.
    layers.push(play(note(beepNotes), 'beeps')
      .attack(0.003).decay(0.05).sustain(0.4).release(0.02)
      .lpf(3800)
      .delay(0.3).delaytime(echo).delayfeedback(0.35)
      .pan(rand.range(0.2, 0.8))
      .orbit(2).room(0.5).size(deep)
      .gain(level(o.beeps)))
    layers.push(play(note(boopNotes), 'boops')
      .attack(0.005).decay(0.12).sustain(0.2).release(0.05)
      .lpf(1400)
      .delay(0.25).delaytime(echo).delayfeedback(0.3)
      .pan(rand.range(0.3, 0.7))
      .orbit(2).room(0.45).size(deep)
      .gain(level(o.beeps)))
  }
  if (on(o.hook)) {
    // The space hook on a phased supersaw lead, with a square an octave
    // below (`hookLow`) layered in as the pieces speed up.
    layers.push(play(note(hookNotes), 'hook')
      .attack(0.01).decay(0.2).sustain(0.45).release(0.25)
      .lpf(2800).lpq(3)
      .phaser(0.3).phaserdepth(0.35).phasersweep(800)
      .delay(0.3).delaytime(echo).delayfeedback(0.35)
      .pan(0.55).room(0.5).size(hall)
      .gain(level(o.hook)))
    if (on(o.hookLow)) {
      layers.push(play(note(octaveDown(hookNotes)), 'hookLow')
        .attack(0.005).decay(0.15).sustain(0.3).release(0.15)
        .lpf(1500)
        .pan(0.42).room(0.35).size(hall)
        .gain(level(o.hookLow)))
    }
    if (on(o.hookEcho)) {
      // An echo canon: the hook an octave up and one eighth late, as short
      // filtered plucks that land in the hook's rests.
      layers.push(play(note(octaveUp(hookNotes)), 'hookEcho').late(1 / 8)
        .decay(0.1).sustain(0).release(0.05)
        .lpf(700).lpenv(4).lpdecay(0.08).lpq(5)
        .delay(0.25).delaytime(echo).delayfeedback(0.3)
        .pan(0.7).room(0.4).size(hall)
        .gain(level(o.hookEcho)))
    }
  }
  if (on(o.seqbass)) {
    // An analogue-sequencer bass: root, octave and fifth through a resonant
    // filter that slowly opens and closes.
    layers.push(play(note(o.seqSteps).add(note(spaceRootNumbers)), 'seqbass')
      .decay(0.13).sustain(0).release(0.04)
      .lpf(sine.range(350, 1500).slow(8)).lpq(8)
      .room(0.3).size(hall)
      .gain(level(o.seqbass)))
  }
  if (on(o.machine)) {
    // A soft rhythm-box groove: light kicks, woodblock-like clicks and maracas.
    layers.push(note("g1 ~ ~ g1 ~ ~ g1 ~").s("sine")
      .decay(0.2).sustain(0).penv(18).pdecay(0.04)
      .room(0.15).size(hall).gain(level(o.machine)).postgain(0.45))
    layers.push(note("~ e6 ~ e6 ~ e6 e6 ~").s("sine")
      .decay(0.025).sustain(0).hpf(1500)
      .pan(0.62).room(0.3).size(hall).gain(level(o.machine)).postgain(0.2))
    layers.push(s("pink*8")
      .decay(0.03).sustain(0).bandf(6500).bandq(1.5)
      .pan(0.38).room(0.2).size(hall).gain(level(o.machine)).postgain(0.18))
  }
  if (on(o.zaps)) {
    // Laser zaps: falling sawtooth sweeps, now and then.
    layers.push(play(note("<[~ ~ ~ a5] ~ [~ ~ d6 ~] ~ [~ a5 ~ ~] ~ [~ ~ ~ e6] ~>"), 'zaps')
      .decay(0.3).sustain(0).penv(-24).pattack(0.28)
      .lpf(3200)
      .delay(0.35).delaytime(echo).delayfeedback(0.4)
      .pan(rand.range(0.2, 0.8))
      .orbit(2).room(0.6).size(deep)
      .gain(level(o.zaps)))
  }
  return { from, to, bpm, next: o.next, pattern: stack(...layers) }
}

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
window.blocks = {
  // 60 BPM: drifting.
  drift_a: block({ from: 60, keys: 0.24, ensemble: 0.24, sequence: 0.1, pad: 0.1,
    bells: 0.16, stars: 0.1, starsSparse: 0.45,
    next: { drift_b: 2, space: 1, rise_70: 2 } }),
  drift_b: block({ from: 60, keys: 0.22, ensemble: 0.23, bass: 0.5, guitar: 0.16, pad: 0.09, viola: 0.12,
    sequence: 0.09, bells: 0.1, stars: 0.08,
    next: { drift_a: 1, space: 1, rise_70: 3 } }),
  // 60 to 120 BPM: the space pieces.
  space: block({ from: 60, ...spaceHarmony, keys: 0.12, ensemble: 0.2, bass: 0.26, space: 0.19,
    sequence: 0.1, pad: 0.1, halo: 0.12, crystal: 0.08, beeps: 0.1, hook: 0.08,
    bells: 0.1, stars: 0.07, starsSparse: 0.6,
    next: { drift_a: 2, drift_b: 1 } }),
  rise_70: block({ from: 60, to: 70, keys: [0.22, 0.2], ensemble: [0.23, 0.2], guitar: [0.08, 0.13], pad: [0.1, 0.09],
    arp: [0, 0.07], sequence: [0.1, 0.09],
    shaker: [0, 0.08], lift: [0, 0.07], bells: 0.14, stars: 0.1,
    next: { rise_80: 2 } }),
  rise_80: block({ from: 70, to: 80, keys: [0.2, 0.18], ensemble: [0.2, 0.16], guitar: [0.13, 0.17], pad: [0.09, 0.07], pluck: [0, 0.04],
    arp: [0.07, 0.13], sequence: [0.09, 0], hats: [0, 0.22],
    shaker: [0.08, 0.1], lift: [0.07, 0.09], bells: 0.12, stars: 0.1,
    next: { depart_a: 1, depart_b: 1, space_80: 1 } }),
  // 80 BPM: departing.
  depart_a: block({ from: 80, keys: 0.18, arp: 0.13, guitar: 0.19, pad: 0.06, viola: 0.1, organ: 0.04, horns: 0.05,
    hats: 0.22, shaker: 0.09, lift: 0.07,
    bells: 0.13, stars: 0.1,
    next: { depart_b: 2, rise_90: 2, space_80: 1 } }),
  depart_b: block({ from: 80, keys: 0.16, arp: 0.13, arpNotes: arpWide, guitar: 0.17, pad: 0.06, viola: 0.08,
    organ: 0.05, horns: 0.05, woodwinds: 0.06, counter: 0.12, hats: 0.22,
    shaker: 0.09, lift: 0.07, bells: 0.1,
    stars: 0.08, next: { depart_a: 1, rise_90: 2, space_80: 1 } }),
  rise_90: block({ from: 80, to: 90, keys: [0.18, 0.14], arp: 0.13, arpLpf: [1400, 1800], guitar: [0.17, 0.22],
    guitarNotes: guitarStrums, rhythm: [0, 0.025], pad: [0.06, 0.03], pluck: [0.04, 0.06], viola: 0.08,
    organ: [0.05, 0.07], horns: [0.05, 0.08], woodwinds: [0.04, 0.06], hats: 0.22,
    shaker: [0.08, 0.11], lift: [0.08, 0.1],
    bells: 0.1, stars: 0.08, next: { rise_100: 2 } }),
  rise_100: block({ from: 90, to: 100, keys: [0.14, 0.1], arp: 0.13, arpLpf: [1800, 2200], guitar: 0.22, rhythm: [0.025, 0.045], pluck: 0.06, pump: [0, 0.05],
    organ: [0.07, 0.08], horns: [0.08, 0.1], woodwinds: [0.06, 0.07], drums: 'in', hats: 0.22,
    shaker: [0.11, 0.14], lift: [0.1, 0.12],
    bells: [0.12, 0.05], stars: 0.07, next: { cruise_a: 2, cruise_b: 1 } }),
  space_80: block({ from: 80, ...spaceHarmony, keys: 0.08, ensemble: 0.12, bass: 0,
    seqbass: 0.1, halo: 0.09, pad: 0.08, sequence: 0.06, crystal: 0.06, beeps: 0.08,
    machine: 0.5, hook: 0.12, bells: 0.08, stars: 0.06,
    next: { depart_a: 1, depart_b: 1, rise_90: 1 } }),
  // 100 BPM: cruising, with the melodies and the solo.
  cruise_a: block({ from: 100, ...cruise, next: { cruise_b: 2, solo: 1, rise_110: 1, space_100: 1 } }),
  cruise_b: block({ from: 100, ...cruise, leadNotes: melodyB, guitarNotes: guitarStrums, viola: 0.08, next: { cruise_a: 1, solo: 2, rise_110: 1, space_100: 1 } }),
  solo: block({ from: 100, bars: 16, ...soloGuitar, ensemble: [0.09, 0.14], arp: [0.05, 0.11],
    guitar: [0.22, 0.3], guitarNotes: solo, guitarBends: true,
    organ: [0.04, 0.11], horns: [0, 0.1], viola: [0, 0.12],
    next: { cruise_a: 1, rise_110: 2, fall_90: 1 } }),
  space_100: block({ from: 100, ...spaceHarmony, keys: 0.08, ensemble: 0.1, bass: 0.2,
    seqbass: 0.12, seqSteps: sixteenthSteps, halo: 0.06, choir: 0.07, pad: 0.05,
    arp: 0.08, arpLpf: 2000, crystal: 0.04, beeps: 0.06,
    drums: 1, hats: 0.16, toms: 0.05, hook: 0.15, hookLow: [0, 0.1], pluck: [0, 0.04],
    next: { cruise_a: 1, rise_110: 1, fall_90: 1 } }),
  rise_110: block({ from: 100, to: 110, ...cruise, arpLpf: [2200, 2700], stabs: [0, 0.04],
    guitarNotes: guitarStrums, rhythm: [0.045, 0.055], pump: [0.05, 0.07], pluck: 0.06,
    organ: [0.08, 0.1], horns: [0.1, 0.12], woodwinds: [0.07, 0.08],
    shaker: [0.12, 0.15], lift: [0.1, 0.12], bells: 0, stars: 0,
    next: { rise_120: 1 } }),
  rise_120: block({ from: 110, to: 120, ...arrive, stabs: [0.04, 0.075],
    organ: [0.1, 0.12], horns: [0.12, 0.14], woodwinds: 0.08,
    ostinato: [0, 0.055], ride: [0, 0.055], toms: [0, 0.07],
    shaker: [0.15, 0.16], lift: [0.12, 0.14], leadNotes: octaveUp(melodyA),
    next: { arrive_a: 1, arrive_b: 1, fanfare: 1, space_120: 1 } }),
  // 120 BPM: arriving, the peak.
  arrive_a: block({ from: 120, ...arrive, leadNotes: octaveUp(melodyA), next: { arrive_b: 2, fanfare: 1, fall_110: 2, space_120: 1 } }),
  arrive_b: block({ from: 120, ...arrive, leadNotes: octaveUp(melodyB), guitarNotes: guitarStrums, next: { arrive_a: 1, fanfare: 1, fall_110: 2, space_120: 1 } }),
  fanfare: block({ from: 120, ...arrive, lead: 0.21, leadNotes: fanfareLine,
    horns: 0.18, organ: 0.14, woodwinds: 0.05, guitar: 0.11, counter: 0, pump: 0.07,
    arp: 0.1, stabs: 0.05, ostinato: 0.07,
    next: { arrive_a: 1, arrive_b: 1, fall_110: 2, space_120: 1 } }),
  space_120: block({ from: 120, ...spaceHarmony, keys: 0.06, ensemble: 0.12, bass: 0.4,
    seqbass: 0.13, seqSteps: sixteenthSteps, choir: 0.09, pump: 0.07, stabs: 0.04,
    arp: 0.12, arpLpf: 3200, arpPhaserRate: 0.25, arpPhaserDepth: 0.16, arpPhaserSweep: 450,
    zaps: 0.06, beeps: 0.04, drums: 1, hats: 0.18, shaker: 0.14, ride: 0.06, toms: 0.07,
    hook: 0.16, hookLow: 0.11, hookEcho: 0.07, pluck: 0.05, pad: 0.05,
    next: { arrive_a: 1, arrive_b: 1, fanfare: 1, fall_110: 1 } }),
  fall_110: block({ from: 120, to: 110, ...arrive, stabs: [0.075, 0.04], pump: [0.08, 0.05], pluck: [0.05, 0.03],
    shaker: [0.16, 0], ostinato: [0.055, 0], ride: [0.055, 0], toms: [0.07, 0],
    lift: [0.12, 0], leadNotes: octaveUp(melodyB),
    next: { fall_100: 1 } }),
  fall_100: block({ from: 110, to: 100, ...cruise, keys: [0.1, 0.13], arpLpf: [3200, 2200], stabs: [0.04, 0],
    guitarNotes: guitarStrums, pump: [0.05, 0.02], pluck: [0.03, 0],
    shaker: 0, lift: 0,
    bells: [0, 0.05], next: { solo: 1, cruise_a: 1, fall_90: 2, space_100: 1 } }),
  // Back down through 90, 80, 70 to 60.
  fall_90: block({ from: 100, to: 90, keys: [0.13, 0.16], arp: 0.13, guitar: [0.21, 0.14], rhythm: [0.045, 0], pump: [0.02, 0], counter: [0.16, 0.08], pad: [0, 0.04],
    organ: [0.08, 0.03], horns: [0.1, 0], woodwinds: [0.07, 0],
    drums: 'out', hats: 0.22, bells: 0.08, stars: 0.08, next: { fall_80: 1 } }),
  fall_80: block({ from: 90, to: 80, keys: [0.16, 0.18], arp: 0.13, arpLpf: [1800, 1400], guitar: [0.14, 0.12], counter: [0.08, 0], pad: [0.04, 0.07], viola: 0.1,
    hats: 0.22, bells: [0.08, 0.13], stars: 0.1, next: { wind: 1 } }),
  wind: block({ from: 80, keys: 0.18, arp: 0.13, guitar: 0.12, pad: 0.07, viola: 0.11, hats: 0.22,
    sequence: 0.035, bells: 0.13, stars: 0.1,
    next: { fall_70: 2, depart_a: 1, space_80: 1 } }),
  fall_70: block({ from: 80, to: 70, keys: [0.18, 0.2], arp: [0.13, 0.07], guitar: [0.12, 0], sequence: [0.035, 0.06], pad: 0.08, viola: [0.1, 0.06],
    hats: [0.22, 0],
    bells: 0.14, stars: 0.1, next: { fall_60: 1 } }),
  fall_60: block({ from: 70, to: 60, keys: [0.2, 0.22], arp: [0.07, 0], pad: [0.08, 0.1],
    sequence: [0.06, 0.045],
    bells: 0.16, stars: 0.1, next: { drift_a: 1, space: 1 } }),
}

// strudel.cc: one pass through every block.
const demo = ['drift_a', 'drift_b', 'space', 'drift_a', 'rise_70', 'rise_80', 'space_80', 'depart_a', 'depart_b', 'rise_90',
  'rise_100', 'cruise_a', 'solo', 'space_100', 'cruise_a', 'cruise_b', 'rise_110', 'rise_120', 'arrive_a', 'fanfare', 'space_120', 'arrive_b',
  'fall_110', 'fall_100', 'fall_90', 'fall_80', 'wind', 'fall_70', 'fall_60']
arrange(...demo.map((name) => [window.blocks[name].bpm.length, window.blocks[name].pattern]))
