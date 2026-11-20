//! Small seeded starts with a guaranteed market reachable from Sol.
use crate::{GalaxyPosition, StarSystem};

pub fn generate_galaxy(seed: u64) -> Vec<StarSystem> {
    let mut systems = crate::demo_galaxy();
    if seed == 0 {
        return systems;
    } // Stable baseline fixture for reproducible debugging.
    let mut state = seed;
    for system in systems.iter_mut().skip(1) {
        let mut next = || {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = state;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            ((value ^ (value >> 31)) % 5) as i64 - 2
        };
        // Jitter stays on the plane: the vertical coordinate is always 0.
        system.position = GalaxyPosition {
            x: system.position.x + next(),
            y: 0,
            z: system.position.z + next(),
        };
    }
    systems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::travel::TravelState;

    #[test]
    fn seeds_are_reproducible_distinct_and_every_start_has_reachable_markets() {
        assert_eq!(generate_galaxy(42), generate_galaxy(42));
        assert_ne!(generate_galaxy(42), generate_galaxy(43));
        for seed in 0..256 {
            let systems = generate_galaxy(seed);
            let ship = TravelState::default();
            for destination in systems.iter().skip(1) {
                assert!(
                    ship.quote(destination.id, systems[0].position, destination.position)
                        .is_ok(),
                    "seed {seed}"
                );
            }
        }
    }
}
