use crate::constants::MAX_COMBAT_TIME;
use crate::models::ship::ship::Ship;
use rand::Rng;

pub fn auto_resolve_ship_combat(
    attackers: &mut Vec<Ship>,
    defenders: &mut Vec<Ship>,
) -> CombatResult {
    let mut combat_tic = 0;

    while combat_tic < MAX_COMBAT_TIME {
        combat_tic += 1;

        let defenders_len = defenders.len();
        let attackers_len = attackers.len();

        for attacker in attackers.iter_mut() {
            if attacker.hp <= 0 {
                continue;
            }

            let target_index = rand::thread_rng().gen_range(0..defenders_len);
            let mut target = &mut defenders[target_index];

            let total_damage = attacker.weapons.iter().map(|w| w.damage()).sum::<i32>();
            let shield_damage = target
                .shields
                .calculate_damage(total_damage, target.shields.strength);
            let damage = target.armor.calculate_damage(shield_damage);

            target.hp -= damage;

            println!(
                "Attacker: {} attacks {} and deals {} damage. Target's HP: {} Shield: {} Armor: {}",
                attacker.name,
                target.name,
                total_damage,
                target.hp,
                target.shields.strength,
                target.armor.durability
            );
        }

        attackers.retain(|ship| ship.hp > 0);
        defenders.retain(|ship| ship.hp > 0);

        if attackers.is_empty() && !defenders.is_empty() {
            return CombatResult::DefendersVictory(defenders.clone());
        } else if !attackers.is_empty() && defenders.is_empty() {
            return CombatResult::AttackersVictory(attackers.clone());
        } else if attackers.is_empty() && defenders.is_empty() {
            return CombatResult::TotalDestruction();
        }

        for defender in defenders.iter_mut() {
            if defender.hp <= 0 {
                continue;
            }

            let target_index = rand::thread_rng().gen_range(0..attackers_len);
            let mut target = &mut attackers[target_index];

            let total_damage = defender.weapons.iter().map(|w| w.damage()).sum::<i32>();
            let shield_damage = target
                .shields
                .calculate_damage(total_damage, target.shields.strength);
            let damage = target.armor.calculate_damage(shield_damage);

            target.hp -= damage;

            println!(
                "Defender {} fires at {} and deals {} damage. Target's HP: {} Shield: {} Armor: {}",
                defender.name,
                target.name,
                total_damage,
                target.hp,
                target.shields.strength,
                target.armor.durability
            );
        }

        attackers.retain(|ship| ship.hp > 0);
        defenders.retain(|ship| ship.hp > 0);

        if attackers.is_empty() && !defenders.is_empty() {
            return CombatResult::DefendersVictory(defenders.clone());
        } else if !attackers.is_empty() && defenders.is_empty() {
            return CombatResult::AttackersVictory(attackers.clone());
        } else if attackers.is_empty() && defenders.is_empty() {
            return CombatResult::TotalDestruction();
        }
    }
    println!("Auto combat timed out");
    let combat_result = CombatResult::TimedOut(attackers.clone(), defenders.clone());
    combat_result
}

pub enum CombatResult {
    AttackersVictory(Vec<Ship>),
    DefendersVictory(Vec<Ship>),
    TotalDestruction(),
    TimedOut(Vec<Ship>, Vec<Ship>),
}
