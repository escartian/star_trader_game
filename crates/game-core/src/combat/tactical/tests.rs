use super::{
    Band, Battle, BattleError, CEASEFIRE_ROUNDS, Hex, MAX_ROUNDS, Position, auto_resolve,
    move_points, play_turn,
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

/// Sets the initiative order and gives the new first ship its movement.
pub(super) fn reorder(battle: &mut Battle, order: Vec<u8>) {
    battle.order = order;
    battle.turn = 0;
    battle.moves_left = move_points(battle.units[usize::from(battle.order[0])].profile.engine);
}

#[test]
fn sides_deploy_on_opposite_columns_in_unique_hexes() {
    let setups: Vec<_> = (0..4)
        .map(|_| ship(ShipKind::Scout, Side::Player))
        .chain((0..3).map(|_| ship(ShipKind::Scout, Side::Hostile)))
        .collect();
    let battle = battle(&setups, 1);
    let hexes: std::collections::HashSet<_> = battle.units.iter().map(|unit| unit.at.hex).collect();
    assert_eq!(hexes.len(), 7);
    for unit in &battle.units {
        assert!(battle.board.contains(unit.at.hex));
        let column = battle.board.home_column(unit.side);
        assert_eq!((unit.at.hex.q - column).abs(), 1);
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
fn moves_spend_engine_points_on_free_hexes_and_band_changes() {
    let mut battle = battle(
        &[
            ship(ShipKind::Freighter, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        3,
    );
    reorder(&mut battle, vec![0, 1]);
    let from = battle.units[0].at;
    let points = move_points(battle.units[0].profile.engine);
    assert_eq!(points, 2, "freighter engine 3");
    let step = |q: i8| Position::new(Hex::new(from.hex.q + q, from.hex.r), from.band);
    assert_eq!(battle.move_to(step(3)), Err(BattleError::TooFar));
    let off_board = Position::new(Hex::new(from.hex.q - 2, from.hex.r), from.band);
    assert_eq!(battle.move_to(off_board), Err(BattleError::Blocked));
    assert_eq!(battle.move_to(from), Err(BattleError::Blocked), "no-op");
    // Two hexes and a climb is three points: one too many.
    let far_high = Position::new(step(2).hex, Band::High);
    assert!(!battle.reachable().contains(&far_high));
    assert_eq!(battle.move_to(far_high), Err(BattleError::TooFar));
    // Another ship's hex is blocked on every band.
    battle.units[1].at = Position::new(step(1).hex, Band::Low);
    assert_eq!(battle.move_to(step(1)), Err(BattleError::Blocked));
    battle.units[1].at = Position::new(Hex::new(3, -1), Band::Level);
    // Points split across actions: one hex, then a climb.
    battle.move_to(step(1)).unwrap();
    assert_eq!(battle.moves_left, 1);
    assert!(battle.moved());
    let climbed = Position::new(step(1).hex, Band::High);
    assert_eq!(
        battle.reachable().len(),
        8,
        "six neighbours, a climb or a dive"
    );
    battle.move_to(climbed).unwrap();
    assert_eq!(battle.units[0].at, climbed);
    assert!(battle.reachable().is_empty());
    assert_eq!(battle.move_to(step(0)), Err(BattleError::AlreadyMoved));
    // The next ship starts with its own full allowance.
    battle.end_turn().unwrap();
    assert_eq!(
        battle.moves_left,
        move_points(battle.units[1].profile.engine)
    );
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
    reorder(&mut battle, vec![0, 1]);
    // Tough enough to survive one volley.
    battle.units[1].profile.hull = 1_000;
    // Eight hexes apart, edge to edge: beyond every weapon.
    battle.units[0].at = Position::new(Hex::new(-4, 0), Band::Level);
    battle.units[1].at = Position::new(Hex::new(4, 0), Band::Level);
    assert_eq!(
        battle.fire(0, &catalog),
        Err(BattleError::InvalidTarget),
        "own ship"
    );
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::OutOfRange));
    // Three hexes and one band apart: range 4.
    battle.units[0].at = Position::new(Hex::new(1, 0), Band::High);
    assert_eq!(battle.targets(0, &catalog), vec![1]);
    battle.fire(1, &catalog).unwrap();
    let shots = &battle.rounds[0];
    assert!(
        !shots.is_empty()
            && shots
                .iter()
                .all(|shot| (shot.attacker, shot.target) == (0, 1))
    );
    // Range 4: the photon beam and torpedo reach; the 3-hex disruptor does not.
    assert!(
        shots
            .iter()
            .all(|shot| shot.weapon != "magnetic_resonance_disruptor")
    );
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::AlreadyFired));
}

#[test]
fn firing_down_a_band_hits_more_often_than_firing_up() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    let beam = catalog.weapon("photon_singularity_beam").unwrap();
    let place = |battle: &mut Battle, mine: Band, theirs: Band| {
        battle.units[0].at = Position::new(Hex::new(0, 0), mine);
        battle.units[1].at = Position::new(Hex::new(2, 0), theirs);
        battle.shot_chance(0, 1, beam)
    };
    let level = place(&mut battle, Band::Level, Band::Level);
    // One band apart is one step farther (−4%) either way, then ±10%.
    assert_eq!(place(&mut battle, Band::High, Band::Level), level - 4 + 10);
    assert_eq!(place(&mut battle, Band::Low, Band::Level), level - 4 - 10);
    assert_eq!(
        place(&mut battle, Band::High, Band::Low),
        level - 8 + 10,
        "a flat bonus, not per band"
    );
}

