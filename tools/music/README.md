# Music export

Game music is written in [Strudel](https://strudel.cc) and kept as source in
`assets/source/music/*.strudel.js`. To hear or edit a piece, paste the file into
strudel.cc and press play. The game embeds the exported Ogg Vorbis loops in
`assets/music/`.

| Source | Runtime file | Plays |
| --- | --- | --- |
| `main-theme.strudel.js` (“Long Haul”, 48 bars easing 60-110-60 BPM) | `main-theme.ogg` (135.1 s loop) | Main menu, and in game when Settings allow |

Every piece is an original composition made with Strudel's built-in
synthesizers, FM and noise only (no samples), so the export needs no downloads.
The export prints any warnings
Strudel logs (for example an unknown sound).

## Re-export

Needs Node.js, Google Chrome and ffmpeg. From the repository root:

```sh
npm ci --prefix tools/music
node tools/music/render.mjs            # main theme → assets/music/main-theme.ogg
node tools/music/render.mjs SOURCE.strudel.js OUTPUT.ogg --bars 16
```

Set `CHROME=/path/to/chrome` when Chrome is not in the macOS default location.
The script renders offline in headless Chrome (`renderPatternAudio`, an
OfflineAudioContext, so no audio device and faster than real time), one bar at
a time. Each bar renders at its own tempo: from the source's
`window.tempoMap` (BPM for every bar, which strudel.cc ignores: there it plays
at the `setcps` tempo), or else `--bars` bars at the `setcps` tempo. Each bar's
notes ring out for 10 s, and the bars are added at their start times; what rings
past the loop's end is folded back onto the start, so the loop point has no
click or dry gap. Every bar gets a fresh page, because Strudel keeps audio
nodes from one offline render to the next and later renders in the same page
lose sounds. A bar that renders silent is reported (`DEBUG_BARS=1` prints
every bar's peak and all of Strudel's console output). Strudel's reverb (`room`, `size`) does nothing in offline renders, so the
export adds it itself (`reverb.mjs`): every bar is rendered a second time as a
reverb send, each note scaled by its `room` amount, and the send runs through
one shared hall, a convolution with a generated stereo impulse whose decay
follows the source's `const hall = N` room size (8: about 3.6 s), thinned
below 180 Hz and darkened above 5 kHz. The send is convolved as two copies of
the loop back to back, so the reverb at the loop's end rings into its start.
The mix is then
peak-normalized to -1 dBFS and encoded with libvorbis, or ffmpeg's built-in
Vorbis encoder at 192 kbit/s when libvorbis is missing. Vorbis keeps the loop
sample-exact; MP3 would pad it.
