// Star Trader — main menu theme, "Long Haul".
// An original composition for this game. Paste this file into https://strudel.cc
// and press play to hear or edit it. `tools/music/render.mjs` exports it to the
// looping runtime file assets/music/main-theme.ogg (see tools/music/README.md).
//
// Only Strudel's built-in synthesizers, FM and noise are used: no samples, so
// the export is self-contained and works offline.
//
// Time: one cycle is one 4/4 bar at 84 BPM (about 2.86 s). The piece is 32
// bars (about 91 s) in four 8-bar sections and loops back to the start:
//   bars  1-8   drift    drone, pads, solar wind and the first star bells
//   bars  9-16  depart   twinkling stars, a slow bass pulse and ticks
//   bars 17-24  cruise   the theremin-like melody and a soft half-time kick
//   bars 25-32  arrive   the melody climbs an octave and the stars brighten
// A noise riser swells into each section.
setcps(84 / 60 / 4)

// Strings that JavaScript edits use single quotes: Strudel parses double-quoted
// and backtick strings as patterns.
// '1' where a layer plays, per bar across the 32-bar form: silent before `bar`.
const from = (bar) => '<0!' + bar + ' 1!' + (32 - bar) + '>'
// One octave up: "a4" -> "a5", "bb4" -> "bb5".
const octaveUp = (notes) => notes.replace(/([a-g]b?)(\d)/g, (_, name, octave) => name + (Number(octave) + 1))

// D minor with Lydian colour, voiced by hand (one chord per bar):
// Dm9, Bbmaj7#11, Fmaj9, C6/9, Dm9, Bbmaj7#11, Gm9, Asus4(7).
const chords = "<[d3,a3,c4,e4,f4] [bb2,f3,a3,d4,e4] [f3,a3,c4,e4,g4] [c3,g3,a3,d4,e4] [d3,a3,c4,e4,f4] [bb2,f3,a3,d4,e4] [g3,bb3,d4,f4,a4] [a2,e3,g3,d4,e4]>"
const roots = "<d2 bb1 f1 c2 d2 bb1 g1 a1>"

// A low open fifth that never stops; its filter opens and closes over 32 bars.
const drone = note("[d1,a1]")
  .s("sawtooth")
  .attack(2).release(4)
  .lpf(sine.range(110, 420).slow(32)).lpq(4)
  .gain(0.32)

// Wide, slow pads.
const pad = note(chords)
  .s("supersaw").unison(5).spread(0.8).detune(0.25)
  .attack(1.4).release(3.5)
  .lpf(sine.range(500, 1800).slow(16)).lpq(1.5)
  .room(0.9).size(9)
  .gain(0.16)

// Pink noise swelling and fading every two bars, its band sweeping.
const wind = s("pink").slow(2)
  .attack(2.5).release(3)
  .bandf(sine.range(400, 2400).slow(12)).bandq(3)
  .pan(sine.range(0.2, 0.8).slow(8))
  .gain(0.22)

// FM bells: sparse, high and echoing.
const bells = n("<[0 ~ ~ ~ 4 ~ ~ ~] [~ ~ 7 ~ ~ ~ 2 ~] [4 ~ ~ ~ ~ 9 ~ ~] [~ 2 ~ ~ 6 ~ ~ ~]>")
  .scale("D5:minor")
  .s("sine").fmi(2.2).fmh(3.5)
  .decay(1.4).sustain(0)
  .delay(0.5).delaytime(0.536).delayfeedback(0.55)
  .pan(rand.range(0.15, 0.85))
  .room(0.8).size(8)
  .gain(0.18)

// Twinkling stars: random notes from the scale, some dropped, bouncing in the
// delay. Brighter in the last section.
const stars = n(irand(10).segment(16))
  .scale("D5:minor")
  .degradeBy(0.45)
  .s("triangle")
  .decay(0.12).sustain(0)
  .hpf(900).lpf('<2600!24 4800!8>')
  .delay(0.45).delaytime(0.402).delayfeedback(0.5)
  .pan(rand)
  .room(0.6).size(6)
  .gain(0.12)
  .mask(from(8))

// A slow, round bass pulse.
const bass = note(roots)
  .struct("x ~ ~ ~ ~ ~ x ~ ~ ~ x ~ ~ ~ ~ ~")
  .s("triangle")
  .decay(0.6).sustain(0.4).release(0.4)
  .lpf(320)
  .gain(0.6)
  .mask(from(8))

// Faint, high ticks like a ship's instruments.
const ticks = s("[~ white]*4")
  .degradeBy(0.3)
  .decay(0.025).sustain(0)
  .hpf(9500)
  .pan(0.65)
  .gain(0.14)
  .mask(from(8))

// Eight bars, one per chord. The lead plays it in bars 17-24 and an octave up
// in bars 25-32; the first 16 bars of the sequence are masked.
const melody = '[a4@2 d5 e5] [f5@3 [e5 d5]] [c5@2 a4 c5] g4 [a4@2 d5 e5] [f5@2 g5 f5] [e5@2 d5 bb4] a4'
// A soft triangle with wide vibrato, like a theremin, an octave-up sine above.
const lead = note('<' + [melody, melody, melody, octaveUp(melody)].join(' ') + '>')
  .layer(
    (x) => x.s("triangle").gain(0.2),
    (x) => x.add(note(12)).s("sine").gain(0.06),
  )
  .attack(0.12).decay(0.4).sustain(0.7).release(1.2)
  .lpf(2400)
  .vib(4.5).vibmod(0.22)
  .delay(0.35).delaytime(0.536).delayfeedback(0.45)
  .room(0.85).size(9)
  .mask(from(16))

// A soft half-time kick.
const kick = note("g1 ~ ~ ~ ~ ~ ~ ~ g1 ~ ~ ~ ~ ~ [~ g1] ~")
  .s("sine")
  .decay(0.35).sustain(0)
  .penv(24).pdecay(0.06)
  .gain(0.7)
  .mask(from(16))

// White noise rising through the last bar of each section.
const riser = s("<~!7 white>")
  .attack(2.4).release(0.3)
  .lpf(600).lpenv(5).lpattack(2.6)
  .room(0.7).size(8)
  .gain(0.12)

stack(drone, pad, wind, bells, stars, bass, ticks, lead, kick, riser)
