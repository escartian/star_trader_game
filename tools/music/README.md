# Music export

Game music is written in [Strudel](https://strudel.cc) and kept in
`assets/source/music/*.strudel.js`. Paste a source file into strudel.cc to hear
its fixed demo. The game embeds the exported Ogg Vorbis audio in `assets/music/`.

| Source | Runtime files | Plays |
| --- | --- | --- |
| `long-haul.strudel.js` | `long-haul/*.ogg`, `manifest.json` | Main menu; in game when enabled |
| `main-theme.strudel.js` | `main-theme.ogg` | Earlier 48-bar loop, retained as a source and export |

Long Haul has eight-bar blocks: steady drift, depart, cruise and arrive with
variations; 10 BPM ramp blocks from 60 to 120 and back; a 16-bar guitar solo
split into two blocks (`solo` always continues into `solo_b`) and a 60 BPM space
interlude. A dry FM/triangle electric-guitar synth plays short answer phrases
in depart, cruise and arrival, then strums chords in the solo, alternating
rhythmic strums with held high chords and rests. Three short note-by-note runs,
at 1.8 times the eighth-note rate, and seven whole- or half-step bends into held
chords spread across the two halves. A quiet arpeggio and a rhythm guitar play
under every solo bar, with off-beat chord stabs beneath the strums and a ringing
chord plus re-strike beneath each held note, while organ and horns gradually
join. The solo has less echo and room effect; its recurring phrases have no
vibrato.
Organ chords and horn/woodwind-like synth replies enter during departure,
build through cruise and the tempo ramps, and peak at arrival. All of these
voices are written in Strudel; no orchestra or instrument samples are used.
The 120 BPM Orbital Fanfare is a separate eight-bar theme with brass-like calls
and a clipped string-machine rhythm. Quiet ride and tuned tom voices join the
arrival blocks and fade in and out with the tempo ramps; the main kick remains
even and the hats are slightly softer there.
Short FM keyboard chords replace the sustained pad; the interlude uses spaced
notes and noise ticks instead of a held low drone.
Arrival keeps a gentle phaser sweep on the arpeggio. The sequence and arpeggio
elsewhere now use shallow sweeps; the regular lead has no vibrato.
A short, unmodulated midrange ensemble figure now supports every block, with
more of it and more sequence in the slow sections. Space also has a quiet moving
bass. These parts have rests between notes, so the harmony feels present
without restoring the old sustained drone.
A phased, filter-swept synth sequence moves through the
interlude and early ascent, with a quieter reprise as the music winds down.
The upward ramps add an even shaker pulse and a warm FM
keyboard pulse that build into the 120 BPM arrival. Quieter versions continue
through depart and cruise before the parts fade on descent. Every
bar uses four evenly spaced kick hits, without an end-of-bar pickup. The kick
and snare sit lower in the mix while the lead melody is stronger in cruise
and arrival. Each
source block declares weighted possible successors
whose starting tempo matches its ending tempo. The game chooses randomly at
each boundary. Strudel.cc uses the fixed demo order and a single live tempo;
the exporter renders the block-specific tempo for each bar.

To hear the source in a browser, open [strudel.cc](https://strudel.cc), paste
`assets/source/music/long-haul.strudel.js` into its editor and click **Play**
(or press Ctrl+Enter). Use **Stop** (Ctrl+.) to end playback. The last `arrange`
line plays every part in a fixed order at the demo tempo. The game instead
uses the rendered OGG blocks with their tempo ramps and weighted transitions.
To audition one part in Strudel, change `const demo` near the bottom to a
one-item list such as `['arrive_a']` and press **Update**. In the game, select
that part from the main-menu Songbook; it crossfades in over two seconds and
then follows its weighted transitions.

## Re-export

Requires Node.js, Google Chrome and ffmpeg. From the repository root:

```sh
npm ci --prefix tools/music
node tools/music/render.mjs
node tools/music/render.mjs assets/source/music/main-theme.strudel.js assets/music/main-theme.ogg
```

Set `CHROME=/path/to/chrome` if Chrome is not at the macOS default location.
The exporter renders offline in headless Chrome one bar at a time. Each bar's
notes can ring for nine seconds after its end. It adds bars at their exact
start frames, compresses large average loudness differences between blocks,
and applies a shared peak gain with headroom for transitions. The capped
per-block adjustment preserves a little dynamic contrast while allowing
extra instruments to add density. Each Ogg includes its tail; the manifest's
`frames` is the exact join point, excluding the tail. The game starts the next block
there and plays the old tail beneath it. The single loop export instead wraps
its tail onto the start. Vorbis preserves the sample count without MP3 padding.
The manifest also records each block's leveled active RMS. When the next block is
louder, the sequencer eases its gain up over the first bar; a quieter block
enters at its exported level while the previous tail fades. This keeps the
track's intended quiet and loud passages without a sudden upward step.

Each bar gets a fresh page because Strudel keeps audio nodes between offline
renders. The exporter waits for its reverb impulse response before rendering.
`DEBUG_BARS=1` prints per-bar peaks and Strudel console messages. A silent bar
also produces a warning.
