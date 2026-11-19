use super::engagement::{CombatantSetup, MAX_ROUNDS, Side, resolve};
use super::{CombatCatalog, CombatProfile, HullSize};
use crate::fleet::{ShipCatalog, ShipKind};

fn setup(kind: ShipKind, side: Side, name: &str) -> CombatantSetup {
    CombatantSetup {
        name: name.into(),
        side,
        ship: None,
        kind,
        profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
        hull_damage: 0,
    }
}

/// A perfectly accurate test weapon so damage arithmetic is roll-independent.
fn catalog_with(id: &str, damage: u32, efficiency: u32, pierce: u32) -> CombatCatalog {
    let mut catalog = CombatCatalog::default();
    let mut weapon = catalog.weapons[0].clone();
    weapon.id = id.into();
    weapon.damage = damage;
    weapon.shots = 1;
    weapon.accuracy_percent = 100;
    weapon.shield_efficiency_percent = efficiency;
    weapon.armor_penetration = pierce;
    catalog.weapons.push(weapon);
    catalog
}

fn dummy(side: Side, hull: u32, shields: u32, armor: u32, loadout: &[&str]) -> CombatantSetup {
    CombatantSetup {
        name: format!("{side:?}"),
        side,
        ship: None,
        kind: ShipKind::Freighter,
        profile: CombatProfile {
            size: HullSize::Medium,
            hull,
            shields,
            armor,
            engine: 1,
            weapon_slots: 3,
            loadout: loadout.iter().map(|id| (*id).to_string()).collect(),
        },
        hull_damage: 0,
    }
}

#[test]
fn identical_seeds_reproduce_identical_engagements() {
    let catalog = CombatCatalog::default();
    let ships = [
        setup(ShipKind::Patrol, Side::Player, "Patrol"),
        setup(ShipKind::Freighter, Side::Player, "Freighter"),
        setup(ShipKind::Scout, Side::Hostile, "Raider A"),
        setup(ShipKind::Scout, Side::Hostile, "Raider B"),
    ];
    let first = resolve(&ships, &catalog, 42).unwrap();
    assert_eq!(first, resolve(&ships, &catalog, 42).unwrap());
    let differs = (0..20).any(|seed| resolve(&ships, &catalog, seed).unwrap() != first);
    assert!(differs, "the seed must influence rolls");
}

#[test]
fn shields_absorb_then_overflow_converts_back_and_armor_reduces() {
    // 20 damage at 200% vs 10 shields: 5 raw breaks shields, 15 raw passes,
    // armor 6 − pierce 2 = 4 → 11 hull.
    let catalog = catalog_with("test_lance", 20, 200, 2);
    let ships = [
        dummy(Side::Player, 50, 0, 0, &["test_lance"]),
        dummy(Side::Hostile, 100, 10, 6, &[]),
    ];
    let result = resolve(&ships, &catalog, 1).unwrap();
    let first = &result.rounds[0][0];
    assert!(first.hit);
    assert_eq!((first.shield_damage, first.hull_damage), (10, 11));
    // Shields stay down for the engagement: the next hit is 20 − 4 = 16.
    assert_eq!(result.rounds[1][0].hull_damage, 16);
}

#[test]
fn shields_that_hold_take_scaled_damage_and_armored_hits_deal_at_least_one() {
    let catalog = catalog_with("test_pin", 3, 50, 0);
    let ships = [
        dummy(Side::Player, 50, 0, 0, &["test_pin"]),
        dummy(Side::Hostile, 100, 4, 50, &[]),
    ];
    let result = resolve(&ships, &catalog, 1).unwrap();
    // 3 × 50% = 1 shield point; nothing reaches the hull while shields hold.
    assert_eq!(
        (
            result.rounds[0][0].shield_damage,
            result.rounds[0][0].hull_damage
        ),
        (1, 0)
    );
    // Once shields are gone, armor 50 still lets one point through.
    let later = result
        .rounds
        .iter()
        .flatten()
        .find(|shot| shot.hull_damage > 0);
    assert_eq!(later.map(|shot| shot.hull_damage), Some(1));
}

