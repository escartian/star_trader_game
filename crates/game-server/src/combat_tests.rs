use super::{
    Session,
    navigation_tests::{plot, run, succeed, two_fleets},
};
use game_core::{
    BodyId, SystemId,
    combat::CombatOutcome,
    economy::Commodity,
    fleet::{FleetId, ShipKind},
    movement::MoveTarget,
    navigation::{BattleAction, EncounterChoice, EncounterKind},
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn body(id: u64) -> MoveTarget {
    MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(id),
    }
}

pub(super) fn reload(session: &Session) -> Session {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("combat.json");
    session.save(&path).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_none());
    assert_eq!(restored.state, session.state);
    restored
}

/// Runs one navigation run with a guaranteed contact for `ship`'s fleet, then
/// turns that contact into raiders with the given hulls.
pub(super) fn raid(
    session: &mut Session,
    ship: ShipId,
    hold: Option<FleetId>,
    hostiles: Vec<ShipKind>,
) -> u64 {
    session.state.movement_rules.encounter_chance_percent = 100;
    plot(session, ship, body(1));
    if let Some(fleet) = hold {
        let revision = session.state.revision;
        succeed(
            session,
            ClientCommand::HoldFleet {
                fleet,
                expected_revision: revision,
            },
        );
    }
    run(session);
    session.state.movement_rules.encounter_chance_percent = 0;
    let encounter = &mut session.state.navigation.encounters[0];
    encounter.kind = EncounterKind::Raiders;
    encounter.hostiles = hostiles;
    encounter.id
}

pub(super) fn choose(
    session: &mut Session,
    encounter: u64,
    choice: EncounterChoice,
) -> ServerMessage {
    session.handle(ClientCommand::ResolveEncounter {
        encounter,
        choice,
        expected_revision: session.state.revision,
    })
}

#[test]
fn raider_fights_are_reproducible_saved_and_resolve_once() {
    let mut session = Session::default();
    let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Scout]);
    let mut twin = reload(&session);
    for choice in [EncounterChoice::Bypass, EncounterChoice::Investigate] {
        let before = session.snapshot(Vec::new());
        let rejected = choose(&mut session, encounter, choice);
        assert!(matches!(rejected, ServerMessage::CommandRejected(_)));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    choose(&mut session, encounter, EncounterChoice::AutoResolve);
    choose(&mut twin, encounter, EncounterChoice::AutoResolve);
    assert_eq!(session.state, twin.state, "same saved RNG, same fight");
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.encounter, encounter);
    assert!(session.state.navigation.encounters.is_empty());
    // Hull damage on the owned ship matches the saved report exactly.
    let entry = &report.combatants[0];
    assert_eq!(entry.ship, Some(ShipId(1)));
    assert_eq!(session.state.ships[0].hull_damage, entry.final_damage);
    if report.outcome == CombatOutcome::Victory {
        // Salvage comes from wrecks; a raider that fled leaves none.
        let wrecks = report
            .side(game_core::combat::Side::Hostile)
            .filter(|ship| ship.destroyed)
            .count() as u32;
        assert_eq!(report.salvage_ore, 2 * wrecks);
        assert_eq!(
            session.state.ships[0].cargo[Commodity::Ore.index()],
            2 * wrecks
        );
    }
    let resolved = session.snapshot(Vec::new());
    assert!(matches!(
        choose(&mut session, encounter, EncounterChoice::AutoResolve),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), resolved);
    reload(&session);
}

#[test]
fn the_last_owned_ship_survives_crippled_and_loses_its_cargo() {
    let mut session = Session::default();
    session.state.ships[0].cargo[Commodity::Food.index()] = 5;
    let raiders = vec![ShipKind::Patrol; 3];
    let encounter = raid(&mut session, ShipId(1), None, raiders);
    choose(&mut session, encounter, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.outcome, CombatOutcome::Defeat);
    assert_eq!(report.crippled, Some(ShipId(1)));
    assert_eq!(report.cargo_lost, 5);
    let ship = &session.state.ships[0];
    assert_eq!(session.state.ships.len(), 1);
    assert_eq!(ship.hull_damage, 59, "freighter hull 60, one point left");
    assert_eq!(ship.cargo_used(), 0);
    // The crippled fleet's course continues on the next run.
    assert!(session.state.navigation.order(FleetId(1)).is_some());
    reload(&session);
}

