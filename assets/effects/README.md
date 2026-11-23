# Kenney effects

These are unmodified originals from Kenney's [Sci-Fi Sounds](https://kenney.nl/assets/sci-fi-sounds), [UI Audio](https://kenney.nl/assets/ui-audio), and [Particle Pack](https://kenney.nl/assets/particle-pack). Each pack is CC0; its original license is in `licenses/`. The OGGs and transparent PNGs are embedded in the desktop executable at build time.

To reproduce the selection, download the three official ZIP archives in that order and run:

```sh
python3 tools/art/import_kenney_effects.py sci-fi-sounds.zip ui-audio.zip particle-pack.zip
```

The particle PNGs are the original 512-pixel source images and runtime images. The game tints and scales them while drawing.
