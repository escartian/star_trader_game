use super::{
    Band, Battle, Board, Hex, Position,
    morale::{BREAK_MORALE, morale, retreating},
    play_turn,
    tests::reorder,
};
use crate::combat::{
    CombatCatalog,
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

fn duel(player: ShipKind, hostile: ShipKind) -> Battle {
    Battle::new(
        &[ship(player, Side::Player), ship(hostile, Side::Hostile)],
        &CombatCatalog::default(),
        7,
    )
    .unwrap()
}

#[test]
fn battles_open_on_a_board_sized_for_the_fleets_with_ships_on_the_level_band() {
    let battle = duel(ShipKind::Freighter, ShipKind::Scout);
    assert_eq!(battle.board, Board::for_ships(2));
    assert_eq!(battle.units[1].at.hex.q, 3, "raiders deploy one column in");
    assert!(battle.units.iter().all(|unit| unit.at.band == Band::Level));
    // The board is part of the saved battle.
    let mut json = serde_json::to_value(&battle).unwrap();
    json.as_object_mut().unwrap().remove("board");
    assert!(serde_json::from_value::<Battle>(json).is_err());
}

#[test]
fn morale_falls_with_damage_and_losses_and_rises_with_kills() {
    let mut battle = Battle::new(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        &CombatCatalog::default(),
        1,
    )
    .unwrap();
    assert_eq!(morale(&battle, Side::Player), 100);
    battle.units[2].damage = battle.units[2].profile.hull;
    // Raiders: half their hull and half their ships gone: 100 − 25 − 15 = 60.
    assert_eq!(morale(&battle, Side::Hostile), 60);
    assert_eq!(
        morale(&battle, Side::Player),
        100,
        "kills cannot exceed 100"
    );
    battle.units[3].withdrawn = true;
    assert_eq!(morale(&battle, Side::Hostile), 50);
    battle.units[0].damage = battle.units[0].profile.hull / 2;
    assert!(morale(&battle, Side::Player) < 100);
}

#[test]
fn a_badly_damaged_ship_retreats_to_its_edge_and_withdraws() {
    let catalog = CombatCatalog::default();
    let mut battle = duel(ShipKind::Patrol, ShipKind::Scout);
    let raider = 1;
    reorder(&mut battle, vec![1, 0]);
    assert!(!retreating(&battle, raider));
    battle.units[raider].damage = battle.units[raider].profile.hull * 3 / 4;
    assert!(retreating(&battle, raider), "25% hull left");
    play_turn(&mut battle, &catalog).unwrap();
    assert!(
        battle.units[raider].withdrawn,
        "one hex from its edge: out at once"
    );
    assert_eq!(
        battle.outcome(),
        Some(Some(Side::Player)),
        "a fled raider is a win"
    );
}

#[test]
fn a_broken_side_routs_even_healthy_ships() {
    let catalog = CombatCatalog::default();
    let mut battle = Battle::new(
        &[
            ship(ShipKind::Patrol, Side::Player),
            ship(ShipKind::Scout, Side::Hostile),
            ship(ShipKind::Scout, Side::Hostile),
            ship(ShipKind::Scout, Side::Hostile),
        ],
        &catalog,
        3,
    )
    .unwrap();
    battle.units[1].damage = battle.units[1].profile.hull;
    battle.units[2].damage = battle.units[2].profile.hull;
    assert!(morale(&battle, Side::Hostile) < BREAK_MORALE);
    assert!(retreating(&battle, 3), "an untouched raider still flees");
    reorder(&mut battle, vec![3, 0, 1, 2]);
    let before = battle.units[3].at;
    play_turn(&mut battle, &catalog).unwrap();
    let edge = battle.board.home_column(Side::Hostile);
    assert!(battle.units[3].withdrawn || battle.units[3].at.hex.q > before.hex.q);
    assert!(battle.units[3].at.hex.q <= edge);
}

#[test]
fn the_player_escaping_is_not_a_defeat_and_movement_holds_off_the_ceasefire() {
    let mut battle = duel(ShipKind::Scout, ShipKind::Scout);
    reorder(&mut battle, vec![0, 1]);
    let here = battle.units[0].at;
    let edge = Position::new(Hex::new(here.hex.q - 1, here.hex.r), here.band);
    battle.move_to(edge).unwrap();
    battle.withdraw().unwrap();
    assert_eq!(battle.outcome(), Some(None));
    // Ships that keep manoeuvring out of reach are still acting: no ceasefire.
    let mut battle = duel(ShipKind::Scout, ShipKind::Scout);
    for round in 0..4 {
        for _ in 0..2 {
            let me = battle.active().unwrap();
            let here = battle.units[me].at;
            let band = if round % 2 == 0 {
                Band::High
            } else {
                Band::Level
            };
            battle.move_to(Position::new(here.hex, band)).unwrap();
            battle.end_turn().unwrap();
        }
    }
    assert!(!battle.finished(), "four rounds of manoeuvre");
    // Three whole rounds in which nobody acts end the battle, disengaged.
    for _ in 0..6 {
        battle.end_turn().unwrap();
    }
    assert!(battle.finished());
    assert_eq!(battle.outcome(), Some(None));
}

#[test]
fn only_the_player_side_surrenders_and_only_on_its_turn() {
    let mut battle = duel(ShipKind::Scout, ShipKind::Scout);
    reorder(&mut battle, vec![1, 0]);
    assert_eq!(battle.surrender(), Err(super::BattleError::NotYourTurn));
    battle.end_turn().unwrap();
    battle.surrender().unwrap();
    assert!(battle.finished());
    assert_eq!(battle.outcome(), Some(None), "nobody won");
    assert_eq!(battle.surrender(), Err(super::BattleError::Finished));
}
