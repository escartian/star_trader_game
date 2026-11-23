// Star Trader music — the shared library: instruments, helpers and every
// layer a block can use. An original composition for this game.
//
// The music is split into this library and one file per family of blocks
// (main.strudel.js, space.strudel.js, funk.strudel.js). `tools/music/render.mjs`
// joins them into music.strudel.js (paste that into https://strudel.cc and
// press play to hear a demo of every block at one fixed tempo) and exports
// each block, with its reverb tail, to assets/music/ plus the transitions in
// manifest.json. It re-renders a block only when this library or the block's
// family file changes; see tools/music/README.md.
//
// The game does not play one recording. It plays blocks of bars (8 unless a
// block sets `bars`, such as the 16-bar solo), choosing each next block at
// random from the ones the current block lists in `next` (with weights), so
// the music keeps changing over a long session, rising from 60 to 120 BPM and
// settling back, and moving between the families. A block's tempo eases from
// `from` to `to` BPM over its bars, and a block may only lead to blocks that
// start at the tempo it ends on.
//
// The main track plays an 8-chord cycle, one chord per bar, in D minor:
// i - VI - III - VII, then i - VI - iv - V (sus, resolving on the next block).
// The other families hold chords longer but keep D minor and end on A, so
// every block resolves into every other.
//
// The sound mixes Strudel's own synthesizers (pads, pumping chords, plucks, a
// laser harp, the drums and noise effects) with General MIDI instruments
// (FluidR3_GM, MIT licence). The exporter serves the instrument data from
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
  hook: { sound: "sine", boost: 1.8 },
  riff: { sound: "sawtooth", boost: 2.5 },
  laser: { sound: "sawtooth", boost: 1.5 },
  harp: { sound: "gm_orchestral_harp:1", boost: 1 },
  chop: { sound: "supersaw", boost: 1.27 },
  hookLow: { sound: "square", boost: 1.75 },
  hookEcho: { sound: "sawtooth", boost: 2.5 },
  seqbass: { sound: "sawtooth", boost: 1.25 },
  zaps: { sound: "sawtooth", boost: 2.5 },
  // The funk pieces: funk voices first, then phonk (cowbell, 808). Sparse
  // voices (clavinet riff, horn stabs) are boosted less than measured.
  talk: { sound: "sawtooth", boost: 0.65 },
  clav: { sound: "gm_clavinet:1", boost: 2.5 },
  scratch: { sound: "gm_electric_guitar_muted:7", boost: 1.8 },
  slap: { sound: "gm_slap_bass_1:2", boost: 1.3 },
  brass: { sound: "gm_brass_section:3", boost: 3 },
  cowbell: { sound: "square", boost: 1.5 },
  bass808: { sound: "sine", boost: 0.19 },
}
const play = (pattern, name) => pattern.s(instruments[name].sound).postgain(instruments[name].boost)

// Strings that JavaScript edits use single quotes: Strudel parses double-quoted
// and backtick strings as patterns.
const chords = "<Dm9 Bb^7 F^7 C6 Dm9 Bb^7 Gm7 A7sus>"
const roots = "<d2 bb1 f1 c2 d2 bb1 g1 a1>"
// The same roots as MIDI numbers, for adding sequencer steps to them.
const rootNumbers = "<38 34 29 36 38 34 31 33>"
// One octave above the bass: a warm, syncopated electric-piano pulse.
const liftRoots = "<d3 bb2 f2 c3 d3 bb2 g2 a2>"
// One octave up: "a4" -> "a5", "bb4" -> "bb5".
const octaveUp = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) + 1))

// One octave down: "a5" -> "a4".
const octaveDown = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) - 1))

