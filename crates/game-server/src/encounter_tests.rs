use super::{
    Session,
    navigation_tests::{plot, run, succeed, two_fleets},
};
use game_core::{
    BodyId, SystemId,
    fleet::FleetId,
    movement::MoveTarget,
    navigation::{EncounterChoice, EncounterKind},
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

fn body(id: u64) -> MoveTarget {
    MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(id),
    }
}
fn round_trip(session: &Session) -> Session {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("navigation.json");
    session.save(&path).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_none());
    assert_eq!(restored.state, session.state);
    assert_eq!(restored.economy, session.economy);
    restored
}

#[test]
fn encounter_rolls_and_choices_survive_reload_without_duplicate_rewards() {
    let mut session = two_fleets();
    session.state.movement_rules.encounter_chance_percent = 100;
    plot(&mut session, ShipId(1), body(1));
    plot(&mut session, ShipId(2), body(4));
    let mut restored = round_trip(&session);
    run(&mut session);
    run(&mut restored);
    assert_eq!(restored.state, session.state);
    // Fleet 1 is now secondary: its contact was settled automatically, and
    // only the flagship's waits for the player.
    assert_eq!(session.state.navigation.encounters.len(), 1);
    assert_eq!(session.state.navigation.encounters[0].fleet, FleetId(2));
    assert!(
        session
            .state
            .navigation
            .orders
            .iter()
            .all(|order| order.started && order.remaining_ticks == order.plan.duration_ticks)
    );
    restored = round_trip(&session);
    let before = restored.snapshot(Vec::new());
    assert!(matches!(
        restored.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: restored.state.revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(restored.snapshot(Vec::new()), before);
    for encounter in restored.state.navigation.encounters.clone() {
        let id = encounter.id;
        let choice = match encounter.kind {
            EncounterKind::DebrisField => EncounterChoice::Investigate,
            EncounterKind::DistressSignal => EncounterChoice::Bypass,
            EncounterKind::Raiders => EncounterChoice::AutoResolve,
        };
        let revision = restored.state.revision;
        succeed(
            &mut restored,
            ClientCommand::ResolveEncounter {
                encounter: id,
                choice,
                expected_revision: revision,
            },
        );
        let paid = restored.snapshot(Vec::new());
        assert!(matches!(
            restored.handle(ClientCommand::ResolveEncounter {
                encounter: id,
                choice,
                expected_revision: restored.state.revision
            }),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(restored.snapshot(Vec::new()), paid);
    }
    assert_eq!(restored.state.tick, 4);
    let mut restored = round_trip(&restored);
    restored.state.movement_rules.encounter_chance_percent = 0;
    run(&mut restored);
    assert!(restored.state.navigation.orders.is_empty());
}

#[test]
fn a_contact_pauses_only_its_fleet_while_others_progress() {
    let mut session = two_fleets();
    // SplitMix seed zero gives 35 then 0: a ten-percent chance hits Fleet 2 only.
    session.state.movement_rules.encounter_chance_percent = 10;
    plot(&mut session, ShipId(1), body(1));
    plot(&mut session, ShipId(2), body(4));
    run(&mut session);
    assert!(session.state.navigation.order(FleetId(1)).is_none());
    assert!(session.state.navigation.order(FleetId(2)).is_some());
    assert_eq!(session.state.navigation.encounters.len(), 1);
    assert_eq!(session.state.navigation.encounters[0].fleet, FleetId(2));
}

#[test]
fn distress_assistance_requires_cargo_and_records_one_rescue() {
    let mut session = two_fleets();
    session.state.movement_rules.encounter_chance_percent = 100;
    // Find a deterministic seed whose first encounter is a distress signal.
    for seed in 0..100 {
        session.state.navigation.rng_state = seed;
        let roll = session.state.navigation.clone().rolls();
        if (roll >> 32) & 1 == 1 && !(roll >> 40).is_multiple_of(3) {
            break;
        }
    }
    plot(&mut session, ShipId(1), body(1));
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(2),
            expected_revision: revision,
        },
    );
    run(&mut session);
    let encounter = session.state.navigation.encounters[0].clone();
    assert_eq!(encounter.kind, EncounterKind::DistressSignal);
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::ResolveEncounter {
            encounter: encounter.id,
            choice: EncounterChoice::Assist,
            expected_revision: session.state.revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    session.state.ships[0].cargo[2] = 1;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::ResolveEncounter {
            encounter: encounter.id,
            choice: EncounterChoice::Assist,
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.ships[0].cargo[2], 0);
    assert_eq!(session.state.navigation.rescues, 1);
    round_trip(&session);
}