#[test]
fn a_destroyed_fleet_is_removed_with_its_course_while_others_continue() {
    let mut session = two_fleets();
    let raiders = vec![ShipKind::Patrol; 3];
    let encounter = raid(&mut session, ShipId(2), Some(FleetId(1)), raiders);
    choose(&mut session, encounter, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.outcome, CombatOutcome::Defeat);
    assert_eq!(report.crippled, None, "another owned ship remains");
    assert!(session.state.ship(ShipId(2)).is_err());
    assert!(
        session
            .state
            .fleets
            .iter()
            .all(|fleet| fleet.id != FleetId(2))
    );
    assert!(session.state.navigation.order(FleetId(2)).is_none());
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    run(&mut session);
    reload(&session);
}

#[test]
fn evasion_either_escapes_untouched_or_opens_a_battle() {
    let mut escaped = false;
    let mut forced = false;
    for seed in 0..40 {
        let mut session = Session::default();
        session.state.navigation.rng_state = seed;
        let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Scout]);
        let before = session.state.navigation.last_combat.clone();
        choose(&mut session, encounter, EncounterChoice::Evade);
        if let Some(open) = &session.state.navigation.battle {
            assert!(open.evasion_failed, "a failed escape opens the battle");
            let revision = session.state.revision;
            succeed(
                &mut session,
                ClientCommand::BattleAction {
                    action: BattleAction::AutoResolve,
                    expected_revision: revision,
                },
            );
        }
        assert!(session.state.navigation.encounters.is_empty());
        if session.state.navigation.last_combat == before {
            escaped = true;
            assert_eq!(session.state.ships[0].hull_damage, 0);
        } else {
            forced = true;
        }
    }
    assert!(escaped && forced, "freighter vs scout evasion is 10%");
}

#[test]
fn repairs_charge_per_point_into_the_market_and_reject_intact_hulls() {
    let mut session = Session::default();
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::RepairShip {
            ship: ShipId(1),
            expected_revision: 0
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    session.state.ships[0].hull_damage = 10;
    let credits = session.economy.credits.0;
    let market = session
        .economy
        .current_market(&session.state.ships[0])
        .unwrap()
        .id;
    let treasury = session.economy.market(market).unwrap().credits.0;
    succeed(
        &mut session,
        ClientCommand::RepairShip {
            ship: ShipId(1),
            expected_revision: 0,
        },
    );
    let cost = 10 * session.economy.rules.repair_point_price.0;
    assert_eq!(session.state.ships[0].hull_damage, 0);
    assert_eq!(session.economy.credits.0, credits - cost);
    assert_eq!(
        session.economy.market(market).unwrap().credits.0,
        treasury + cost
    );
    // Away from a market, repairs are unavailable.
    session.state.ships[0].hull_damage = 5;
    session.state.ships[0].location = Location::InSystem {
        system: SystemId(1),
        position: game_core::LocalPosition {
            x_km: 1_000_000,
            y_km: 0,
            z_km: 0,
        },
    };
    let revision = session.state.revision;
    assert!(matches!(
        session.handle(ClientCommand::RepairShip {
            ship: ShipId(1),
            expected_revision: revision
        }),
        ServerMessage::CommandRejected(_)
    ));
}

#[test]
fn forged_raider_contacts_and_reports_fail_save_validation() {
    let mut session = Session::default();
    let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Scout]);
    assert!(session.export_json().is_ok());
    let mut forged = session.clone();
    forged.state.navigation.encounters[0].hostiles.clear();
    assert!(forged.export_json().is_err(), "raiders without hostiles");
    let mut forged = session.clone();
    forged.state.navigation.encounters[0].kind = EncounterKind::DebrisField;
    assert!(
        forged.export_json().is_err(),
        "hostiles on a non-raider contact"
    );
    choose(&mut session, encounter, EncounterChoice::AutoResolve);
    assert!(session.export_json().is_ok());
    let mut forged = session.clone();
    let report = forged.state.navigation.last_combat.as_mut().unwrap();
    report.combatants[0].final_damage = report.combatants[0].hull + 1;
    assert!(forged.export_json().is_err(), "impossible report damage");
    let mut forged = session.clone();
    let report = forged.state.navigation.last_combat.as_mut().unwrap();
    report.encounter = forged.state.navigation.next_encounter_id;
    assert!(
        forged.export_json().is_err(),
        "report from a future encounter"
    );
}
