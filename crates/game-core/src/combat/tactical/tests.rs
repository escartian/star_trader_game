use super::{
    Battle, BattleError, CEASEFIRE_ROUNDS, COVER_PERCENT, Hex, MAX_ROUNDS, Terrain, TerrainHex,
    auto_resolve, move_points, play_turn,
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
        gear: Vec::new(),
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

/// A battle on open ground: terrain cleared so tests place their own.
fn battle(setups: &[CombatantSetup], seed: u64) -> Battle {
    let mut battle = Battle::new(setups, &CombatCatalog::default(), seed).unwrap();
    battle.terrain.clear();
    battle
}

fn rock(battle: &mut Battle, hex: Hex, kind: Terrain) {
    battle.terrain.push(TerrainHex { hex, kind });
}

/// Sets the initiative order and gives the new first ship its movement.
pub(super) fn reorder(battle: &mut Battle, order: Vec<u8>) {
    battle.order = order;
    battle.turn = 0;
    battle.moves_left = battle.units[usize::from(battle.order[0])].move_points();
}

#[test]
fn sides_deploy_on_opposite_columns_in_unique_hexes() {
    let setups: Vec<_> = (0..4)
        .map(|_| ship(ShipKind::Scout, Side::Player))
        .chain((0..3).map(|_| ship(ShipKind::Scout, Side::Hostile)))
        .collect();
    let battle = battle(&setups, 1);
    let hexes: std::collections::HashSet<_> = battle.units.iter().map(|unit| unit.at).collect();
    assert_eq!(hexes.len(), 7);
    for unit in &battle.units {
        assert!(battle.board.contains(unit.at));
        let column = battle.board.home_column(unit.side);
        assert_eq!((unit.at.q - column).abs(), 1);
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
fn moves_spend_engine_points_on_free_hexes_around_asteroids() {
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
    let step = |q: i8| Hex::new(from.q + q, from.r);
    assert_eq!(battle.move_to(step(3)), Err(BattleError::TooFar));
    let off_board = Hex::new(from.q - 2, from.r);
    assert_eq!(battle.move_to(off_board), Err(BattleError::Blocked));
    assert_eq!(battle.move_to(from), Err(BattleError::Blocked), "no-op");
    // Another ship's hex is blocked, but ships can be passed.
    battle.units[1].at = step(1);
    assert_eq!(battle.move_to(step(1)), Err(BattleError::Blocked));
    assert!(battle.reachable().contains(&step(2)));
    battle.units[1].at = Hex::new(3, -1);
    // An asteroid is blocked, and the way around it costs more steps.
    rock(&mut battle, step(1), Terrain::Asteroid);
    assert_eq!(battle.move_to(step(1)), Err(BattleError::Blocked));
    assert!(!battle.reachable().contains(&step(2)), "three steps around");
    battle.terrain.clear();
    // Points split across actions: one hex, then another.
    battle.move_to(step(1)).unwrap();
    assert_eq!(battle.moves_left, 1);
    assert!(battle.moved());
    assert_eq!(battle.reachable().len(), 6, "six neighbours");
    battle.move_to(step(2)).unwrap();
    assert_eq!(battle.units[0].at, step(2));
    assert!(battle.reachable().is_empty());
    assert_eq!(battle.move_to(step(1)), Err(BattleError::AlreadyMoved));
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
    battle.units[0].at = Hex::new(-4, 0);
    battle.units[1].at = Hex::new(4, 0);
    assert_eq!(
        battle.fire(0, &catalog),
        Err(BattleError::InvalidTarget),
        "own ship"
    );
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::OutOfRange));
    // Four hexes apart.
    battle.units[0].at = Hex::new(0, 0);
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
fn asteroids_block_the_line_of_fire_and_debris_gives_cover() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Freighter, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    reorder(&mut battle, vec![0, 1]);
    battle.units[1].profile.hull = 1_000;
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(2, 0);
    let beam = catalog.weapon("neutron_beam").unwrap();
    let open = battle.shot_chance(0, 1, beam);
    rock(&mut battle, Hex::new(2, 0), Terrain::Debris);
    assert_eq!(
        battle.shot_chance(0, 1, beam),
        open - COVER_PERCENT as u32,
        "cover"
    );
    rock(&mut battle, Hex::new(0, 0), Terrain::Asteroid);
    assert!(battle.targets(0, &catalog).is_empty());
    assert_eq!(battle.fire(1, &catalog), Err(BattleError::NoLineOfFire));
    // Side by side, nothing stands between.
    battle.units[0].at = Hex::new(1, 0);
    assert_eq!(battle.targets(0, &catalog), vec![1]);
    battle.fire(1, &catalog).unwrap();
}

#[test]
fn the_ai_finds_a_clear_line_of_fire_around_an_asteroid() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Scout, Side::Player),
            ship(ShipKind::Freighter, Side::Hostile),
        ],
        2,
    );
    reorder(&mut battle, vec![1, 0]);
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    rock(&mut battle, Hex::new(0, 0), Terrain::Asteroid);
    assert!(battle.targets(1, &catalog).is_empty());
    play_turn(&mut battle, &catalog).unwrap();
    assert_ne!(battle.units[1].at, Hex::new(1, 0), "it moved");
    assert!(battle.units[1].shots > 0, "and fired");
}

