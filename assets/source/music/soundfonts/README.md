# Soundfont instruments

Local copies of the General MIDI instrument data that Strudel's
`@strudel/soundfonts` loader fetches from
`https://felixroos.github.io/webaudiofontdata/sound/`. The music exporter
(`tools/music/strudel-session.mjs`) serves these files, so renders work offline
and do not change if the remote copies do.

| File | Strudel sound | Used for |
| --- | --- | --- |
| `0040_FluidR3_GM_sf2_file.js` | `gm_epiano1:1` | keys |
| `0050_FluidR3_GM_sf2_file.js` | `gm_epiano2:1` | lift (root accents) |
| `0070_FluidR3_GM_sf2_file.js` | `gm_clavinet:1` | clavinet (funk pieces) |
| `0080_FluidR3_GM_sf2_file.js` | `gm_celesta:3` | bells |
| `0100_FluidR3_GM_sf2_file.js` | `gm_music_box:3` | stars |
| `0110_FluidR3_GM_sf2_file.js` | `gm_vibraphone:3` | countermelody |
| `0160_FluidR3_GM_sf2_file.js` | `gm_drawbar_organ:3` | organ |
| `0270_FluidR3_GM_sf2_file.js` | `gm_electric_guitar_clean:2` | guitar, rhythm guitar |
| `0281_FluidR3_GM_sf2_file.js` | `gm_electric_guitar_muted:7` | chicken-scratch guitar (funk pieces) |
| `0290_FluidR3_GM_sf2_file.js` | `gm_overdriven_guitar:0` | the solo's rock guitar |
| `0340_FluidR3_GM_sf2_file.js` | `gm_electric_bass_pick:1` | bass guitar (phonk pieces) |
| `0360_FluidR3_GM_sf2_file.js` | `gm_slap_bass_1:2` | slap bass (funk pieces) |
| `0380_FluidR3_GM_sf2_file.js` | `gm_synth_bass_1:1` | bass |
| `0410_FluidR3_GM_sf2_file.js` | `gm_viola:1` | viola |
| `0460_FluidR3_GM_sf2_file.js` | `gm_orchestral_harp:1` | harp (space pieces) |
| `0480_FluidR3_GM_sf2_file.js` | `gm_string_ensemble_1:2` | ostinato |
| `0501_FluidR3_GM_sf2_file.js` | `gm_synth_strings_1:3` | ensemble |
| `0540_FluidR3_GM_sf2_file.js` | `gm_synth_choir:2` | choir (space pieces) |
| `0590_FluidR3_GM_sf2_file.js` | `gm_muted_trumpet:3` | muted trumpet (funk pieces) |
| `0600_FluidR3_GM_sf2_file.js` | `gm_french_horn:1` | horns |
| `0610_FluidR3_GM_sf2_file.js` | `gm_brass_section:3` | horn stabs (funk pieces) |
| `0620_FluidR3_GM_sf2_file.js` | `gm_synth_brass_1:1` | stabs |
| `0730_FluidR3_GM_sf2_file.js` | `gm_flute:2` | woodwinds |
| `0800_FluidR3_GM_sf2_file.js` | `gm_lead_1_square:2` | arpeggio |
| `0810_FluidR3_GM_sf2_file.js` | `gm_lead_2_sawtooth:3` | lead |
| `0830_FluidR3_GM_sf2_file.js` | `gm_lead_4_chiff:2` | sequence |
| `0940_FluidR3_GM_sf2_file.js` | `gm_pad_halo:1` | halo pad (space pieces) |
| `0981_FluidR3_GM_sf2_file.js` | `gm_fx_crystal:4` | crystal (space pieces) |
| `0990_FluidR3_GM_sf2_file.js` | `gm_fx_atmosphere:3` | space interlude |
| `1170_FluidR3_GM_sf2_file.js` | `gm_melodic_tom:3` | toms |

All files come from the FluidR3_GM soundfont by Frank Wen, released under the
MIT licence.

Only use FluidR3_GM entries (or another source with a clear permissive licence)
for anything that ends up in the game's audio. To add an instrument, find its
FluidR3 entry in `tools/music/node_modules/@strudel/soundfonts/gm.mjs` (the
`:n` index is its position in that list), download the matching `.js` file
into this folder and add a row above.
