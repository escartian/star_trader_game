use super::{
    Battle, BattleError, CEASEFIRE_ROUNDS, Cell, MAX_ROUNDS, auto_resolve, home_edge, move_range,
};
use crate::combat::{
    CombatCatalog, CombatProfile, HullSize,
    engagement::{CombatantSetup, Side},
};
use crate::fleet::{ShipCatalog, ShipKind};

fn ship(kind: ShipKind, side: Side) -> CombatantSetup {
    CombatantSetup {
        name: format!("{side:?} {kind:?}"),
        side,
        ship: None,
        kind,
        profile: ShipCatalog::default().spec(kind).unwrap().combat.clone(),
        hull_damage: 0,
    }
}

fn unarmed(side: Side) -> CombatantSetup {
    CombatantSetup {
        profile: CombatProfile {
            size: HullSize::Medium,
            hull: 10,
            shields: 0,
            armor: 0,
            engine: 1,
            weapon_slots: 0,
            loadout: Vec::new(),
        },
        ..ship(ShipKind::Freighter, side)
    }
}

fn battle(setups: &[CombatantSetup], seed: u64) -> Battle {
    Battle::new(setups, &CombatCatalog::default(), seed).unwrap()
}

#[test]
fn sides_deploy_on_opposite_columns_in_unique_cells() {
    let setups: Vec<_> = (0..4)
        .map(|_| ship(ShipKind::Scout, Side::Player))
        .chain((0..3).map(|_| ship(ShipKind::Scout, Side::Hostile)))
        .collect();
    let battle = battle(&setups, 1);
    let cells: std::collections::HashSet<_> = battle.units.iter().map(|unit| unit.cell).collect();
    assert_eq!(cells.len(), 7);
    for unit in &battle.units {
        assert!(unit.cell.in_bounds());
        let column = if unit.side == Side::Player {
            1
        } else {
            super::WIDTH - 2
        };
        assert_eq!(unit.cell.x, column);
    }
    // Checkerboard: no two ships of a side share a face.
    for a in &battle.units {
        for b in &battle.units {
            let face = (a.cell.y - b.cell.y).abs() + (a.cell.z - b.cell.z).abs() == 1;
            assert!(
                !(a.side == b.side && face),
                "{:?} touches {:?}",
                a.cell,
                b.cell
            );
        }
    }
}

#[test]
fn faster_ships_act_first() {
    let battle = battle(
        &[
            ship(ShipKind::Freighter, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        3,
    );
    assert_eq!(battle.active(), Some(1), "scout engine 8 beats freighter 3");
}

#[test]
fn moves_respect_engine_range_free_cells_and_one_move_per_turn() {
    let mut battle = battle(
        &[
            ship(ShipKind::Freighter, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        3,
    );
    let me = battle.active().unwrap();
    let from = battle.units[me].cell;
    let reach = move_range(battle.units[me].profile.engine) as i8;
    let far = Cell {
        x: from.x - reach - 1,
        ..from
    };
    assert_eq!(battle.move_to(far), Err(BattleError::TooFar));
    let outside = Cell { z: -1, ..from };
    assert_eq!(battle.move_to(outside), Err(BattleError::Blocked));
    let occupied = battle.units[1 - me].cell;
    assert_eq!(battle.move_to(occupied), Err(BattleError::Blocked));
    let step = Cell {
        x: from.x - reach,
        ..from
    };
    assert!(battle.reachable().contains(&step));
    battle.move_to(step).unwrap();
    assert_eq!(battle.units[me].cell, step);
    assert!(battle.reachable().is_empty());
    assert_eq!(battle.move_to(from), Err(BattleError::AlreadyMoved));
}

#[test]
fn fire_needs_an_enemy_in_weapon_range_and_happens_once_per_turn() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    // Put the player first in line, 7 cells from the raider: beyond every weapon.
    battle.order = vec![0, 1];
    battle.units[1].cell = Cell { x: 8, y: 4, z: 2 };
    assert_eq!(
        battle.fire(0, &catalog),
        Err(BattleError::InvalidTarget),
        "own ship"
    );
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::OutOfRange));
    battle.units[0].cell = Cell { x: 4, y: 4, z: 2 };
    assert_eq!(battle.targets(0, &catalog), vec![1]);
    battle.fire(1, &catalog).unwrap();
    let shots = &battle.rounds[0];
    assert!(
        !shots.is_empty()
            && shots
                .iter()
                .all(|shot| (shot.attacker, shot.target) == (0, 1))
    );
    // Distance 4: the photon beam and torpedo reach; the 3-cell disruptor does not.
    assert!(
        shots
            .iter()
            .all(|shot| shot.weapon != "magnetic_resonance_disruptor")
    );
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::AlreadyFired));
}

