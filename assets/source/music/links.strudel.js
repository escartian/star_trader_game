// Star Trader music — transitions between the families, at matching tempos.
// Each family's own file holds the transitions within it; these join them.
// Links change only the manifest, never a block's audio, so editing them does
// not re-render anything.

// From the main track.
link('drift_a', { space: 1, disco_60: 1, night_60: 1 })
link('drift_b', { space: 1, disco_60: 1 })
link('rise_80', { space_80: 1, disco_80: 1 })
link('depart_a', { space_80: 1, disco_80: 1 })
link('depart_b', { space_80: 1, disco_80: 1, night_80: 1 })
link('rise_100', { disco_100: 1 })
link('cruise_a', { space_100: 1, disco_100: 1 })
link('cruise_b', { space_100: 1, disco_100: 1, night_100: 1 })
link('solo', { disco_100: 1 })
link('rise_120', { space_120: 1, disco_120: 1 })
link('arrive_a', { space_120: 1, disco_120: 1 })
link('arrive_b', { space_120: 1, disco_120: 1, night_120: 1 })
link('fanfare', { space_120: 1, disco_120: 1, night_120: 1 })
link('fall_100', { space_100: 1, disco_100: 1, night_100: 1 })
link('wind', { space_80: 1, disco_80: 1, night_80: 1 })
link('fall_60', { space: 1, disco_60: 1, night_60: 1 })

// From the space pieces.
link('space', { drift_a: 2, drift_b: 1, disco_60: 1, night_60: 1 })
link('space_80', { depart_a: 1, depart_b: 1, rise_90: 1, disco_80: 1, night_80: 1 })
link('space_100', { cruise_a: 1, rise_110: 1, fall_90: 1, disco_100: 1, night_100: 1 })
link('space_120', { arrive_a: 1, arrive_b: 1, fanfare: 1, fall_110: 1, disco_120: 1, night_120: 1 })

// From the Space Disco pieces.
link('disco_60', { drift_a: 1, space: 1, night_60: 1 })
link('disco_80', { depart_a: 1, rise_90: 1, space_80: 1, night_80: 1 })
link('disco_100', { cruise_a: 1, rise_110: 1, space_100: 1, night_100: 1 })
link('disco_120', { arrive_a: 1, fall_110: 1, space_120: 1, night_120: 1 })

// From the Night Drift pieces.
link('night_60', { drift_a: 1, space: 1, disco_60: 1 })
link('night_80', { depart_b: 1, rise_90: 1, disco_80: 1 })
link('night_100', { cruise_b: 1, fall_90: 1, disco_100: 1, space_100: 1 })
link('night_120', { arrive_b: 1, fall_110: 1, disco_120: 1, space_120: 1 })
