// Star Trader music — transitions between the families, at matching tempos.
// Each family's own file holds the transitions within it; these join them.
// Links change only the manifest, never a block's audio, so editing them does
// not re-render anything.

// From the main track.
link('drift_a', { space: 1, funk_60: 1, phonk_60: 1 })
link('drift_b', { space: 1, funk_60: 1 })
link('rise_80', { space_80: 1, funk_80: 1 })
link('depart_a', { space_80: 1, funk_80: 1 })
link('depart_b', { space_80: 1, funk_80: 1, phonk_80: 1 })
link('rise_100', { funk_100: 1 })
link('cruise_a', { space_100: 1, funk_100: 1 })
link('cruise_b', { space_100: 1, funk_100: 1, phonk_100: 1 })
link('solo', { funk_100: 1 })
link('rise_120', { space_120: 1, funk_120: 1 })
link('arrive_a', { space_120: 1, funk_120: 1 })
link('arrive_b', { space_120: 1, funk_120: 1, phonk_120: 1 })
link('fanfare', { space_120: 1, funk_120: 1, phonk_120: 1 })
link('fall_100', { space_100: 1, funk_100: 1, phonk_100: 1 })
link('wind', { space_80: 1, funk_80: 1, phonk_80: 1 })
link('fall_60', { space: 1, funk_60: 1, phonk_60: 1 })

// From the space pieces.
link('space', { drift_a: 2, drift_b: 1, funk_60: 1, phonk_60: 1 })
link('space_80', { depart_a: 1, depart_b: 1, rise_90: 1, funk_80: 1, phonk_80: 1 })
link('space_100', { cruise_a: 1, rise_110: 1, fall_90: 1, funk_100: 1, phonk_100: 1 })
link('space_120', { arrive_a: 1, arrive_b: 1, fanfare: 1, fall_110: 1, funk_120: 1, phonk_120: 1 })

// From the funk pieces.
link('funk_60', { drift_a: 1, space: 1, phonk_60: 1 })
link('funk_80', { depart_a: 1, rise_90: 1, space_80: 1, phonk_80: 1 })
link('funk_100', { cruise_a: 1, rise_110: 1, space_100: 1, phonk_100: 1 })
link('funk_120', { arrive_a: 1, fall_110: 1, space_120: 1, phonk_120: 1 })

// From the phonk pieces.
link('phonk_60', { drift_a: 1, space: 1, funk_60: 1 })
link('phonk_80', { depart_b: 1, rise_90: 1, funk_80: 1 })
link('phonk_100', { cruise_b: 1, fall_90: 1, funk_100: 1, space_100: 1 })
link('phonk_120', { arrive_b: 1, fall_110: 1, funk_120: 1, space_120: 1 })