#[test]
fn fire_focuses_on_the_weakest_enemy_and_destroyed_ships_stop_acting() {
    let catalog = catalog_with("test_cannon", 40, 100, 0);
    let mut ships = [
        dummy(Side::Player, 200, 0, 0, &["test_cannon"]),
        dummy(Side::Hostile, 80, 0, 0, &["test_cannon"]),
        dummy(Side::Hostile, 30, 0, 0, &["test_cannon"]),
    ];
    ships[0].profile.engine = 2; // act first regardless of initiative
    let result = resolve(&ships, &catalog, 7).unwrap();
    let opening = &result.rounds[0][0];
    assert_eq!((opening.attacker, opening.target), (0, 2));
    assert!(opening.destroyed);
    // The destroyed raider (index 2) never fires.
    assert!(
        result
            .rounds
            .iter()
            .flatten()
            .all(|shot| shot.attacker != 2)
    );
    assert_eq!(result.winner, Some(Side::Player));
    let wreck = &result.combatants[2];
    assert!(wreck.destroyed);
    assert_eq!(
        wreck.final_damage, wreck.hull,
        "damage is capped at the hull"
    );
}

#[test]
fn evenly_matched_unarmed_fleets_disengage_at_the_round_limit() {
    let catalog = CombatCatalog::default();
    let ships = [
        dummy(Side::Player, 10, 0, 0, &[]),
        dummy(Side::Hostile, 10, 0, 0, &[]),
    ];
    let result = resolve(&ships, &catalog, 3).unwrap();
    assert_eq!(result.rounds.len(), MAX_ROUNDS as usize);
    assert_eq!(result.winner, None);
}

#[test]
fn invalid_engagements_are_rejected() {
    let catalog = CombatCatalog::default();
    assert!(resolve(&[], &catalog, 0).is_err());
    let mut wrecked = setup(ShipKind::Scout, Side::Player, "Wreck");
    wrecked.hull_damage = wrecked.profile.hull;
    assert!(resolve(&[wrecked], &catalog, 0).is_err());
}

#[test]
fn engine_order_lets_faster_ships_fire_first() {
    let catalog = CombatCatalog::default();
    let ships = [
        setup(ShipKind::Freighter, Side::Player, "Freighter"),
        setup(ShipKind::Scout, Side::Hostile, "Raider"),
    ];
    let result = resolve(&ships, &catalog, 11).unwrap();
    assert_eq!(result.rounds[0][0].attacker, 1, "scout engine 8 beats 3");
}

fn player_wins(player: &[ShipKind], hostile: &[ShipKind]) -> (u32, u32) {
    let catalog = CombatCatalog::default();
    let mut ships: Vec<_> = player
        .iter()
        .map(|k| setup(*k, Side::Player, "P"))
        .collect();
    ships.extend(hostile.iter().map(|k| setup(*k, Side::Hostile, "H")));
    (0..400).fold((0, 0), |(wins, losses), seed| {
        match resolve(&ships, &catalog, seed).unwrap().winner {
            Some(Side::Player) => (wins + 1, losses),
            Some(Side::Hostile) => (wins, losses + 1),
            None => (wins, losses),
        }
    })
}

#[test]
fn mirror_matches_are_fair_and_a_lone_trader_survives_a_scout_raider() {
    for kind in [ShipKind::Scout, ShipKind::Patrol] {
        let (wins, losses) = player_wins(&[kind], &[kind]);
        let share = wins * 100 / (wins + losses);
        assert!((40..=60).contains(&share), "{kind:?} mirror: {share}%");
    }
    let (_, losses) = player_wins(&[ShipKind::Freighter], &[ShipKind::Scout]);
    assert_eq!(
        losses, 0,
        "the starting freighter must not lose to one scout"
    );
}