#[test]
fn the_ai_takes_the_higher_band_when_it_costs_nothing() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Scout, Side::Player),
            ship(ShipKind::Patrol, Side::Hostile),
        ],
        2,
    );
    reorder(&mut battle, vec![1, 0]);
    // The raider is already in reach with every weapon: it climbs, then fires.
    battle.units[0].at = Position::new(Hex::new(-1, 0), Band::Level);
    battle.units[1].at = Position::new(Hex::new(1, 0), Band::Level);
    play_turn(&mut battle, &catalog).unwrap();
    assert_eq!(battle.units[1].at.band, Band::High);
    assert!(battle.units[1].shots > 0);
    // Above its target already, it holds the band rather than climb out of reach.
    let mut battle = battle_with_raider_above(&catalog);
    play_turn(&mut battle, &catalog).unwrap();
    assert_eq!(battle.units[1].at.band, Band::Level);
}

fn battle_with_raider_above(catalog: &CombatCatalog) -> Battle {
    let mut battle = Battle::new(
        &[
            ship(ShipKind::Scout, Side::Player),
            ship(ShipKind::Patrol, Side::Hostile),
        ],
        catalog,
        2,
    )
    .unwrap();
    reorder(&mut battle, vec![1, 0]);
    battle.units[0].at = Position::new(Hex::new(-1, 0), Band::Low);
    battle.units[1].at = Position::new(Hex::new(1, 0), Band::Level);
    battle
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
    reorder(&mut battle, vec![0, 1]);
    assert_eq!(battle.withdraw(), Err(BattleError::NotAtEdge));
    assert!(!battle.can_withdraw());
    let here = battle.units[0].at;
    let edge = Position::new(
        Hex::new(battle.board.home_column(Side::Player), here.hex.r),
        here.band,
    );
    battle.move_to(edge).unwrap();
    assert!(battle.can_withdraw());
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

/// Over 400 seeds: (fights where the raiders were driven off or destroyed,
/// fights the player lost outright). Every other fight the player escaped.
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
fn the_largest_battles_end_with_a_side_off_the_field_far_below_the_safety_cap() {
    // 8 patrols a side is the most a battle holds.
    let catalog = CombatCatalog::default();
    let mut setups: Vec<_> = (0..8)
        .map(|_| ship(ShipKind::Patrol, Side::Player))
        .collect();
    setups.extend((0..8).map(|_| ship(ShipKind::Patrol, Side::Hostile)));
    for seed in 0..60 {
        let mut battle = Battle::new(&setups, &catalog, seed).unwrap();
        super::finish(&mut battle, &catalog).unwrap();
        let fighting = |side| {
            battle
                .units
                .iter()
                .any(|unit| unit.side == side && unit.active())
        };
        assert!(
            !fighting(Side::Player) || !fighting(Side::Hostile),
            "seed {seed}: ended without a side leaving the field"
        );
        assert!(battle.rounds.len() < MAX_ROUNDS as usize / 4, "seed {seed}");
    }
}

#[test]
fn mirror_matches_are_fair_and_a_lone_trader_survives_a_scout_raider() {
    // Fair = each side is driven from the field about half the time. Raiders
    // that flee count as a player win; a player who flees has escaped.
    for kind in [ShipKind::Scout, ShipKind::Patrol] {
        let (wins, _) = player_record(&[kind], &[kind]);
        let share = wins * 100 / 400;
        assert!((40..=60).contains(&share), "{kind:?} mirror: {share}%");
    }
    let (_, losses) = player_record(&[ShipKind::Freighter], &[ShipKind::Scout]);
    assert_eq!(
        losses, 0,
        "the starting freighter must not lose to one scout"
    );
}
