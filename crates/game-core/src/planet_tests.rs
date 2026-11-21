use crate::{economy::EconomyRules, planet::PlanetCatalog};

#[test]
fn planet_configuration_rejects_missing_classes_invalid_biomes_and_excessive_allocations() {
    let valid = serde_json::to_value(PlanetCatalog::default()).unwrap();
    for (field, value) in [
        ("starting_credits", serde_json::json!(u64::MAX)),
        ("market_template", serde_json::json!(64)),
        ("ship_stock", serde_json::json!([1001, 0, 0])),
        ("kind", serde_json::json!("Moon")),
    ] {
        let mut invalid = valid.clone();
        invalid["templates"][0][field] = value;
        assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    }
    let mut invalid = valid.clone();
    invalid["templates"]
        .as_array_mut()
        .unwrap()
        .retain(|template| template["kind"] == "RockyPlanet");
    assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    let mut invalid = valid;
    invalid["templates"][0]["profile"]["biome"] = serde_json::json!("gas");
    assert!(PlanetCatalog::from_json(&invalid.to_string()).is_err());
    let mut rules = EconomyRules {
        ship_resale_percent: 101,
        ..EconomyRules::default()
    };
    assert!(rules.validate().is_err());
    rules.ship_resale_percent = 60;
    rules.markets.truncate(1);
    assert!(rules.validate().is_ok());
    assert!(crate::economy::Economy::with_rules(&crate::demo_galaxy(), rules).is_ok());
}

#[test]
fn every_moon_and_asteroid_gets_a_surface_and_every_variant_appears() {
    use crate::{BodyKind, planet::Biome};
    let mut seen = Vec::new();
    for seed in 0..20 {
        let mut systems = crate::generation::generate_galaxy(seed);
        PlanetCatalog::default().populate(&mut systems);
        for body in systems.iter().flat_map(|system| &system.bodies) {
            let surfaced = matches!(body.kind, BodyKind::Moon | BodyKind::Asteroid);
            assert_eq!(body.surface.is_some(), surfaced, "{}", body.name);
            assert!(
                !(body.planet.is_some() && surfaced),
                "no market profile on {}",
                body.name
            );
            if let Some(surface) = &body.surface {
                seen.push((body.kind, surface.biome));
                assert_eq!(body.danger(), Some(surface.danger));
            }
        }
    }
    for variant in [
        (BodyKind::Moon, Biome::Barren),
        (BodyKind::Moon, Biome::Frozen),
        (BodyKind::Asteroid, Biome::Barren),
        (BodyKind::Asteroid, Biome::Metallic),
    ] {
        assert!(seen.contains(&variant), "{variant:?} never generated");
    }
}

#[test]
fn surface_templates_reject_planets_gas_and_extreme_richness() {
    let valid = serde_json::to_value(PlanetCatalog::default()).unwrap();
    for (path, value) in [
        ("kind", serde_json::json!("RockyPlanet")),
        ("biome", serde_json::json!("gas")),
        ("ore_percent", serde_json::json!(0)),
    ] {
        let mut invalid = valid.clone();
        let surface = &mut invalid["surfaces"][0];
        if path == "kind" {
            surface[path] = value;
        } else {
            surface["profile"][path] = value;
        }
        assert!(
            PlanetCatalog::from_json(&invalid.to_string()).is_err(),
            "{path}"
        );
    }
    let rich = crate::planet::SurfaceProfile {
        biome: crate::planet::Biome::Metallic,
        danger: crate::planet::Danger::Dangerous,
        ore_percent: 200,
    };
    assert_eq!(rich.ore_per_ship(2), 4);
    assert_eq!(rich.ore_per_ship(0), 0, "no base yield, no ore");
    let poor = crate::planet::SurfaceProfile {
        ore_percent: 10,
        ..rich
    };
    assert_eq!(poor.ore_per_ship(1), 1, "at least 1 where there is a yield");
}
