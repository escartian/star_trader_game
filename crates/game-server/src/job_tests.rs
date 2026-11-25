//! Job boards and contracts through real commands (see `game_core::jobs`).
use super::*;
use crate::test_support::{execute_quiet_run, move_fleet};
use game_core::{
    economy::Credits,
    jobs::{Job, JobKind, Task},
    movement::MoveTarget,
    travel::ShipId,
};

fn events(message: ServerMessage) -> Vec<SessionEvent> {
    match message {
        ServerMessage::SessionSnapshot { events, .. } => events,
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

#[test]
fn a_new_game_posts_the_home_board_and_a_job_is_taken_and_abandoned_by_command() {
    let mut session = Session::with_seed(3);
    let home = session
        .economy
        .current_market(&session.state.ships[0])
        .unwrap()
        .id;
    let offers = session.state.jobs.offers.clone();
    assert!(!offers.is_empty());
    assert!(offers.iter().all(|job| job.issuer == home));
    let job = offers[0].id;
    let accepted = events(session.handle(ClientCommand::AcceptJob {
        job,
        ship: ShipId(1),
        expected_revision: session.state.revision,
    }));
    assert!(matches!(&accepted[0], SessionEvent::JobAccepted { job: id, .. } if *id == job));
    assert_eq!(session.state.jobs.active.len(), 1);
    let again = session.handle(ClientCommand::AcceptJob {
        job,
        ship: ShipId(1),
        expected_revision: session.state.revision,
    });
    assert!(matches!(
        again,
        ServerMessage::CommandRejected(CommandError::Job(_))
    ));
    let dropped = events(session.handle(ClientCommand::AbandonJob {
        job,
        expected_revision: session.state.revision,
    }));
    assert!(matches!(&dropped[0], SessionEvent::JobFinished(outcome) if !outcome.success));
    assert!(session.state.jobs.active.is_empty());
    assert_eq!(session.state.jobs.failed, 1);
}

#[test]
fn queries_never_post_boards_or_settle_jobs() {
    let mut session = Session::with_seed(3);
    session.state.jobs = Default::default();
    let before = session.state.clone();
    session.handle(ClientCommand::GetSession);
    assert_eq!(session.state, before);
}

#[test]
fn a_survey_pays_once_when_the_fleet_reaches_its_body() {
    let mut session = Session::with_seed(3);
    let ship = &session.state.ships[0];
    let system = ship.location.system().unwrap();
    let home = session.economy.current_market(ship).unwrap().id;
    let body = session
        .systems
        .iter()
        .find(|known| known.id == system)
        .unwrap()
        .bodies
        .iter()
        .find(|body| body.id != home.body)
        .unwrap()
        .id;
    session.state.jobs.active.push(Job {
        id: 999,
        kind: JobKind::Survey,
        issuer: home,
        faction: None,
        task: Task::Survey { system, body },
        reward: Credits(12_345),
        standing: 0,
        penalty: 0,
        deadline: 1_000,
        title: "Survey".into(),
        brief: String::new(),
        quest: None,
    });
    let credits = session.economy.credits;
    move_fleet(&mut session, ShipId(1), MoveTarget::Body { system, body });
    assert!(session.state.jobs.active.is_empty());
    assert_eq!(session.state.jobs.completed, 1);
    assert_eq!(session.economy.credits.0, credits.0 + 12_345);
    execute_quiet_run(&mut session);
    assert_eq!(session.economy.credits.0, credits.0 + 12_345, "paid once");
}

#[test]
fn accepted_jobs_survive_save_and_load() {
    let mut session = Session::with_seed(3);
    let job = session.state.jobs.offers[0].id;
    session.handle(ClientCommand::AcceptJob {
        job,
        ship: ShipId(1),
        expected_revision: session.state.revision,
    });
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("jobs.json");
    session.save(&path).unwrap();
    let (loaded, _) = Session::load(&path).unwrap();
    assert_eq!(loaded.state.jobs, session.state.jobs);
}

#[test]
fn friendly_standing_brings_a_quest_offer_with_the_next_order_and_it_saves() {
    let mut session = Session::with_galaxy(
        5,
        game_core::economy::EconomyRules::default(),
        game_core::generation::GalaxyConfig::default(),
    )
    .unwrap();
    session
        .set_up_factions(5, &game_core::factions::FactionSetup::default())
        .unwrap();
    let faction = game_core::factions::Faction::Federation;
    session.economy.factions.standing[faction.index()] = 20;
    let held = events(session.handle(ClientCommand::HoldFleet {
        fleet: game_core::fleet::FleetId(1),
        expected_revision: session.state.revision,
    }));
    let Some(SessionEvent::QuestOffered { job }) = held
        .iter()
        .find(|event| matches!(event, SessionEvent::QuestOffered { .. }))
        .cloned()
    else {
        panic!("no quest offered: {held:?}");
    };
    // Accepted from wherever the ship is.
    events(session.handle(ClientCommand::AcceptJob {
        job,
        ship: ShipId(1),
        expected_revision: session.state.revision,
    }));
    let active = &session.state.jobs.active;
    assert!(
        active
            .iter()
            .any(|taken| taken.id == job && taken.quest.is_some())
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("quest.json");
    session.save(&path).unwrap();
    let (loaded, _) = Session::load(&path).unwrap();
    assert_eq!(loaded.state.jobs, session.state.jobs);
}