// Melodies: eight bars each, one per chord.
const melodyA = '[a4@2 d5 e5] [f5@3 [e5 d5]] [c5@2 a4 c5] g4 [a4@2 d5 e5] [f5@2 g5 f5] [e5@2 d5 bb4] a4'
// A countermelody answering the lead in the gaps between its phrases.
const counterLine = '[~ ~ [f4 e4] [d4 a3]] [~ ~ [d4 f4] a4] [~ ~ [a4 g4] [f4 e4]] [[e4 g4] [a4 c5] [g4 e4] d4] [~ ~ [f4 a4] [c5 a4]] [~ ~ [d5 c5] [bb4 a4]] [~ ~ [g4 bb4] [d5 bb4]] [[a4 g4] [e4 d4] [e4 g4] a4]'
// Guitar notation. A chord `[a,b,c]` is strummed: each later string starts
// 1/STRUM of the chord's span after the one before. `d6^2` bends up into d6
// from two semitones below. `bb5h` is slurred from the previous note (a
// hammer-on or pull-off, as in guitar tab): softer, with a gentler attack, so a
// run flows from its one picked note. `perNote` turns these marks into
// patterns with one value per note or chord, matching the notes' structure.
const STRUM = 32
const chordMark = /\[([^\[\]]*,[^\[\]]*)\]/g
const strum = (text) => text.replace(chordMark, (_, inner) => '[' + inner.split(',')
  .map((note, string) => (string ? '[~@' + string + ' ' + note.trim() + '@' + (STRUM - string) + ']' : note.trim())).join(', ') + ']')
const notesOf = (text) => text.replace(/\^\d|(?<=\d)h/g, '')
// `value` receives a note with its marks, or a chord's notes ('d5,f5,a5').
const perNote = (text, value) => '<' + text
  .replace(chordMark, (_, inner) => String(value(inner)))
  .replace(/[a-g]b?\d[\^\dh]*/g, (note) => String(value(note))) + '>'
const bendOf = (note) => Number((note.match(/\^(\d)/) ?? [0, 0])[1])
const slurred = (note) => /\dh/.test(note)
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
// Short phrases answer the synth melody; rests keep the lead from being busy.
const guitarLine = '[~ d4 ~ a4] [~ f4 ~ a4] [~ c5 ~ a4] [g4 ~ e4 ~] [~ d4 ~ f4] [~ a4 ~ c5] [~ bb4 ~ g4] [a4 ~ ~ ~]'
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

