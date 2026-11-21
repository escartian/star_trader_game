// Star Trader — main menu theme, "Long Haul".
// An original composition for this game. Paste this file into https://strudel.cc
// and press play to hear or edit it. `tools/music/render.mjs` exports it to the
// looping runtime file assets/music/main-theme.ogg (see tools/music/README.md).
//
// Only Strudel's built-in synthesizers, FM and noise are used: no samples, so the
// export is self-contained and works offline.
//
// Time: one cycle is one 4/4 bar at 96 BPM (2.5 s). The piece is 32 bars
// (80 s) in four 8-bar sections and loops back to the start:
//   bars  1-8   drift   pads, a soft bass pulse and the first star bells
//   bars  9-16  depart  the arpeggio, hats and twinkling stars come in
//   bars 17-24  cruise  the melody, kick and snare join
//   bars 25-32  arrive  the melody climbs an octave, the arpeggio opens up
setcps(96 / 60 / 4)

// D minor: i - VI - III - VII, then i - VI - iv - V (sus, resolving on the loop).
const chords = "<Dm9 Bb^7 F^7 C6 Dm9 Bb^7 Gm7 A7sus4>"
const roots = "<d2 bb1 f1 c2 d2 bb1 g1 a1>"

// Strings that JavaScript edits use single quotes: Strudel parses double-quoted
// and backtick strings as patterns.
// '1' where a layer plays, per bar across the 32-bar form: silent before `bar`.
const from = (bar) => '<0!' + bar + ' 1!' + (32 - bar) + '>'
// One octave up: "a4" -> "a5", "bb4" -> "bb5".
const octaveUp = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) + 1))

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
  .lpf('<1400!16 3200!16>')
  .delay(0.35).delaytime(0.469).delayfeedback(0.42)
  .pan(sine.range(0.3, 0.7).slow(4))
  .room(0.4)
  .gain(0.13)
  .mask(from(8))

// Eight bars, one per chord. The lead plays it in bars 17-24 and an octave up
// in bars 25-32; the first 16 bars of the sequence are masked.
const melody = '[a4@2 d5 e5] [f5@3 [e5 d5]] [c5@2 a4 c5] g4 [a4@2 d5 e5] [f5@2 g5 f5] [e5@2 d5 bb4] a4'
const lead = note('<' + [melody, melody, melody, octaveUp(melody)].join(' ') + '>')
  .s("sawtooth")
  .attack(0.04).decay(0.3).sustain(0.6).release(0.7)
  .lpf(2200).lpq(1)
  .vib(5).vibmod(0.1)
  .delay(0.3).delaytime(0.469).delayfeedback(0.35)
  .room(0.6).size(5)
  .gain(0.2)
  .mask(from(16))

const kick = note("g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ ~ g1 ~ ~ [~ g1]")
  .s("sine")
  .decay(0.28).sustain(0)
  .penv(30).pdecay(0.05)
  .gain(0.9)
  .mask(from(16))

const snare = s("~ pink ~ pink")
  .decay(0.16).sustain(0)
  .bandf(1700).bandq(0.8)
  .room(0.3)
  .gain(0.5)
  .mask(from(16))

const hats = s("[~ white]*4")
  .decay(0.035).sustain(0)
  .hpf(8500)
  .pan(0.62)
  .gain(0.22)
  .mask(from(8))

// Spacey chirps. FM bells: sparse, high and echoing.
const bells = n("<[0 ~ ~ ~ 4 ~ ~ ~] [~ ~ 7 ~ ~ ~ 2 ~] [4 ~ ~ ~ ~ 9 ~ ~] [~ 2 ~ ~ 6 ~ ~ ~]>")
  .scale("D5:minor")
  .s("sine").fmi(2.2).fmh(3.5)
  .decay(1.2).sustain(0)
  .delay(0.45).delaytime(0.469).delayfeedback(0.5)
  .pan(rand.range(0.15, 0.85))
  .room(0.8).size(8)
  .gain(0.15)

// Twinkling stars: random notes from the scale, some dropped, bouncing in the
// delay (three sixteenths at 96 BPM). Brighter in the last section.
const stars = n(irand(10).segment(16))
  .scale("D5:minor")
  .degradeBy(0.45)
  .s("triangle")
  .decay(0.12).sustain(0)
  .hpf(900).lpf('<2600!24 4800!8>')
  .delay(0.4).delaytime(0.469).delayfeedback(0.45)
  .pan(rand)
  .room(0.6).size(6)
  .gain(0.1)
  .mask(from(8))

stack(pad, bass, arp, lead, kick, snare, hats, bells, stars)
