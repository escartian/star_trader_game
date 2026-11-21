use super::*;
use crate::generation::{GalaxyConfig, generate_galaxy_with};

fn galaxy(seed: u64) -> Vec<crate::StarSystem> {
    generate_galaxy_with(seed, GalaxyConfig::default()).unwrap()
}

#[test]
fn the_embedded_catalog_lists_the_four_factions_in_order() {
    let catalog = FactionCatalog::default();
    let ids: Vec<_> = catalog.factions.iter().map(|faction| faction.id).collect();
    assert_eq!(ids, Faction::ALL);
}

#[test]
fn territory_is_deterministic_claims_the_coverage_and_spares_home() {
    let catalog = FactionCatalog::default();
    let systems = galaxy(7);
    let setup = FactionSetup::default();
    let factions = setup.generate(7, &systems, &catalog).unwrap();
    assert_eq!(factions, setup.generate(7, &systems, &catalog).unwrap());
    factions.validate(&systems, &catalog).unwrap();
    assert_eq!(factions.owner(systems[0].id), None, "home is independent");
    let claimed = factions.territory.len();
    let quota = (systems.len() * 70).div_ceil(100);
    assert!(
        claimed + 1 >= quota && claimed <= quota,
        "{claimed} of {quota}"
    );
    for faction in Faction::ALL {
        assert!(factions.influence_percent(faction, systems.len()) > 0);
        assert_eq!(factions.standing(faction), 0);
    }
    let full = FactionSetup {
        coverage: Coverage::Full,
        ..setup
    };
    let all = full.generate(7, &systems, &catalog).unwrap();
    assert_eq!(all.territory.len(), systems.len() - 1);
}

#[test]
fn allegiance_puts_the_capital_at_home_and_spreads_standing() {
    let catalog = FactionCatalog::default();
    let systems = galaxy(3);
    let setup = FactionSetup {
        allegiance: Some(Faction::Federation),
        ..FactionSetup::default()
    };
    let factions = setup.generate(3, &systems, &catalog).unwrap();
    assert_eq!(factions.owner(systems[0].id), Some(Faction::Federation));
    // Classic: the Republic is the Federation's ally, the Empire its rival.
    assert_eq!(factions.standing(Faction::Federation), 30);
    assert_eq!(factions.standing(Faction::Republic), 15);
    assert_eq!(factions.standing(Faction::Empire), -15);
    assert_eq!(factions.standing(Faction::Alliance), 0);
    assert_eq!(factions.tier(Faction::Federation), Tier::Friendly);
    assert_eq!(factions.tier(Faction::Empire), Tier::Unfriendly);
    factions.validate(&systems, &catalog).unwrap();
}

#[test]
fn factions_out_of_play_hold_nothing_and_none_turns_factions_off() {
    let catalog = FactionCatalog::default();
    let systems = galaxy(5);
    let two = FactionSetup {
        in_play: [true, false, true, false],
        ..FactionSetup::default()
    };
    let factions = two.generate(5, &systems, &catalog).unwrap();
    assert!(
        factions
            .territory
            .iter()
            .all(|(_, faction)| matches!(faction, Faction::Federation | Faction::Republic))
    );
    let mut factions = factions;
    assert!(factions.adjust(Faction::Empire, -20).is_empty());
    let off = FactionSetup {
        in_play: [false; 4],
        ..FactionSetup::default()
    };
    let none = off.generate(5, &systems, &catalog).unwrap();
    assert!(!none.is_active() && none.territory.is_empty());
    let disloyal = FactionSetup {
        allegiance: Some(Faction::Empire),
        ..two
    };
    assert!(disloyal.generate(5, &systems, &catalog).is_err());
}

#[test]
fn standing_spreads_to_allies_and_rivals_and_stays_in_range() {
    let catalog = FactionCatalog::default();
    let systems = galaxy(1);
    let mut factions = FactionSetup::default()
        .generate(1, &systems, &catalog)
        .unwrap();
    let changes = factions.adjust(Faction::Federation, -25);
    assert_eq!(
        changes,
        vec![
            (Faction::Federation, -25),
            (Faction::Empire, 12),
            (Faction::Republic, -12)
        ]
    );
    for _ in 0..10 {
        factions.adjust(Faction::Federation, -25);
    }
    assert_eq!(factions.standing(Faction::Federation), MIN_STANDING);
    assert_eq!(factions.tier(Faction::Federation), Tier::Hostile);
    factions.validate(&systems, &catalog).unwrap();
}

#[test]
fn sales_earn_standing_with_the_ports_holder_point_by_point() {
    let catalog = FactionCatalog::default();
    let systems = galaxy(2);
    let mut factions = FactionSetup {
        relations: RelationMode::Random,
        ..FactionSetup::default()
    }
    .generate(2, &systems, &catalog)
    .unwrap();
    let (system, holder) = factions.territory[0];
    let per_point = catalog.get(holder).sale_credits_per_point;
    assert!(
        factions
            .record_sale(system, per_point - 1, &catalog)
            .is_empty()
    );
    // With the first sale's remainder this makes three points exactly.
    let changes = factions.record_sale(system, per_point * 2 + 1, &catalog);
    assert!(changes.contains(&(holder, 3)), "{changes:?}");
    assert_eq!(factions.sales_cents[holder.index()], 0);
    assert!(
        factions
            .record_sale(systems[0].id, 10_000_000, &catalog)
            .is_empty()
    );
}

#[test]
fn terms_scale_prices_by_tier_and_close_hostile_ports() {
    let at = |tier| PortTerms {
        holder: Some((Faction::Empire, tier)),
    };
    assert_eq!(PortTerms::LIST.buying(1000), 1000);
    assert_eq!(at(Tier::Honored).buying(1000), 900);
    assert_eq!(at(Tier::Honored).selling(1000), 1100);
    assert_eq!(
        at(Tier::Friendly).buying(999),
        950,
        "rounded up for you to pay"
    );
    assert_eq!(
        at(Tier::Unfriendly).selling(999),
        899,
        "rounded down for you to receive"
    );
    assert_eq!(at(Tier::Hostile).service(1000), 1500);
    assert!(at(Tier::Hostile).closed() && !at(Tier::Unfriendly).closed());
    assert_eq!(Tier::of(-50), Tier::Hostile);
    assert_eq!(Tier::of(-15), Tier::Unfriendly);
    assert_eq!(Tier::of(14), Tier::Neutral);
    assert_eq!(Tier::of(50), Tier::Honored);
}