#[test]
fn piercing_beams_ignore_cover_and_steady_beams_ignore_range() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(2, 0);
    let lance = catalog.weapon("photon_singularity_beam").unwrap();
    let open = battle.shot_chance(0, 1, lance);
    rock(&mut battle, Hex::new(2, 0), Terrain::Debris);
    assert_eq!(battle.shot_chance(0, 1, lance), open, "piercing");
    battle.terrain.clear();
    let steady = catalog.weapon("neutron_beam").unwrap();
    let far = battle.shot_chance(0, 1, steady);
    battle.units[1].at = Hex::new(0, 0);
    assert_eq!(battle.shot_chance(0, 1, steady), far, "no range falloff");
}

#[test]
fn guided_torpedoes_fly_around_asteroids_and_point_defense_counts_double() {
    use crate::combat::gear::{Gear, POINT_DEFENSE_PERCENT};
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    reorder(&mut battle, vec![0, 1]);
    battle.units[1].profile.hull = 1_000;
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    rock(&mut battle, Hex::new(0, 0), Terrain::Asteroid);
    let (from, to) = (battle.units[0].at, battle.units[1].at);
    let volley = battle.volley_from(0, from, to, &catalog);
    assert_eq!(volley.len(), 1);
    assert_eq!(volley[0].id, "quantum_entanglement_torpedo");
    battle.fire(1, &catalog).unwrap();
    assert!(
        battle.rounds[0]
            .iter()
            .all(|shot| shot.weapon == "quantum_entanglement_torpedo")
    );
    let torpedo = catalog.weapon("quantum_entanglement_torpedo").unwrap();
    let open = battle.shot_chance(0, 1, torpedo);
    battle.units[1].gear = vec![Gear::PointDefense];
    let expected = open.saturating_sub(2 * POINT_DEFENSE_PERCENT as u32).max(5);
    assert_eq!(battle.shot_chance(0, 1, torpedo), expected);
}

#[test]
fn ripples_splash_packed_enemies_and_disruptors_jam_engines() {
    let catalog = CombatCatalog::default();
    let mut setups = vec![ship(ShipKind::Scout, Side::Player)];
    setups.extend((0..3).map(|_| ship(ShipKind::Freighter, Side::Hostile)));
    let mut battle = battle(&setups, 5);
    reorder(&mut battle, vec![0, 1, 2, 3]);
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    battle.units[2].at = Hex::new(1, 1);
    battle.units[3].at = Hex::new(3, -2);
    // Every pulse hits: the neighbour takes ripples, the far ship nothing.
    battle.units[0].profile.loadout = vec!["graviton_pulse".into()];
    let mut sure = catalog.clone();
    sure.weapons
        .iter_mut()
        .for_each(|weapon| weapon.accuracy_percent = 100);
    battle.fire(1, &sure).unwrap();
    let shots = &battle.rounds[0];
    assert!(shots.iter().any(|shot| shot.target == 2 && shot.hit));
    assert!(shots.iter().all(|shot| shot.target != 3));
    assert!(battle.units[2].shields < battle.units[2].profile.shields);
    // A disruptor hit costs the target a move on its next turn, then wears off.
    let mut battle = self::battle(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        5,
    );
    reorder(&mut battle, vec![0, 1]);
    battle.units[1].profile.hull = 1_000;
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    battle.units[0].profile.loadout = vec!["magnetic_resonance_disruptor".into()];
    battle.fire(1, &sure).unwrap();
    assert!(battle.units[1].jammed);
    battle.end_turn().unwrap();
    assert_eq!(battle.moves_left, battle.units[1].move_points() - 1);
    battle.end_turn().unwrap();
    assert!(!battle.units[1].jammed, "one turn only");
}

#[test]
fn the_ai_prefers_cover_when_it_can_fire_from_there() {
    let catalog = CombatCatalog::default();
    let mut battle = battle(
        &[
            ship(ShipKind::Scout, Side::Player),
            ship(ShipKind::Patrol, Side::Hostile),
        ],
        2,
    );
    reorder(&mut battle, vec![1, 0]);
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    rock(&mut battle, Hex::new(1, -1), Terrain::Debris);
    play_turn(&mut battle, &catalog).unwrap();
    assert_eq!(battle.units[1].at, Hex::new(1, -1));
    assert!(battle.units[1].shots > 0);
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
    let edge = Hex::new(battle.board.home_column(Side::Player), here.r);
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

#[test]
fn afterburners_add_a_move_and_point_defense_turns_shots_aside() {
    use crate::combat::gear::{Gear, POINT_DEFENSE_PERCENT};
    let catalog = CombatCatalog::default();
    let mut fast = ship(ShipKind::Freighter, Side::Player);
    fast.gear = vec![Gear::Afterburners];
    let mut armored = ship(ShipKind::Scout, Side::Hostile);
    armored.gear = vec![Gear::PointDefense];
    let mut battle = battle(&[fast, armored], 3);
    reorder(&mut battle, vec![0, 1]);
    assert_eq!(battle.moves_left, 3, "freighter 2 + afterburners 1");
    battle.units[0].at = Hex::new(-1, 0);
    battle.units[1].at = Hex::new(1, 0);
    let beam = catalog.weapon("neutron_beam").unwrap();
    let guarded = battle.shot_chance(0, 1, beam);
    battle.units[1].gear.clear();
    assert_eq!(
        battle.shot_chance(0, 1, beam),
        guarded + POINT_DEFENSE_PERCENT as u32
    );
}
