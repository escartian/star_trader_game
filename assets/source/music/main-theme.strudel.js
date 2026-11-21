// Star Trader — main menu theme, "Long Haul".
// An original composition for this game. Paste this file into https://strudel.cc
// and press play to hear or edit it. `tools/music/render.mjs` exports it to the
// looping runtime file assets/music/main-theme.ogg (see tools/music/README.md).
//
// Only Strudel's built-in synthesizers, FM and noise are used: no samples, so the
// export is self-contained and works offline.
//
// Form: one cycle is one 4/4 bar. 48 bars in six 8-bar sections, an arch that
// speeds up and slows down again, then loops:
//   bars  1-8   drift    60 BPM  pads, a soft bass pulse and many star chirps
//   bars  9-16  depart   80 BPM  the arpeggio and hats come in
//   bars 17-24  cruise  100 BPM  the melody, kick and snare join; fewer chirps
//   bars 25-32  arrive  120 BPM  the melody climbs an octave; chirps fall silent
//   bars 33-40  return  100 BPM  back to the cruise, a few chirps return
//   bars 41-48  wind    80 BPM  melody and drums leave, the chirps come back
// Strudel plays one tempo at a time, so strudel.cc plays it all at `setcps`.
// The export reads `tempoMap` (BPM for each bar) and renders bar by bar.
setcps(80 / 60 / 4)

// Per-section values become a 48-bar pattern: '<a!8 b!8 c!8 d!8 e!8 f!8>'.
// Strings that JavaScript edits use single quotes: Strudel parses double-quoted
// and backtick strings as patterns.
const sections = (values) => '<' + values.map((value) => value + '!8').join(' ') + '>'
const tempos = [60, 80, 100, 120, 100, 80]
window.tempoMap = tempos.flatMap((bpm) => Array(8).fill(bpm))
// A dotted eighth (three sixteenths) in seconds at each section's tempo.
const echo = sections(tempos.map((bpm) => (45 / bpm).toFixed(4)))
// One octave up: "a4" -> "a5", "bb4" -> "bb5".
const octaveUp = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) + 1))

// D minor: i - VI - III - VII, then i - VI - iv - V (sus, resolving on the loop).
const chords = "<Dm9 Bb^7 F^7 C6 Dm9 Bb^7 Gm7 A7sus>"
const roots = "<d2 bb1 f1 c2 d2 bb1 g1 a1>"

const pad = chord(chords).voicing()
  .s("sawtooth")
  .attack(0.9).release(2.2)
  .lpf(sine.range(600, 1500).slow(16)).lpq(2)
  .room(0.8).size(6)
  .gain(0.22)

const bass = note(roots)
  .struct("x ~ x x ~ x ~ x")
  .s("sawtooth")
  .decay(0.22).sustain(0.35).release(0.1)
  .lpf(380).lpenv(2.5).lpdecay(0.12)
  .gain(0.55)

const arp = n("0 2 4 6 7 6 4 2 1 3 5 7 8 7 5 3")
  .chord(chords).voicing()
  .s("square")
  .decay(0.14).sustain(0)
  .lpf(sections([1400, 1400, 2200, 3200, 2200, 1400]))
  .delay(0.35).delaytime(echo).delayfeedback(0.42)
  .pan(sine.range(0.3, 0.7).slow(4))
  .room(0.4)
  .gain(0.13)
  .mask(sections([0, 1, 1, 1, 1, 1]))

// Eight bars, one per chord: at home pitch in cruise and return, an octave up
// in arrive. The other sections are masked.
const melody = '[a4@2 d5 e5] [f5@3 [e5 d5]] [c5@2 a4 c5] g4 [a4@2 d5 e5] [f5@2 g5 f5] [e5@2 d5 bb4] a4'
const up = octaveUp(melody)
const lead = note('<' + [melody, melody, melody, up, melody, melody].join(' ') + '>')
  .s("sawtooth")
  .attack(0.04).decay(0.3).sustain(0.6).release(0.7)
  .lpf(2200).lpq(1)
  .vib(5).vibmod(0.1)
  .delay(0.3).delaytime(echo).delayfeedback(0.35)
  .room(0.6).size(5)
  .gain(0.2)
  .mask(sections([0, 0, 1, 1, 1, 0]))

const drums = sections([0, 0, 1, 1, 1, 0])

const kick = note("g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ [~ g1]")
  .s("sine")
  .decay(0.28).sustain(0)
  .penv(30).pdecay(0.05)
  .gain(0.9)
  .mask(drums)

const snare = s("~ pink ~ pink")
  .decay(0.16).sustain(0)
  .bandf(1700).bandq(0.8)
  .room(0.3)
  .gain(0.5)
  .mask(drums)

const hats = s("[~ white]*4")
  .decay(0.035).sustain(0)
  .hpf(8500)
  .pan(0.62)
  .gain(0.22)
  .mask(sections([0, 1, 1, 1, 1, 1]))

// Spacey chirps, thinning out as the music builds and returning as it calms.
// FM bells: sparse, high and echoing.
const bells = n("<[0 ~ ~ ~ 4 ~ ~ ~] [~ ~ 7 ~ ~ ~ 2 ~] [4 ~ ~ ~ ~ 9 ~ ~] [~ 2 ~ ~ 6 ~ ~ ~]>")
  .scale("D5:minor")
  .s("sine").fmi(2.2).fmh(3.5)
  .decay(1.2).sustain(0)
  .delay(0.45).delaytime(echo).delayfeedback(0.5)
  .pan(rand.range(0.15, 0.85))
  .room(0.8).size(8)
  .gain(sections([0.16, 0.13, 0.06, 0, 0.06, 0.13]))

// Twinkling stars: random notes from the scale, some dropped, bouncing in the
// delay. Fewer as the music speeds up.
const stars = n(irand(10).segment(16))
  .scale("D5:minor")
  .degradeBy(sections([0.4, 0.5, 0.75, 1, 0.75, 0.5]))
  .s("triangle")
  .decay(0.12).sustain(0)
  .hpf(900).lpf(2600)
  .delay(0.4).delaytime(echo).delayfeedback(0.45)
  .pan(rand)
  .room(0.6).size(6)
  .gain(0.1)

stack(pad, bass, arp, lead, kick, snare, hats, bells, stars)
