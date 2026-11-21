# Music export

Game music is written in [Strudel](https://strudel.cc) and kept as source in
`assets/source/music/*.strudel.js`. To hear or edit a piece, paste the file into
strudel.cc and press play. The game embeds the exported Ogg Vorbis loops in
`assets/music/`.

| Source | Runtime file | Plays |
| --- | --- | --- |
| `main-theme.strudel.js` (“Long Haul”, 84 BPM, 32 bars) | `main-theme.ogg` (91.4 s loop) | Main menu, and in game when Settings allow |

Every piece is an original composition made with Strudel's built-in
synthesizers, FM and noise only (no samples), so the export needs no downloads.
Chords are written out as notes rather than chord symbols, so the voicing does
not depend on Strudel's voicing dictionary. The export prints any warnings
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
OfflineAudioContext, so no audio device and faster than real time). It renders
two extra bars with no new notes and folds their reverb and delay tails back onto
the start, so the loop point has no click or dry gap. It then peak-normalizes to
-1 dBFS and encodes with libvorbis, or ffmpeg's built-in Vorbis encoder at
192 kbit/s when libvorbis is missing. Vorbis keeps the loop sample-exact; MP3
would pad it. `--bars` must match the piece's length in cycles. The tempo comes
from the source's `setcps(...)` line.
