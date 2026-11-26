use super::{Battle, actions::defense_profile};
use crate::{
    combat::{
        CombatCatalog,
        engagement::{CombatantSetup, Side, apply_hit},
        gear::Gear,
    },
    fleet::{ShipCatalog, ShipKind},
};

fn setup(kind: ShipKind, side: Side, gear: Vec<Gear>) -> CombatantSetup {
    CombatantSetup {
        name: format!("{kind:?}"),
        side,
        ship: None,
        kind,
        profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
        hull_damage: 0,
        gear,
    }
}

#[test]
fn size_shields_and_armor_change_battle_defenses() {
    let weapons = CombatCatalog::default();
    let battle = Battle::new(
        &[
            setup(ShipKind::Freighter, Side::Player, vec![Gear::ShieldArray]),
            setup(
                ShipKind::Prospector,
                Side::Hostile,
                vec![Gear::ArmoredBulkheads],
            ),
        ],
        &weapons,
        7,
    )
    .unwrap();
    assert_eq!(
        battle.units[0].shields,
        battle.units[0].profile.shields + 10
    );
    assert!(battle.validate(&weapons).is_ok());
    let target = &battle.units[1];
    let armored = defense_profile(target);
    assert_eq!(armored.armor, target.profile.armor + 3);
    let mut weapon = weapons.weapons[0].clone();
    weapon.damage = 40;
    weapon.armor_penetration = 0;
    let (mut shields, mut plain) = (0, 0);
    let (_, plain_damage) = apply_hit(&mut shields, &mut plain, &target.profile, &weapon);
    let (mut shields, mut armored_damage) = (0, 0);
    let (_, reduced) = apply_hit(&mut shields, &mut armored_damage, &armored, &weapon);
    assert_eq!(plain_damage - reduced, 3);
}

#[test]
fn scout_jets_and_patrol_targeting_change_shot_odds() {
    let weapons = CombatCatalog::default();
    let mut battle = Battle::new(
        &[
            setup(ShipKind::Patrol, Side::Player, vec![]),
            setup(ShipKind::Scout, Side::Hostile, vec![]),
        ],
        &weapons,
        3,
    )
    .unwrap();
    let terrain = std::mem::take(&mut battle.terrain);
    let weapon = weapons.weapon(&battle.units[0].profile.loadout[0]).unwrap();
    let base = battle.shot_chance(0, 1, weapon);
    battle.units[1].gear.push(Gear::EvasionJets);
    assert_eq!(battle.shot_chance(0, 1, weapon), base - 10);
    battle.units[0].gear.push(Gear::PatrolTargeting);
    assert_eq!(battle.shot_chance(0, 1, weapon), base);
    battle.terrain = terrain;
    assert!(battle.validate(&weapons).is_ok());
}
