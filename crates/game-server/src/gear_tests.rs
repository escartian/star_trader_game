//! Anti-piracy gear and hired escorts (`game_core::combat::gear`).
use super::{
    Session,
    combat_tests::{choose, raid, reload},
    test_support::execute_quiet_run,
};
use game_core::{
    combat::{
        Side,
        gear::{AFTERBURNER_EVADE_PERCENT, ESCORT_KIND, ESCORT_RUNS, Gear},
    },
    economy::Credits,
    fleet::ShipKind,
    navigation::EncounterChoice,
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

fn ship_as(session: &mut Session, kind: ShipKind) {
    let spec = session.economy.rules.ship_catalog.spec(kind).unwrap();
    let ship = &mut session.state.ships[0];
    ship.kind = kind;
    ship.base_cargo_capacity = spec.cargo_capacity;
    ship.cargo_capacity = spec.cargo_capacity;
    ship.fuel_capacity = spec.fuel_capacity;
    ship.fuel = spec.fuel_capacity;
    ship.range_ly = spec.range_ly;
}

fn fit(session: &mut Session, gear: Gear) -> ServerMessage {
    session.handle(ClientCommand::FitGear {
        ship: ShipId(1),
        gear,
        expected_revision: session.state.revision,
    })
}

fn hire(session: &mut Session) -> ServerMessage {
    session.handle(ClientCommand::HireEscort {
        ship: ShipId(1),
        expected_revision: session.state.revision,
    })
}

fn ok(message: ServerMessage) {
    assert!(
        matches!(message, ServerMessage::SessionSnapshot { .. }),
        "{message:?}"
    );
}

#[test]
fn gear_is_fitted_once_at_a_market_and_paid_into_its_treasury() {
    let mut session = Session::default();
    let market = session
        .economy
        .current_market(&session.state.ships[0])
        .unwrap()
        .id;
    let treasury = session.economy.market(market).unwrap().credits;
    let credits = session.economy.credits;
    let price = session.economy.gear_price(Gear::Afterburners);
    ok(fit(&mut session, Gear::Afterburners));
    assert_eq!(session.state.ships[0].gear, vec![Gear::Afterburners]);
    assert_eq!(session.economy.credits.0, credits.0 - price.0);
    assert_eq!(
        session.economy.market(market).unwrap().credits.0,
        treasury.0 + price.0
    );
    assert!(matches!(
        fit(&mut session, Gear::Afterburners),
        ServerMessage::CommandRejected(_)
    ));
    session.economy.credits = Credits(0);
    assert!(matches!(
        fit(&mut session, Gear::PointDefense),
        ServerMessage::CommandRejected(_)
    ));
    reload(&session);
}

#[test]
fn model_upgrades_change_only_their_ship_and_reject_incompatible_hulls() {
    let cases = [
        (ShipKind::Scout, Gear::ScoutDrive),
        (ShipKind::Freighter, Gear::FreighterRacks),
        (ShipKind::Patrol, Gear::PatrolTargeting),
        (ShipKind::Prospector, Gear::ProspectorDrill),
    ];
    for (kind, gear) in cases {
        let mut session = Session::default();
        ship_as(&mut session, kind);
        session.economy.credits = Credits(1_000_000);
        let before = session.state.ships[0].clone();
        let wallet = session.economy.credits;
        let wrong = if gear == Gear::ScoutDrive {
            Gear::FreighterRacks
        } else {
            Gear::ScoutDrive
        };
        assert!(matches!(
            fit(&mut session, wrong),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.state.ships[0], before);
        assert_eq!(session.economy.credits, wallet);
        ok(fit(&mut session, gear));
        let ship = &session.state.ships[0];
        assert!(ship.gear.contains(&gear));
        assert_eq!(
            ship.range_ly,
            before.range_ly + u32::from(gear == Gear::ScoutDrive) * 4
        );
        assert_eq!(
            ship.cargo_capacity,
            before.cargo_capacity + u32::from(gear == Gear::FreighterRacks) * 20
        );
        assert!(session.economy.credits < wallet);
        reload(&session);
    }
}

#[test]
fn size_upgrades_fit_only_their_size_and_survive_saves() {
    for (kind, gear) in [
        (ShipKind::Scout, Gear::EvasionJets),
        (ShipKind::Freighter, Gear::ShieldArray),
        (ShipKind::Prospector, Gear::ArmoredBulkheads),
    ] {
        let mut session = Session::default();
        ship_as(&mut session, kind);
        session.economy.credits = Credits(1_000_000);
        ok(fit(&mut session, gear));
        let restored = reload(&session);
        assert_eq!(restored.state.ships[0].gear, [gear]);
        assert!(matches!(
            fit(&mut session, Gear::EvasionJets),
            ServerMessage::CommandRejected(_)
        ));
    }
}

#[test]
fn freighter_racks_stack_with_the_first_hold_upgrade() {
    let mut session = Session::default();
    session.economy.credits = Credits(1_000_000);
    let base = session.state.ships[0].cargo_capacity;
    ok(fit(&mut session, Gear::FreighterRacks));
    ok(session.handle(ClientCommand::UpgradeHold {
        ship: ShipId(1),
        expected_revision: session.state.revision,
    }));
    assert_eq!(session.state.ships[0].cargo_capacity, base + 40);
    assert_eq!(reload(&session).state.ships[0].cargo_capacity, base + 40);
}

#[test]
fn saves_reject_incompatible_fittings_and_missing_drive_range() {
    let mut session = Session::default();
    session.state.ships[0].gear.push(Gear::ScoutDrive);
    assert!(session.export_json().is_err());
    session.state.ships[0].gear.clear();
    ship_as(&mut session, ShipKind::Scout);
    session.state.ships[0].gear.push(Gear::ScoutDrive);
    assert!(session.export_json().is_err());
}

#[test]
fn afterburners_on_every_ship_improve_the_odds_of_evading() {
    let mut session = Session::default();
    let fleet = session.state.fleets[0].clone();
    let rules = session.economy.rules.clone();
    let raiders = [ShipKind::Scout];
    let base = session
        .state
        .evade_chance(&fleet, &raiders, &rules)
        .unwrap();
    ok(fit(&mut session, Gear::Afterburners));
    let boosted = session
        .state
        .evade_chance(&fleet, &raiders, &rules)
        .unwrap();
    assert_eq!(boosted, base + AFTERBURNER_EVADE_PERCENT);
}

#[test]
fn an_escort_contract_runs_down_and_can_be_renewed() {
    let mut session = Session::default();
    let fleet = session.state.fleets[0].id;
    ok(hire(&mut session));
    assert_eq!(session.state.navigation.escort_runs(fleet), ESCORT_RUNS);
    execute_quiet_run(&mut session);
    execute_quiet_run(&mut session);
    assert_eq!(session.state.navigation.escort_runs(fleet), ESCORT_RUNS - 2);
    let mut session = reload(&session);
    ok(hire(&mut session));
    assert_eq!(session.state.navigation.escort_runs(fleet), ESCORT_RUNS);
    for _ in 0..ESCORT_RUNS {
        execute_quiet_run(&mut session);
    }
    assert_eq!(session.state.navigation.escort_runs(fleet), 0);
    assert!(session.state.navigation.escorts.is_empty());
}

#[test]
fn a_hired_escort_fights_beside_the_fleet_and_is_lost_if_destroyed() {
    let mut lost = false;
    for seed in 0..30 {
        let mut session = Session::default();
        session.state.navigation.rng_state = seed;
        let fleet = session.state.fleets[0].id;
        ok(hire(&mut session));
        let encounter = raid(&mut session, ShipId(1), None, vec![ShipKind::Patrol; 3]);
        ok(choose(&mut session, encounter, EncounterChoice::Fight));
        let battle = session.state.navigation.battle.clone().unwrap().battle;
        let escorts: Vec<_> = battle
            .units
            .iter()
            .filter(|unit| unit.side == Side::Player && unit.ship.is_none())
            .collect();
        assert_eq!(escorts.len(), 1);
        assert_eq!(escorts[0].kind, ESCORT_KIND);
        // The open battle with its escort survives a save.
        let mut session = reload(&session);
        ok(session.handle(ClientCommand::BattleAction {
            action: game_core::navigation::BattleAction::AutoResolve,
            expected_revision: session.state.revision,
        }));
        let report = session.state.navigation.last_combat.clone().unwrap();
        let destroyed = report
            .side(Side::Player)
            .any(|unit| unit.ship.is_none() && unit.destroyed);
        assert_eq!(
            session.state.navigation.escort_runs(fleet) == 0,
            destroyed,
            "seed {seed}"
        );
        lost |= destroyed;
        if !session.state.fleets.is_empty() && session.state.campaign.ended.is_none() {
            reload(&session);
        }
    }
    assert!(lost, "three patrols destroy an escort sometimes");
}

#[test]
fn wealthy_traders_arm_themselves_and_hire_escorts() {
    let mut session = Session::default();
    session.economy.rules.npc_raid_chance_percent = 0;
    let start = session.economy.rules.npc_starting_credits.0;
    for trader in &mut session.economy.traders {
        trader.credits = Credits(start * 10);
    }
    let mut armed = false;
    for _ in 0..12 {
        execute_quiet_run(&mut session);
        let trader = &session.economy.traders[0];
        if trader.state.ships[0].gear.len() == Gear::NPC.len() && trader.escort_runs > 0 {
            armed = true;
            break;
        }
    }
    assert!(armed, "{:?}", session.economy.traders[0]);
    assert!(
        session
            .economy
            .traders
            .iter()
            .all(|trader| trader.credits.0 >= start * 2),
        "traders keep a trading reserve"
    );
    reload(&session);
}

#[test]
fn hailing_an_escorted_trader_brings_its_escort_into_the_fight() {
    let mut session = Session::default();
    // Meet trader 0 in orbit at a planet market (see `piracy_tests`).
    execute_quiet_run(&mut session);
    let market = session
        .economy
        .markets
        .iter()
        .find(|market| !market.is_station)
        .unwrap()
        .id;
    let at = game_core::npc::market_location(market, &session.systems);
    let trader = &mut session.economy.traders[0];
    trader.state.navigation.orders.clear();
    trader.state.ships[0].location = at.clone();
    trader.state.ships[0].gear = vec![Gear::PointDefense];
    trader.escort_runs = 4;
    let id = trader.id;
    session.state.navigation.orders.clear();
    session.state.ships[0].location = at;
    let fleet = session.state.fleets[0].id;
    ok(session.handle(ClientCommand::HailTrader {
        fleet,
        trader: id,
        expected_revision: session.state.revision,
    }));
    let contact = session.state.navigation.encounters[0].clone();
    assert_eq!(contact.hostiles, vec![ShipKind::Freighter, ESCORT_KIND]);
    ok(choose(&mut session, contact.id, EncounterChoice::Fight));
    let battle = session.state.navigation.battle.clone().unwrap().battle;
    let hostile: Vec<_> = battle
        .units
        .iter()
        .filter(|unit| unit.side == Side::Hostile)
        .collect();
    assert_eq!(hostile.len(), 2);
    assert_eq!(hostile[0].gear, vec![Gear::PointDefense]);
    reload(&session);
}
