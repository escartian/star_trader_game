# Game effects

The five Ogg files in `sounds/` are original sounds synthesized for this game.
They cover the UI click (also pitched up for selection), laser shot, shield hit,
hull hit and explosion. They are exported from 44.1 kHz mono PCM to stereo Ogg
Vorbis and embedded in the desktop executable. No samples or sound packs are used.

Regenerate them with Python and ffmpeg:

```sh
python3 assets/source/effects/generate.py
```

Battle ring, spark and smoke visuals are drawn directly in
`crates/game-client/src/battle/particles.rs`; there are no particle sprites to
export.
