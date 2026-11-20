use super::{
    Session,
    combat_tests::{choose, raid, reload},
    navigation_tests::succeed,
};
use game_core::{
    combat::{CombatOutcome, Side, tactical::Cell},
    fleet::{FleetId, ShipKind},
    navigation::{BattleAction, EncounterChoice},
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

/// Applies an action that must succeed.
fn play(session: &mut Session, action: BattleAction) {
    let revision = session.state.revision;
    succeed(
        session,
        ClientCommand::BattleAction {
            action,
            expected_revision: revision,
        },
    );
}

fn act(session: &mut Session, action: BattleAction) -> ServerMessage {
    session.handle(ClientCommand::BattleAction {
        action,
        expected_revision: session.state.revision,
    })
}

/// A freighter meets one scout raider and chooses Fight.
fn open_battle() -> Session {
    let mut session = Session::default();
    let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Scout]);
    choose(&mut session, encounter, EncounterChoice::Fight);
    session
}

fn open(session: &mut Session) -> &mut game_core::combat::tactical::Battle {
    &mut session.state.navigation.battle.as_mut().unwrap().battle
}

fn player_unit_is_up(session: &Session) -> bool {
    let battle = &session.state.navigation.battle.as_ref().unwrap().battle;
    battle
        .active()
        .is_some_and(|unit| battle.units[unit].side == Side::Player)
}

#[test]
fn fight_opens_a_battle_that_waits_on_the_player_and_survives_reload() {
    let mut session = open_battle();
    assert!(
        player_unit_is_up(&session),
        "raider turns are already played"
    );
    assert_eq!(session.state.navigation.encounters.len(), 1);
    assert!(
        matches!(
            session.handle(ClientCommand::ExecuteNavigationRun {
                expected_revision: session.state.revision,
            }),
            ServerMessage::CommandRejected(_)
        ),
        "navigation waits for the battle"
    );
    let mut twin = reload(&session);
    let battle = |session: &Session| session.state.navigation.battle.clone().unwrap().battle;
    let start = battle(&session);
    let me = start.active().unwrap();
    let step = *start
        .reachable()
        .iter()
        .min_by_key(|cell| (cell.distance(start.units[1].cell), cell.y, cell.z))
        .unwrap();
    for session in [&mut session, &mut twin] {
        let revision = session.state.revision;
        succeed(
            session,
            ClientCommand::BattleAction {
                action: BattleAction::Move(step),
                expected_revision: revision,
            },
        );
        let revision = session.state.revision;
        succeed(
            session,
            ClientCommand::BattleAction {
                action: BattleAction::EndTurn,
                expected_revision: revision,
            },
        );
    }
    assert_eq!(session.state, twin.state, "same saved battle, same play");
    let after = battle(&session);
    assert_eq!(after.units[me].cell, step);
    assert!(
        after.round > start.round,
        "the raider acted, a new round began"
    );
    assert!(player_unit_is_up(&session));
    reload(&session);
    play(&mut session, BattleAction::AutoResolve);
    assert!(session.state.navigation.battle.is_none());
    assert!(session.state.navigation.encounters.is_empty());
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.seed, start.seed);
    assert_eq!(
        session.state.ships[0].hull_damage,
        report.combatants[0].final_damage
    );
    reload(&session);
}

#[test]
fn illegal_battle_actions_are_rejected_without_changing_anything() {
    let mut idle = Session::default();
    assert!(matches!(
        act(&mut idle, BattleAction::EndTurn),
        ServerMessage::CommandRejected(_)
    ));
    let mut session = open_battle();
    let battle = session.state.navigation.battle.clone().unwrap().battle;
    let me = battle.active().unwrap();
    let here = battle.units[me].cell;
    let far = Cell {
        x: here.x + 5,
        ..here
    };
    let encounter = session.state.navigation.encounters[0].id;
    let before = session.snapshot(Vec::new());
    for action in [
        BattleAction::Fire { target: me as u8 },
        BattleAction::Fire { target: 99 },
        BattleAction::Move(far),
        BattleAction::Withdraw,
    ] {
        assert!(
            matches!(act(&mut session, action), ServerMessage::CommandRejected(_)),
            "{action:?}"
        );
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    assert!(matches!(
        choose(&mut session, encounter, EncounterChoice::AutoResolve),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
}

#[test]
fn withdrawing_every_ship_disengages_and_the_course_continues() {
    let mut session = open_battle();
    let battle = session.state.navigation.battle.clone().unwrap().battle;
    let me = battle.active().unwrap();
    let edge = Cell {
        x: battle.size.home_edge(Side::Player),
        ..battle.units[me].cell
    };
    play(&mut session, BattleAction::Move(edge));
    play(&mut session, BattleAction::Withdraw);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.outcome, CombatOutcome::Disengaged);
    assert!(session.state.navigation.battle.is_none());
    assert!(session.state.navigation.encounters.is_empty());
    assert!(session.state.navigation.order(FleetId(1)).is_some());
}

#[test]
fn forged_battles_fail_save_validation() {
    let session = open_battle();
    assert!(session.export_json().is_ok());
    let forge = |change: &dyn Fn(&mut Session)| {
        let mut forged = session.clone();
        change(&mut forged);
        forged.export_json().is_err()
    };
    assert!(
        forge(&|s| open(s).units[0].profile.hull = 9_999),
        "boosted hull"
    );
    assert!(
        forge(&|s| open(s).units[0].ship = Some(ShipId(77))),
        "foreign ship"
    );
    assert!(
        forge(&|s| {
            let battle = open(s);
            battle.units[1].damage = battle.units[1].profile.hull;
        }),
        "dead raider left open"
    );
    assert!(
        forge(&|s| {
            let battle = open(s);
            battle.units[1].cell = battle.units[0].cell;
        }),
        "overlapping ships"
    );
    assert!(
        forge(&|s| {
            let battle = open(s);
            let raider = battle.order.iter().position(|&unit| unit == 1).unwrap();
            battle.turn = raider;
        }),
        "a raider waiting for orders"
    );
    assert!(
        forge(&|s| s.state.navigation.encounters.clear()),
        "battle without a contact"
    );
    assert!(
        forge(&|s| s.state.navigation.battle.as_mut().unwrap().fleet = FleetId(9)),
        "wrong fleet"
    );
}
