use super::engagement::{MIN_HIT_PERCENT, apply_hit, hit_chance};
use super::{CombatCatalog, CombatProfile, HullSize, WeaponSpec};

fn weapon(damage: u32, efficiency: u32, pierce: u32) -> WeaponSpec {
    WeaponSpec {
        damage,
        shield_efficiency_percent: efficiency,
        armor_penetration: pierce,
        ..CombatCatalog::default().weapons[0].clone()
    }
}

fn target(hull: u32, armor: u32) -> CombatProfile {
    CombatProfile {
        size: HullSize::Medium,
        hull,
        shields: 0,
        armor,
        engine: 1,
        weapon_slots: 0,
        loadout: Vec::new(),
    }
}

#[test]
fn shields_absorb_then_overflow_converts_back_and_armor_reduces() {
    // 20 damage at 200% vs 10 shields: 5 raw breaks shields, 15 raw passes,
    // armor 6 − pierce 2 = 4 → 11 hull.
    let (mut shields, mut damage) = (10, 0);
    let lance = weapon(20, 200, 2);
    let profile = target(100, 6);
    assert_eq!(
        apply_hit(&mut shields, &mut damage, &profile, &lance),
        (10, 11)
    );
    // Shields stay down: the next hit is 20 − 4 = 16.
    assert_eq!(
        apply_hit(&mut shields, &mut damage, &profile, &lance),
        (0, 16)
    );
    assert_eq!(damage, 27);
}

#[test]
fn shields_that_hold_take_scaled_damage_and_armored_hits_deal_at_least_one() {
    let pin = weapon(3, 50, 0);
    let profile = target(100, 50);
    let (mut shields, mut damage) = (4, 0);
    // 3 × 50% = 1 shield point; nothing reaches the hull while shields hold.
    assert_eq!(apply_hit(&mut shields, &mut damage, &profile, &pin), (1, 0));
    shields = 0;
    // Once shields are gone, armor 50 still lets one point through.
    assert_eq!(apply_hit(&mut shields, &mut damage, &profile, &pin), (0, 1));
}

#[test]
fn hull_damage_never_exceeds_the_remaining_hull() {
    let (mut shields, mut damage) = (0, 25);
    let cannon = weapon(40, 100, 0);
    assert_eq!(
        apply_hit(&mut shields, &mut damage, &target(30, 0), &cannon),
        (0, 5)
    );
    assert_eq!(damage, 30);
}

#[test]
fn evasion_and_distance_lower_hit_chance_down_to_a_floor() {
    let beam = weapon(10, 100, 0);
    let accuracy = beam.accuracy_percent;
    assert_eq!(hit_chance(&beam, 0, 1), accuracy, "adjacent, no evasion");
    assert_eq!(hit_chance(&beam, 2, 1), accuracy - 6, "3% per engine point");
    assert_eq!(hit_chance(&beam, 0, 4), accuracy - 12, "4% per extra cell");
    assert_eq!(hit_chance(&beam, 10, 30), MIN_HIT_PERCENT);
}
