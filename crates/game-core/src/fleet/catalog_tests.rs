//! The embedded hull catalog: every class is defined once, in `ShipKind::ALL`
//! order, and the prospector fills its niche as the slow, deep-hold miner.
use super::{SHIP_KIND_COUNT, ShipCatalog, ShipKind};
use crate::{
    combat::HullSize,
    planet::{EconomyType, PlanetCatalog},
};

#[test]
fn every_hull_class_has_one_spec_and_a_stable_index() {
    let catalog = ShipCatalog::default();
    assert_eq!(catalog.ships.len(), SHIP_KIND_COUNT);
    for (index, kind) in ShipKind::ALL.into_iter().enumerate() {
        assert_eq!(kind.index(), index);
        assert!(catalog.spec(kind).is_some(), "{kind:?} has a spec");
    }
}

#[test]
fn the_prospector_is_a_slow_large_hull_with_the_deepest_hold() {
    let catalog = ShipCatalog::default();
    let prospector = catalog.spec(ShipKind::Prospector).unwrap();
    assert_eq!(prospector.combat.size, HullSize::Large);
    for other in catalog
        .ships
        .iter()
        .filter(|spec| spec.kind != ShipKind::Prospector)
    {
        assert!(
            prospector.cargo_capacity > other.cargo_capacity,
            "{:?}",
            other.kind
        );
        assert!(
            prospector.combat.engine < other.combat.engine,
            "{:?}",
            other.kind
        );
    }
}

#[test]
fn extraction_worlds_stock_prospectors_and_farm_worlds_do_not() {
    let slot = ShipKind::Prospector.index();
    for template in PlanetCatalog::default().templates {
        match template.profile.economy {
            EconomyType::Extraction => assert!(template.ship_stock[slot] > 0),
            EconomyType::Agricultural => assert_eq!(template.ship_stock[slot], 0),
            EconomyType::Industrial => {}
        }
    }
}

#[test]
fn stock_saved_before_the_prospector_loads_with_none_in_stock() {
    #[derive(serde::Deserialize)]
    struct Yard {
        #[serde(deserialize_with = "super::per_class_stock")]
        stock: [u32; SHIP_KIND_COUNT],
    }
    let legacy: Yard = serde_json::from_str(r#"{"stock":[2,1,1]}"#).unwrap();
    assert_eq!(legacy.stock, [2, 1, 1, 0]);
    let current: Yard = serde_json::from_str(r#"{"stock":[2,1,1,3]}"#).unwrap();
    assert_eq!(current.stock, [2, 1, 1, 3]);
    assert!(serde_json::from_str::<Yard>(r#"{"stock":[1,1]}"#).is_err());
    assert!(serde_json::from_str::<Yard>(r#"{"stock":[1,1,1,1,1]}"#).is_err());
}