#[test]
fn ships_withdraw_only_from_their_own_edge_and_a_full_withdrawal_disengages() {
    let mut battle = battle(
        &[
            ship(ShipKind::Scout, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        9,
    );
    battle.order = vec![0, 1];
    assert_eq!(battle.withdraw(), Err(BattleError::NotAtEdge));
    let edge = Cell {
        x: home_edge(Side::Player),
        ..battle.units[0].cell
    };
    battle.move_to(edge).unwrap();
    battle.withdraw().unwrap();
    assert!(battle.units[0].withdrawn && battle.units[0].alive());
    assert!(battle.finished());
    assert_eq!(battle.outcome(), Some(None), "both sides still have ships");
    assert_eq!(battle.end_turn(), Err(BattleError::Finished));
}

#[test]
fn fleets_that_stop_firing_disengage_after_a_ceasefire() {
    let result = auto_resolve(
        &[unarmed(Side::Player), unarmed(Side::Hostile)],
        &CombatCatalog::default(),
        4,
    )
    .unwrap();
    // The ceasefire rounds, plus the round that had just begun when it ended.
    assert_eq!(result.rounds.len(), CEASEFIRE_ROUNDS as usize + 1);
    assert_eq!(result.winner, None);
}

#[test]
fn destroyed_ships_stop_acting_and_identical_seeds_replay_identically() {
    let catalog = CombatCatalog::default();
    let setups = [
        ship(ShipKind::Patrol, Side::Player),
        ship(ShipKind::Freighter, Side::Player),
        ship(ShipKind::Scout, Side::Hostile),
        ship(ShipKind::Scout, Side::Hostile),
    ];
    let first = auto_resolve(&setups, &catalog, 42).unwrap();
    assert_eq!(first, auto_resolve(&setups, &catalog, 42).unwrap());
    assert!((0..20).any(|seed| auto_resolve(&setups, &catalog, seed).unwrap() != first));
    for (round, shots) in first.rounds.iter().enumerate() {
        for (index, shot) in shots.iter().enumerate() {
            let later = &shots[index + 1..];
            if shot.destroyed {
                assert!(
                    later.iter().all(|next| next.attacker != shot.target),
                    "round {round}: a destroyed ship fired"
                );
            }
        }
    }
    assert!(Battle::new(&[], &catalog, 0).is_err());
    let mut wreck = ship(ShipKind::Scout, Side::Player);
    wreck.hull_damage = wreck.profile.hull;
    assert!(Battle::new(&[wreck], &catalog, 0).is_err());
}

fn player_record(player: &[ShipKind], hostile: &[ShipKind]) -> (u32, u32) {
    let catalog = CombatCatalog::default();
    let mut setups: Vec<_> = player
        .iter()
        .map(|kind| ship(*kind, Side::Player))
        .collect();
    setups.extend(hostile.iter().map(|kind| ship(*kind, Side::Hostile)));
    (0..400).fold((0, 0), |(wins, losses), seed| {
        match auto_resolve(&setups, &catalog, seed).unwrap().winner {
            Some(Side::Player) => (wins + 1, losses),
            Some(Side::Hostile) => (wins, losses + 1),
            None => (wins, losses),
        }
    })
}

#[test]
fn the_largest_battles_end_decisively_far_below_the_safety_cap() {
    // 8 patrols a side is the most a battle holds; 2,000 seeds peaked at 68 rounds.
    let catalog = CombatCatalog::default();
    let mut setups: Vec<_> = (0..8)
        .map(|_| ship(ShipKind::Patrol, Side::Player))
        .collect();
    setups.extend((0..8).map(|_| ship(ShipKind::Patrol, Side::Hostile)));
    for seed in 0..60 {
        let result = auto_resolve(&setups, &catalog, seed).unwrap();
        assert!(result.winner.is_some(), "seed {seed} was undecided");
        assert!(result.rounds.len() < MAX_ROUNDS as usize / 2, "seed {seed}");
    }
}

#[test]
fn mirror_matches_are_fair_and_a_lone_trader_survives_a_scout_raider() {
    for kind in [ShipKind::Scout, ShipKind::Patrol] {
        let (wins, losses) = player_record(&[kind], &[kind]);
        let share = wins * 100 / (wins + losses).max(1);
        assert!(
            (40..=60).contains(&share),
            "{kind:?} mirror: {share}% of {}",
            wins + losses
        );
    }
    let (_, losses) = player_record(&[ShipKind::Freighter], &[ShipKind::Scout]);
    assert_eq!(
        losses, 0,
        "the starting freighter must not lose to one scout"
    );
}
