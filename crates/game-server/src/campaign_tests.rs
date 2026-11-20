use super::{
    LOCAL_PLAYER, Session,
    band_tests::near_band,
    combat_tests::{choose, reload},
    navigation_tests::{run, succeed},
};
use game_core::{
    campaign::{CampaignEnd, Road},
    economy::Credits,
    fleet::{FleetId, ShipKind},
    navigation::{EncounterChoice, NavigationEvent},
    routes::TradeRoute,
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn act(session: &mut Session, command: impl Fn(u64) -> ClientCommand) -> ServerMessage {
    let revision = session.state.revision;
    session.handle(command(revision))
}

fn rejected(message: &ServerMessage) -> bool {
    matches!(message, ServerMessage::CommandRejected(_))
}

/// A patrol ship orbiting the Haven's body after one quiet run, with the
/// garrison cut to a single scout.
fn at_the_haven() -> Session {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    let wait = |revision| ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: revision,
    };
    act(&mut session, wait);
    run(&mut session);
    let haven = session.state.navigation.haven.as_mut().unwrap();
    haven.garrison = vec![ShipKind::Scout];
    haven.sunk = 7;
    let (system, body) = (haven.system, haven.body);
    session.state.ships[0].kind = ShipKind::Patrol;
    session.state.ships[0].location = Location::Orbiting { system, body };
    session
}

fn assault(fleet: FleetId) -> impl Fn(u64) -> ClientCommand {
    move |revision| ClientCommand::AssaultHaven {
        fleet,
        expected_revision: revision,
    }
}

#[test]
fn assaults_start_only_at_the_haven_and_can_be_called_off() {
    let mut session = Session::default();
    assert!(rejected(&act(&mut session, assault(FleetId(1)))), "at home");
    let mut session = at_the_haven();
    assert!(
        rejected(&act(&mut session, assault(FleetId(9)))),
        "unknown fleet"
    );
    let message = act(&mut session, assault(FleetId(1)));
    assert!(!rejected(&message));
    let contact = session.state.navigation.encounters[0].clone();
    assert!(contact.haven && contact.hostiles == vec![ShipKind::Scout]);
    assert!(
        rejected(&act(&mut session, assault(FleetId(1)))),
        "one at a time"
    );
    reload(&session);
    choose(&mut session, contact.id, EncounterChoice::Evade);
    assert!(session.state.navigation.encounters.is_empty());
    assert!(session.state.navigation.haven.as_ref().unwrap().standing());
}

#[test]
fn destroying_the_haven_wins_the_soldier_road_and_bands_stop_returning() {
    let mut session = at_the_haven();
    act(&mut session, assault(FleetId(1)));
    let contact = session.state.navigation.encounters[0].clone();
    choose(&mut session, contact.id, EncounterChoice::AutoResolve);
    let haven = session.state.navigation.haven.clone().unwrap();
    assert!(
        !haven.standing(),
        "{:?}",
        session.state.navigation.last_combat
    );
    assert_eq!(haven.sunk, 8);
    let run_now = session.state.navigation.run;
    assert_eq!(
        session.state.campaign.ended,
        Some(CampaignEnd::Won {
            road: Road::Soldier,
            run: run_now
        })
    );
    // A win still plays on, but wiped-out bands no longer return.
    session
        .state
        .navigation
        .band_respawns
        .push((game_core::SystemId(2), run_now));
    run(&mut session);
    assert!(session.state.navigation.band_respawns.is_empty());
    reload(&session);
}

#[test]
fn charters_are_bought_at_a_port_once_and_count_toward_the_trader_road() {
    let mut session = Session::default();
    let buy = |revision| ClientCommand::BuyCharter {
        ship: ShipId(1),
        expected_revision: revision,
    };
    assert!(rejected(&act(&mut session, buy)), "1,000 cr is not enough");
    session.economy.credits = Credits(2_000_000);
    assert!(!rejected(&act(&mut session, buy)));
    assert_eq!(session.economy.chartered_by(LOCAL_PLAYER).len(), 1);
    assert_eq!(session.economy.credits, Credits(1_500_000));
    assert!(rejected(&act(&mut session, buy)), "already chartered");
    reload(&session);
    // Three charters complete the Trader road on the next command.
    let others: Vec<_> = session.economy.markets[1..3].iter().map(|m| m.id).collect();
    for market in others {
        session.economy.charters.push(game_core::economy::Charter {
            market,
            owner: LOCAL_PLAYER,
        });
    }
    succeed(&mut session, ClientCommand::GetSession);
    assert!(matches!(
        session.state.campaign.ended,
        Some(CampaignEnd::Won {
            road: Road::Trader,
            ..
        })
    ));
}

#[test]
fn routes_on_a_chartered_lane_are_never_struck_by_bands() {
    // Lane membership: both ends chartered by the player.
    let mut session = Session::default();
    let stops = [session.economy.markets[0].id, session.economy.markets[1].id];
    session.state.navigation.routes.push(TradeRoute {
        fleet: FleetId(1),
        stops,
        paused: false,
    });
    assert!(session.lane_fleets(LOCAL_PLAYER).is_empty());
    for market in stops {
        session.economy.charters.push(game_core::economy::Charter {
            market,
            owner: LOCAL_PLAYER,
        });
    }
    assert_eq!(session.lane_fleets(LOCAL_PLAYER), vec![FleetId(1)]);
    // A fleet on a lane is never hunted, where it would otherwise be struck.
    let struck = |on_lane: &[FleetId]| {
        let mut session = near_band(1_000_000);
        (0..4).any(|_| {
            let systems = session.systems.clone();
            session
                .state
                .execute_navigation_run_with(LOCAL_PLAYER, &systems, on_lane)
                .unwrap()
                .iter()
                .any(|event| matches!(event, NavigationEvent::EncounterFound(_)))
        })
    };
    assert!(struck(&[]));
    assert!(!struck(&[FleetId(1)]));
}

#[test]
fn a_run_limit_ends_the_game_on_score_and_freezes_it() {
    let mut session = Session::default();
    let limit = |runs| {
        move |revision| ClientCommand::SetRunLimit {
            runs,
            expected_revision: revision,
        }
    };
    assert!(rejected(&act(&mut session, limit(Some(0)))));
    assert!(!rejected(&act(&mut session, limit(Some(1)))));
    assert_eq!(session.state.campaign.run_limit, Some(1));
    act(&mut session, |revision| ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: revision,
    });
    run(&mut session);
    assert_eq!(
        session.state.campaign.ended,
        Some(CampaignEnd::TimeUp { run: 1 })
    );
    assert!(
        rejected(&act(&mut session, limit(None))),
        "the game is over"
    );
    reload(&session);
}

#[test]
fn saves_from_before_the_haven_get_the_one_a_new_game_places() {
    let session = Session::default();
    let mut value: serde_json::Value =
        serde_json::from_str(&session.export_json().unwrap()).unwrap();
    value["player"]["navigation"]
        .as_object_mut()
        .unwrap()
        .remove("haven");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("before-haven.json");
    std::fs::write(&path, value.to_string()).unwrap();
    let (loaded, _) = Session::load(&path).unwrap();
    assert_eq!(
        loaded.state.navigation.haven,
        session.state.navigation.haven
    );
    assert!(loaded.state.navigation.haven.is_some());
}
