//! Pursuing a police patrol (`game_core::pursuit::Quarry::Patrol`): the
//! contact you start can only be fought or let go.
use super::{
    Session,
    band_tests::run,
    combat_tests::{choose, reload},
    police_tests::policed,
};
use game_core::{
    LocalPosition,
    factions::Faction,
    fleet::FleetId,
    navigation::{EncounterChoice, EncounterKind},
    pursuit::Quarry,
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage};

fn pursue(session: &mut Session, patrol: u64) -> ServerMessage {
    session.handle(ClientCommand::Pursue {
        fleet: FleetId(1),
        quarry: Quarry::Patrol(patrol),
        expected_revision: session.state.revision,
    })
}

/// `police_tests::policed` after one quiet run (contacts need run 1 or later).
/// The fleet is put back 100,000 km from the patrol, which moved that run;
/// random contacts are off so only the pursuit makes one.
fn patrolled() -> Session {
    let mut session = policed(Session::default(), FleetId(1), 0);
    session.state.movement_rules.encounter_chance_percent = 0;
    super::test_support::execute_quiet_run(&mut session);
    session.state.navigation.holds.clear();
    let patrol = session.state.navigation.police.patrols[0].clone();
    session.state.ships[0].location = Location::InSystem {
        system: patrol.system,
        position: LocalPosition {
            z_km: patrol.position.z_km + 100_000,
            ..patrol.position
        },
    };
    session
}

fn rejected(message: ServerMessage) -> bool {
    matches!(message, ServerMessage::CommandRejected(_))
}

#[test]
fn a_patrol_in_reach_is_intercepted_and_can_be_let_go_without_a_fine() {
    let mut session = patrolled();
    let patrol = session.state.navigation.police.patrols[0].id;
    let credits = session.economy.credits;
    assert!(!rejected(pursue(&mut session, patrol)));
    let contact = session.state.navigation.encounters[0].clone();
    assert_eq!(contact.kind, EncounterKind::Police);
    let police = contact.police.unwrap();
    assert!(police.intercepted, "you stopped them");
    assert_eq!(session.state.navigation.pursuit, None, "caught");
    let reloaded = reload(&session);
    assert_eq!(reloaded.state.navigation.encounters[0], contact);
    // No fine to pay and nothing to evade: only fight or let go.
    assert!(rejected(choose(
        &mut session,
        contact.id,
        EncounterChoice::Comply
    )));
    assert!(rejected(choose(
        &mut session,
        contact.id,
        EncounterChoice::Evade
    )));
    assert!(!rejected(choose(
        &mut session,
        contact.id,
        EncounterChoice::Bypass
    )));
    assert!(session.state.navigation.encounters.is_empty());
    assert_eq!(session.economy.credits, credits);
    assert!(!session.economy.factions.is_wanted(Faction::Federation));
}

#[test]
fn attacking_an_intercepted_patrol_is_a_crime() {
    let mut session = patrolled();
    let patrol = session.state.navigation.police.patrols[0].id;
    assert!(!rejected(pursue(&mut session, patrol)));
    let contact = session.state.navigation.encounters[0].id;
    assert!(!rejected(choose(
        &mut session,
        contact,
        EncounterChoice::Fight
    )));
    assert!(session.state.navigation.battle.is_some());
    assert!(session.economy.factions.is_wanted(Faction::Federation));
}

#[test]
fn a_distant_patrol_is_chased_down_in_its_system() {
    let mut session = patrolled();
    let patrol = session.state.navigation.police.patrols[0].clone();
    // Start well out of reach, in the same system.
    session.state.ships[0].location = Location::InSystem {
        system: patrol.system,
        position: LocalPosition {
            x_km: patrol.position.x_km - 120_000_000,
            ..patrol.position
        },
    };
    session.state.navigation.holds.clear();
    assert!(!rejected(pursue(&mut session, patrol.id)));
    assert!(session.state.navigation.encounters.is_empty());
    assert!(session.state.navigation.pursuit.is_some());
    for _ in 0..10 {
        if !session.state.navigation.encounters.is_empty() {
            let contact = &session.state.navigation.encounters[0];
            assert!(contact.police.is_some_and(|police| police.intercepted));
            return;
        }
        run(&mut session);
    }
    panic!("the patrol was never caught");
}

#[test]
fn an_unknown_patrol_cannot_be_pursued() {
    let mut session = patrolled();
    assert!(rejected(pursue(&mut session, 9_999)));
}
