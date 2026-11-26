//! A pursuit curve within one system, for any chaser: your flagship after its
//! quarry, a raider band or police patrol after your fleet.
//!
//! A run is split into [`STEPS`] equal steps. The quarry is taken to move in
//! a straight line from where it was last seen to where it is now; at each
//! step the chaser moves its share of the run's distance toward where the
//! quarry will be at the end of that step, so its path bends as the quarry
//! moves instead of running straight at a stale position. Once within
//! `reach` it has caught up and matches the quarry's course to the end of the
//! run. Positions are kilometres from the system's star; the chaser never
//! ends inside the star's exclusion radius. Bodies are small next to a run's
//! distance, so a chase does not route around them.
use super::band_motion::distance;
use crate::LocalPosition;

/// Steps a run is split into for a chase.
pub const STEPS: i64 = 16;

/// Where a chase left the chaser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chase {
    /// The chaser's position at the end of the run.
    pub end: LocalPosition,
    /// It came within reach: it is now alongside the quarry.
    pub caught: bool,
    /// Distance flown, in km (for fuel).
    pub flown_km: i64,
}

/// One run of a chase: the chaser at `from` covers up to `budget_km` while
/// the quarry moves from `seen` to `now`; caught within `reach_km`.
pub fn chase(
    from: LocalPosition,
    budget_km: i64,
    (seen, now): (LocalPosition, LocalPosition),
    reach_km: i64,
    star_exclusion_km: i64,
) -> Chase {
    let step_km = budget_km / STEPS;
    let mut at = from;
    let mut flown_km = 0;
    if distance(at, seen) <= reach_km {
        flown_km += distance(seen, now);
        return alongside(now, flown_km);
    }
    for step in 1..=STEPS {
        let quarry = lerp(seen, now, step, STEPS);
        let gap = distance(at, quarry);
        let moved = gap.min(step_km);
        at = clear_of_star(toward(at, quarry, moved, gap), star_exclusion_km);
        flown_km += moved;
        if distance(at, quarry) <= reach_km {
            // Caught: keep pace with it for the rest of the run.
            flown_km += distance(quarry, now);
            return alongside(now, flown_km);
        }
    }
    Chase {
        end: at,
        caught: false,
        flown_km,
    }
}

/// Caught: alongside the quarry where it ends the run.
fn alongside(now: LocalPosition, flown_km: i64) -> Chase {
    Chase {
        end: now,
        caught: true,
        flown_km,
    }
}

/// `a` moved `moved` km toward `b`, which is `gap` km away.
fn toward(a: LocalPosition, b: LocalPosition, moved: i64, gap: i64) -> LocalPosition {
    if gap == 0 {
        return a;
    }
    lerp(a, b, moved, gap)
}

/// The point `part / whole` of the way from `a` to `b`.
fn lerp(a: LocalPosition, b: LocalPosition, part: i64, whole: i64) -> LocalPosition {
    let mix = |from: i64, to: i64| {
        let offset = i128::from(to - from) * i128::from(part) / i128::from(whole);
        from + i64::try_from(offset).unwrap_or(0)
    };
    LocalPosition {
        x_km: mix(a.x_km, b.x_km),
        y_km: mix(a.y_km, b.y_km),
        z_km: mix(a.z_km, b.z_km),
    }
}

/// Pushed out to the edge of the star's exclusion radius if inside it.
fn clear_of_star(at: LocalPosition, exclusion_km: i64) -> LocalPosition {
    let star = LocalPosition {
        x_km: 0,
        y_km: 0,
        z_km: 0,
    };
    let gap = distance(at, star);
    if gap >= exclusion_km {
        return at;
    }
    if gap == 0 {
        return LocalPosition {
            x_km: exclusion_km,
            ..at
        };
    }
    lerp(star, at, exclusion_km, gap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x_km: i64, z_km: i64) -> LocalPosition {
        LocalPosition {
            x_km,
            y_km: 0,
            z_km,
        }
    }

    const RUN: i64 = 200_000_000;
    const STAR: i64 = 1_000_000;

    #[test]
    fn a_still_quarry_in_range_of_a_run_is_caught() {
        let result = chase(
            at(-150_000_000, 50_000_000),
            RUN,
            (at(0, 50_000_000), at(0, 50_000_000)),
            200_000,
            STAR,
        );
        assert!(result.caught);
        assert_eq!(result.end, at(0, 50_000_000));
        assert!(result.flown_km <= RUN);
    }

    #[test]
    fn a_crossing_quarry_is_cut_off_on_a_curve() {
        // The quarry crosses in front at half speed; the chaser bends to meet it.
        let seen = at(0, -50_000_000);
        let now = at(0, 50_000_000);
        let result = chase(
            at(-120_000_000, -60_000_000),
            RUN,
            (seen, now),
            500_000,
            STAR,
        );
        assert!(result.caught, "{result:?}");
        assert_eq!(result.end, now);
    }

    #[test]
    fn a_slower_chaser_closes_in_over_several_runs() {
        // A band (half speed) after a fleet that has stopped 300 million km away.
        let mut from = at(-150_000_000, 0);
        let target = at(150_000_000, 0);
        let mut runs = 0;
        loop {
            runs += 1;
            let result = chase(from, RUN / 2, (target, target), 500_000, STAR);
            if result.caught {
                break;
            }
            from = result.end;
            assert!(runs < 5);
        }
        assert_eq!(runs, 3);
    }

    #[test]
    fn a_faster_quarry_running_away_is_not_caught() {
        // Half-speed chaser; the quarry flies 290 million km away from it.
        let quarry = (at(0, 110_000_000), at(0, 400_000_000));
        let result = chase(at(0, 100_000_000), RUN / 2, quarry, 500_000, STAR);
        assert!(!result.caught);
        assert_eq!(result.flown_km, RUN / 2);
    }

    #[test]
    fn the_chaser_never_ends_inside_the_star() {
        let result = chase(
            at(-1_500_000, 0),
            RUN,
            (at(1_500_000, 0), at(1_500_000, 0)),
            100,
            2_000_000,
        );
        assert!(distance(result.end, at(0, 0)) >= 2_000_000 || result.caught);
    }
}