// Laser-harp sweeps (theme A): up and down each chord's voicing in eighth
// notes, one octave above it; double time at 120 BPM like the hook.
const laserSweep = '0 2 4 6 8 6 4 2'
// Orchestral-harp glissandi: a quick run up the chord in the first quarter of
// each chord's first bar (chords last two bars in the space pieces).
const harpGlissando = "<[[0 1 2 3 4 5 6 7 8 9] ~ ~ ~] ~>"
// Computer beeps (square, high) and boops (triangle, low): steady pitches from
// D minor pentatonic in short bursts, like ship telemetry.
const beepNotes = "<[d6 d6 ~ a6] ~ [~ c7 c7 ~] ~ [f6 ~ f6 ~] ~ [~ ~ g6 a6] ~>"
const boopNotes = "<~ [~ ~ d4 ~] ~ [a3 ~ ~ ~] ~ [~ f4 ~ ~] ~ [~ ~ ~ [d4 a3]]>"
// Sequencer steps above the root, in semitones: root, octave and fifth.
const eighthSteps = '[0 12 7 12]*2'
const sixteenthSteps = '[0 12 7 12]*4'
// A disco bass: root and octave in eighth notes.
const discoSteps = '[0 12]*4'

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
    chords, roots, rootNumbers, halo: 0, choir: 0, crystal: 0, beeps: 0, seqbass: 0, seqSteps: eighthSteps,
    hook: 0, hookNotes: '', hookLow: 0, hookEcho: 0, riff: 0, riffNotes: '', chop: 0, openhat: 0,
    laser: 0, laserNotes: laserSweep, harp: 0,
    machine: 0, zaps: 0,
    talk: 0, talkNotes: '', clav: 0, clavNotes: '', scratch: 0, slap: 0, slapNotes: '', brass: 0,
    cowbell: 0, cowbellNotes: '', bass808: 0, trap: 0, clap: 0, clapPattern: "~ pink ~ pink", groove: 0,
    congas: 0, tambourine: 0,
    ...settings }
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
    if (o.guitarBends) {
      guitar = guitar.penv(perNote(o.guitarNotes, bendOf)).pattack(0.14)
        .velocity(perNote(o.guitarNotes, (note) => (slurred(note) ? 0.62 : 1)))
        .attack(perNote(o.guitarNotes, (note) => (slurred(note) ? 0.03 : 0.005)))
    }
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
    // The space hook on a glassy FM bell-synth, short and staccato, ringing
    // into the deep reverb, with a square an octave below (`hookLow`) at 120.
    layers.push(play(note(o.hookNotes), 'hook')
      .fmi(1.6).fmh(3)
      .attack(0.004).decay(0.16).sustain(0.08).release(0.25)
      .lpf(5000)
      .phaser(0.2).phaserdepth(0.25).phasersweep(900)
      .delay(0.35).delaytime(echo).delayfeedback(0.4)
      .pan(sine.range(0.35, 0.65).slow(4))
      .orbit(2).room(0.55).size(deep)
      .gain(level(o.hook)))
    if (on(o.hookLow)) {
      layers.push(play(note(octaveDown(o.hookNotes)), 'hookLow')
        .attack(0.005).decay(0.15).sustain(0.3).release(0.15)
        .lpf(1500)
        .pan(0.42).room(0.35).size(hall)
        .gain(level(o.hookLow)))
    }
    if (on(o.hookEcho)) {
      // An echo canon: the hook one eighth late, as short filtered plucks
      // that land in the hook's rests.
      layers.push(play(note(o.hookNotes), 'hookEcho').late(1 / 8)
        .decay(0.1).sustain(0).release(0.05)
        .lpf(700).lpenv(4).lpdecay(0.08).lpq(5)
        .delay(0.25).delaytime(echo).delayfeedback(0.3)
        .pan(0.7).room(0.4).size(hall)
        .gain(level(o.hookEcho)))
    }
  }
  if (on(o.laser)) {
    // A laser harp in the manner of Jarre's: a bright sawtooth pluck snapped
    // open by a fast resonant filter, shimmering through a phaser and ringing
    // out in long echoes and the deep reverb.
    layers.push(play(n(o.laserNotes).chord(o.chords).voicing().transpose(12), 'laser')
      .attack(0.002).decay(0.35).sustain(0).release(0.3)
      .lpf(900).lpenv(5).lpdecay(0.12).lpq(6)
      .phaser(0.4).phaserdepth(0.3).phasersweep(1200)
      .delay(0.45).delaytime(echo).delayfeedback(0.5)
      .pan(sine.range(0.2, 0.8).slow(2))
      .orbit(2).room(0.6).size(deep)
      .gain(level(o.laser)))
  }
  if (on(o.harp)) {
    // Orchestral-harp glissandi that mark each chord change.
    layers.push(play(n(harpGlissando).chord(o.chords).voicing(), 'harp')
      .decay(1.4).sustain(0).release(0.8)
      .pan(sine.range(0.3, 0.7).slow(8))
      .orbit(2).room(0.6).size(deep)
      .gain(level(o.harp)))
  }
  if (on(o.riff)) {
    // The theme-B riff: clipped sawtooth notes through a resonant filter that
    // sweeps open and shut, bouncing between the speakers.
    layers.push(play(note(o.riffNotes), 'riff')
      .attack(0.003).decay(0.07).sustain(0).release(0.03)
      .lpf(sine.range(1200, 4200).slow(8)).lpq(7)
      .delay(0.2).delaytime(echo).delayfeedback(0.3)
      .pan("<0.35 0.65>*8").room(0.3).size(hall)
      .gain(level(o.riff)))
  }
  if (on(o.chop)) {
    // Chopped house chords: a high voicing on every offbeat, cut short and
    // swept by a slow filter, like a looped and filtered disco sample.
    layers.push(play(chord(o.chords).voicing().transpose(12), 'chop')
      .struct("[~ x]*4")
      .attack(0.003).decay(0.09).sustain(0).release(0.04)
      .lpf(sine.range(800, 3600).slow(16)).lpq(5)
      .pan(0.45).room(0.35).size(hall)
      .gain(level(o.chop)))
  }
  if (on(o.openhat)) {
    // Disco open hats on the offbeats.
    layers.push(s("[~ white]*4")
      .decay(0.16).sustain(0).release(0.05)
      .hpf(7000).pan(0.6)
      .room(0.25).size(hall)
      .gain(level(o.openhat)))
  }
  if (on(o.seqbass)) {
    // An analogue-sequencer bass: root, octave and fifth through a resonant
    // filter that slowly opens and closes.
    layers.push(play(note(o.seqSteps).add(note(o.rootNumbers)), 'seqbass')
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
  if (on(o.talk)) {
    // A talkbox-style lead: a sawtooth whose vowel formant shifts each beat.
    layers.push(play(note(o.talkNotes), 'talk')
      .vowel("<o a e a>*4")
      .attack(0.02).decay(0.25).sustain(0.55).release(0.2)
      .lpf(2600)
      .delay(0.25).delaytime(echo).delayfeedback(0.3)
      .pan(0.5).room(0.4).size(hall)
      .gain(level(o.talk)))
  }
  if (on(o.clav)) {
    // A funky clavinet riff, clipped and wah-swept.
    layers.push(play(note(o.clavNotes), 'clav')
      .attack(0.002).decay(0.12).sustain(0.1).release(0.05)
      .hpf(250).lpf(sine.range(1200, 3800).fast(2)).lpq(4)
      .pan(0.62).room(0.25).size(hall)
      .gain(level(o.clav)))
  }
  if (on(o.scratch)) {
    // Chicken-scratch guitar: muted strums on every offbeat sixteenth, wah-swept.
    layers.push(play(chord(o.chords).voicing().transpose(12), 'scratch')
      .struct("[~ x]*8")
      .attack(0.002).decay(0.05).sustain(0).release(0.03)
      .hpf(600).lpf(sine.range(900, 2800).fast(1)).lpq(6)
      .pan(0.35).room(0.2).size(hall)
      .gain(level(o.scratch)))
  }
  if (on(o.slap)) {
    // Slap bass: thumbed roots and popped octaves.
    layers.push(play(note(o.slapNotes), 'slap')
      .attack(0.002).decay(0.25).sustain(0.2).release(0.06)
      .lpf(2600)
      .room(0.15).size(hall)
      .gain(level(o.slap)))
  }
  if (on(o.brass)) {
    // Horn-section stabs on the "and" of two and on four.
    layers.push(play(chord(o.chords).voicing(), 'brass')
      .struct("~ ~ ~ ~ ~ ~ x ~ ~ ~ ~ ~ x ~ ~ ~")
      .attack(0.01).decay(0.18).sustain(0).release(0.08)
      .lpf(3200)
      .pan(0.45).room(0.35).size(hall)
      .gain(level(o.brass)))
  }
  if (on(o.cowbell)) {
    // A phonk cowbell: two square waves a fifth apart, short and bandpassed,
    // playing a melody.
    const cowbell = (pattern) => play(pattern, 'cowbell')
      .attack(0.001).decay(0.09).sustain(0).release(0.04)
      .bandf(1400).bandq(1.2)
      .delay(0.2).delaytime(echo).delayfeedback(0.25)
      .pan(0.56).room(0.3).size(hall)
      .gain(level(o.cowbell))
    layers.push(cowbell(note(o.cowbellNotes)), cowbell(note(o.cowbellNotes).transpose(7)))
  }
  if (on(o.bass808)) {
    // A distorted 808 bass: long sine notes on the roots, two octaves down,
    // in a syncopated phonk rhythm.
    layers.push(play(note(o.roots).transpose(-12), 'bass808')
      .struct("x ~ ~ x ~ ~ x ~ ~ ~ x ~ x ~ ~ ~")
      .attack(0.004).decay(0.6).sustain(0.35).release(0.15)
      .shape(0.55).lpf(420)
      .room(0.1).size(hall)
      .gain(level(o.bass808)))
  }
  if (on(o.trap)) {
    // Trap hats: steady eighths with triplet rolls in alternate bars.
    layers.push(s('<[white white white white white white white white] ' +
        '[white white white white white white white*3 white*3] ' +
        '[white white white white white white white white] ' +
        '[white white white*2 white white white*4 white white*3]>')
      .decay(0.03).sustain(0)
      .hpf(9000).pan(0.55)
      .room(0.15).size(hall)
      .gain(level(o.trap)))
  }
  if (on(o.clap)) {
    // A clap: a wide noise burst, on two and four or (phonk half time) on three.
    layers.push(s(o.clapPattern)
      .decay(0.14).sustain(0).release(0.05)
      .bandf(1500).bandq(1.2)
      .room(0.35).size(hall)
      .gain(level(o.clap)))
  }
  if (on(o.groove)) {
    // A syncopated funk kit: kick on the one and in between, snare on two and
    // four, quiet ghost notes around it. Scaled as a whole by `groove`.
    layers.push(note("g1 ~ ~ g1 ~ ~ g1 ~ ~ ~ g1 ~ ~ ~ ~ ~").s("sine")
      .decay(0.28).sustain(0).penv(30).pdecay(0.05)
      .room(0.15).size(hall).gain(level(o.groove)).postgain(0.6))
    layers.push(s("~ ~ ~ ~ pink ~ ~ ~ ~ ~ ~ ~ pink ~ ~ ~")
      .decay(0.16).sustain(0).bandf(1700).bandq(0.8)
      .room(0.4).size(hall).gain(level(o.groove)).postgain(0.34))
    layers.push(s("~ ~ ~ ~ ~ ~ ~ pink ~ pink ~ ~ ~ ~ ~ pink")
      .decay(0.06).sustain(0).bandf(2200).bandq(1)
      .room(0.3).size(hall).gain(level(o.groove)).postgain(0.1))
  }
  if (on(o.congas)) {
    // Congas: a high and a low drum in a syncopated sixteenth pattern, each a
    // short sine with a quick pitch drop.
    layers.push(note("[~ d4] [a3 ~] [~ d4] [d4 a3] [~ d4] [a3 ~] [d4 ~] [a3 d4]").s("sine")
      .attack(0.002).decay(0.14).sustain(0).penv(7).pdecay(0.03)
      .lpf(1800).pan("<0.3 0.7>*8")
      .room(0.25).size(hall)
      .gain(level(o.congas)).postgain(0.5))
  }
  if (on(o.tambourine)) {
    // A tambourine: jingly sixteenths, accented on two and four.
    layers.push(s("white*16")
      .velocity("[0.4 0.25 0.6 0.25 1 0.25 0.6 0.25]*2")
      .attack(0.001).decay(0.05).sustain(0)
      .bandf(7500).bandq(2)
      .pan(0.66).room(0.2).size(hall)
      .gain(level(o.tambourine)))
  }
  return { from, to, bpm, next: o.next, pattern: stack(...layers) }
}

// A ramp between two pieces of a family whose settings `table` holds by BPM:
// every level eases from where the first piece ends to where the second
// begins; other settings come from the faster piece; the drums come in or go
// out over the second half. `extra` settings apply throughout.
const fixedSettings = ['drums', 'starsSparse', 'arpPhaserRate', 'arpPhaserDepth', 'arpPhaserSweep']
function familyRamp(table, harmony, from, to, next, extra = {}) {
  const first = table[from]
  const second = table[to]
  const ending = (value) => (Array.isArray(value) ? value[1] : value)
  const starting = (value) => (Array.isArray(value) ? value[0] : value)
  const faster = to > from ? second : first
  const settings = {}
  for (const key of new Set([...Object.keys(first), ...Object.keys(second)])) {
    const a = ending(first[key] ?? 0)
    const b = starting(second[key] ?? 0)
    settings[key] = typeof a === 'number' && typeof b === 'number' && !fixedSettings.includes(key)
      ? [a, b]
      : faster[key] ?? first[key] ?? second[key]
  }
  settings.drums = first.drums === second.drums ? first.drums : second.drums ? 'in' : 'out'
  return block({ from, to, ...harmony, ...settings, ...extra, next })
}

// Families register their blocks here; the exporter re-renders a block only
// when the library or its family's file changes.
window.blocks = {}
window.demo = []
function family(name, blocks, demo) {
  for (const [id, built] of Object.entries(blocks)) window.blocks[id] = { ...built, family: name }
  window.demo.push(...demo)
}
