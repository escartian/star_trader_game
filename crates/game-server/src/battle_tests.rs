use super::{
    Session,
    combat_tests::{choose, raid, reload},
    navigation_tests::succeed,
};
use game_core::{
    combat::{
        CombatOutcome, Side,
        tactical::{Hex, Position},
    },
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
        .min_by_key(|to| (to.range(start.units[1].at), to.hex.r, to.band))
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
    assert_eq!(after.units[me].at, step);
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
    let here = battle.units[me].at;
    let far = Position::new(Hex::new(here.hex.q + 5, here.hex.r - 2), here.band);
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
    let here = battle.units[me].at;
    let edge = Position::new(
        Hex::new(battle.board.home_column(Side::Player), here.hex.r),
        here.band,
    );
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
            battle.units[1].at.hex = battle.units[0].at.hex;
        }),
        "two ships on one hex"
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
        forge(&|s| open(s).moves_left = 99),
        "more movement than the engine gives"
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

#[test]
fn surrender_hands_over_the_cargo_and_lets_the_fleet_go() {
    let mut session = open_battle();
    session.state.ships[0].cargo[game_core::economy::Commodity::Food.index()] = 6;
    let damage = session
        .state
        .navigation
        .battle
        .as_ref()
        .unwrap()
        .battle
        .units[0]
        .damage;
    play(&mut session, BattleAction::Surrender);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.outcome, CombatOutcome::Surrendered);
    assert_eq!(report.cargo_lost, 6);
    assert_eq!(session.state.ships[0].cargo_used(), 0);
    assert_eq!(
        session.state.ships[0].hull_damage, damage,
        "ships keep their damage"
    );
    assert!(session.state.navigation.battle.is_none());
    assert!(session.state.navigation.encounters.is_empty());
    assert!(
        session.state.navigation.order(FleetId(1)).is_some(),
        "the course resumes"
    );
    reload(&session);
}

#[test]
fn an_open_battle_from_a_3d_grid_save_is_auto_resolved_on_load() {
    let session = open_battle();
    let seed = open(&mut session.clone()).seed;
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    // Turn the saved battle back into its schema-7 shape.
    value["schema_version"] = 7.into();
    let battle = &mut value["player"]["navigation"]["battle"]["battle"];
    let fields = battle.as_object_mut().unwrap();
    fields.remove("board");
    fields.remove("moves_left");
    fields.insert(
        "size".into(),
        serde_json::json!({"width": 10, "depth": 10, "height": 10}),
    );
    fields.insert("moved".into(), false.into());
    for unit in fields["units"].as_array_mut().unwrap() {
        let unit = unit.as_object_mut().unwrap();
        unit.remove("at");
        unit.insert("cell".into(), serde_json::json!({"x": 1, "y": 4, "z": 4}));
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("grid.json");
    std::fs::write(&path, value.to_string()).unwrap();
    let (loaded, notice) = Session::load(&path).unwrap();
    assert!(notice.unwrap().contains("auto-resolved"));
    assert!(loaded.state.navigation.battle.is_none());
    assert!(loaded.state.navigation.encounters.is_empty());
    let report = loaded.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.seed, seed);
    assert_eq!(
        loaded.state.ships[0].hull_damage,
        report.combatants[0].final_damage
    );
    // It saves in the current schema from then on.
    reload(&loaded);
}
