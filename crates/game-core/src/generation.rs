//! Seeded map layouts with every system reachable from the starting port.
mod bodies;
mod names;
mod positions;
use crate::{GalaxyPosition, StarKind, StarSystem, SystemId};

/// Choices for a newly generated map. Radius is measured from Sol in light-years.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalaxyConfig {
    pub systems: u16,
    pub radius_ly: u16,
    /// Maximum visible objects, including the central star.
    pub max_objects: u8,
}

impl Default for GalaxyConfig {
    fn default() -> Self {
        Self {
            systems: 60,
            radius_ly: 70,
            max_objects: 8,
        }
    }
}

impl GalaxyConfig {
    pub fn validate(self) -> Result<(), String> {
        if !(4..=150).contains(&self.systems)
            || !(20..=95).contains(&self.radius_ly)
            || !(6..=15).contains(&self.max_objects)
        {
            return Err(
                "Galaxy needs 4–150 systems, a 20–95 ly radius and 6–15 objects per system".into(),
            );
        }
        Ok(())
    }
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

const STAR_KINDS: [StarKind; 9] = [
    StarKind::YellowDwarf,
    StarKind::RedDwarf,
    StarKind::BlueGiant,
    StarKind::RedGiant,
    StarKind::WhiteDwarf,
    StarKind::OrangeDwarf,
    StarKind::BlueWhiteStar,
    StarKind::BrownDwarf,
    StarKind::NeutronStar,
];

pub fn generate_galaxy(seed: u64) -> Vec<StarSystem> {
    if seed == 0 {
        return crate::demo_galaxy();
    } // Stable baseline fixture for reproducible debugging.
    let mut state = seed;
    let count = 4 + next(&mut state) % 3;
    generate_galaxy_with(
        seed,
        GalaxyConfig {
            systems: count as u16,
            radius_ly: 20,
            max_objects: 8,
        },
    )
    .expect("small generated map must fit")
}

/// A chosen number of systems spread across a chosen radius. Every system is
/// connected to an earlier one by a hop the starter ship can make.
pub fn generate_galaxy_with(seed: u64, config: GalaxyConfig) -> Result<Vec<StarSystem>, String> {
    config.validate()?;
    let mut systems = crate::demo_galaxy();
    let mut state = seed;
    // The authored systems' and bodies' names are never handed out again.
    let taken: Vec<String> = systems
        .iter()
        .flat_map(|system| {
            std::iter::once(system.name.clone())
                .chain(system.bodies.iter().map(|body| body.name.clone()))
        })
        .collect();
    let mut namer = names::Namer::new(taken.iter().map(String::as_str));
    let count = u64::from(config.systems);
    for id in 4..=count {
        systems.push(StarSystem {
            id: SystemId(id),
            name: namer.place(&mut state),
            star_kind: StarKind::YellowDwarf,
            position: GalaxyPosition { x: 0, y: 0, z: 0 },
            bodies: Vec::new(),
        });
    }
    let mut positions = vec![systems[0].position];
    for system in systems.iter_mut().skip(1) {
        system.position = positions::place(&positions, config, &mut state)?;
        positions.push(system.position);
        system.star_kind = STAR_KINDS[next(&mut state) as usize % STAR_KINDS.len()];
        system.bodies = bodies::generate(&mut namer, config.max_objects - 1, &mut state);
    }
    // The authored home market and its guide remain a stable starting point.
    crate::planet::PlanetCatalog::default().populate(&mut systems);
    Ok(systems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BodyKind;
    use crate::travel::TravelState;

    #[test]
    fn seeds_are_reproducible_distinct_and_every_start_has_reachable_markets() {
        assert_eq!(generate_galaxy(0), crate::demo_galaxy());
        assert_eq!(generate_galaxy(42), generate_galaxy(42));
        assert_ne!(generate_galaxy(42), generate_galaxy(43));
        assert_eq!(generate_galaxy(42)[0], generate_galaxy(0)[0]);
        assert!((4..=6).contains(&generate_galaxy(42).len()));
        for seed in 0..1024 {
            let systems = generate_galaxy(seed);
            let ship = TravelState::default();
            ship.movement_rules.validate_world(&systems).unwrap();
            for destination in systems.iter().skip(1) {
                assert!(
                    ship.quote(destination.id, systems[0].position, destination.position)
                        .is_ok(),
                    "seed {seed}"
                );
            }
        }
    }

    #[test]
    fn generated_systems_vary_their_stars_and_body_lists() {
        let maps: Vec<_> = (1..=64).map(generate_galaxy).collect();
        assert!(maps.iter().any(|map| map.len() == 4));
        assert!(maps.iter().any(|map| map.len() == 6));
        assert!(maps.iter().any(|map| {
            map.iter()
                .skip(1)
                .any(|system| system.star_kind == StarKind::WhiteDwarf)
        }));
        assert!(maps.iter().any(|map| {
            map.iter()
                .skip(1)
                .any(|system| system.bodies.iter().any(|body| body.kind == BodyKind::Moon))
        }));
        assert!(maps.iter().any(|map| {
            map.iter().skip(1).any(|system| {
                system
                    .bodies
                    .iter()
                    .any(|body| body.kind == BodyKind::Asteroid)
            })
        }));
        for map in maps {
            for system in map.iter().skip(1) {
                assert_eq!(
                    system
                        .bodies
                        .iter()
                        .filter(|body| body.kind == BodyKind::Station)
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn dense_generated_systems_vary_planet_biomes_within_a_system() {
        let systems = generate_galaxy_with(
            42,
            GalaxyConfig {
                systems: 150,
                radius_ly: 95,
                max_objects: 15,
            },
        )
        .unwrap();
        assert!(systems.iter().skip(3).any(|system| {
            let rocky: Vec<_> = system
                .bodies
                .iter()
                .filter(|body| body.kind == BodyKind::RockyPlanet)
                .filter_map(|body| body.planet.as_ref().map(|planet| planet.biome))
                .collect();
            rocky
                .iter()
                .any(|biome| rocky.iter().any(|other| other != biome))
        }));
    }

    #[test]
    fn a_large_galaxy_is_connected_and_within_its_selected_radius() {
        let config = GalaxyConfig {
            systems: 150,
            radius_ly: 95,
            max_objects: 8,
        };
        for seed in [1, 42, u64::MAX] {
            let systems = generate_galaxy_with(seed, config).unwrap();
            assert_eq!(systems.len(), 150);
            TravelState::default()
                .movement_rules
                .validate_world(&systems)
                .unwrap();
            let mut reached = vec![false; systems.len()];
            reached[0] = true;
            loop {
                let before = reached.iter().filter(|value| **value).count();
                for (index, system) in systems.iter().enumerate() {
                    if reached[index] {
                        continue;
                    }
                    reached[index] = systems.iter().enumerate().any(|(other, known)| {
                        reached[other] && {
                            let dx = system.position.x - known.position.x;
                            let dz = system.position.z - known.position.z;
                            dx * dx + dz * dz <= 20 * 20
                        }
                    });
                }
                if reached.iter().filter(|value| **value).count() == before {
                    break;
                }
            }
            assert!(reached.iter().all(|value| *value), "seed {seed}");
            assert!(systems.iter().all(|system| {
                system.position.x.pow(2) + system.position.z.pow(2) <= 95_i64.pow(2)
            }));
        }
    }

    #[test]
    fn dense_and_sparse_size_choices_generate_valid_worlds() {
        for config in [
            GalaxyConfig {
                systems: 150,
                radius_ly: 20,
                max_objects: 8,
            },
            GalaxyConfig {
                systems: 4,
                radius_ly: 95,
                max_objects: 8,
            },
        ] {
            for seed in (1..=64).chain([u64::MAX]) {
                let systems = generate_galaxy_with(seed, config)
                    .unwrap_or_else(|error| panic!("seed {seed}, {config:?}: {error}"));
                TravelState::default()
                    .movement_rules
                    .validate_world(&systems)
                    .unwrap();
            }
        }
    }

    #[test]
    fn selected_body_cap_preserves_orbital_clearance() {
        for cap in [6, 15] {
            let config = GalaxyConfig {
                systems: 30,
                radius_ly: 50,
                max_objects: cap,
            };
            for seed in 1..=32 {
                let systems = generate_galaxy_with(seed, config).unwrap();
                TravelState::default()
                    .movement_rules
                    .validate_world(&systems)
                    .unwrap();
                for system in systems.iter().skip(1) {
                    assert!((5..usize::from(cap)).contains(&system.bodies.len()));
                }
                if cap == 15 && seed == 1 {
                    assert!(systems.iter().any(|system| system.bodies.len() == 14));
                }
            }
        }
    }
}
