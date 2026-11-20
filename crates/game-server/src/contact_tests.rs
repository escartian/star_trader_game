use super::{
    Session,
    combat_tests::{choose, reload},
    navigation_tests::{plot, run},
};
use game_core::{
    BodyId, SystemId,
    economy::Commodity,
    movement::MoveTarget,
    navigation::{EncounterChoice, EncounterKind, NavigationError},
    travel::ShipId,
};
use game_protocol::ServerMessage;

/// The flagship meets a contact of `kind` on its way to a nearby body.
fn contact(kind: EncounterKind) -> (Session, u64) {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 100;
    plot(
        &mut session,
        ShipId(1),
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
    );
    run(&mut session);
    session.state.movement_rules.encounter_chance_percent = 0;
    let encounter = &mut session.state.navigation.encounters[0];
    encounter.kind = kind;
    encounter.hostiles.clear();
    let id = encounter.id;
    (session, id)
}

fn resolve(kind: EncounterKind, choice: EncounterChoice) -> (Session, Session) {
    let (mut session, id) = contact(kind);
    let before = session.clone();
    assert!(matches!(
        choose(&mut session, id, choice),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert!(session.state.navigation.encounters.is_empty());
    assert_eq!(session.state.navigation.payout, 0, "settled at once");
    reload(&session);
    (before, session)
}

fn credits(session: &Session) -> i64 {
    session.economy.credits.0 as i64
}

#[test]
fn salvage_contacts_load_goods_into_free_space() {
    let (before, after) = resolve(
        EncounterKind::DerelictFreighter,
        EncounterChoice::Investigate,
    );
    let gained = after.state.ships[0].cargo_used() - before.state.ships[0].cargo_used();
    assert!((2..=6).contains(&gained), "{gained}");
    // A cargo pod gives goods, or springs a trap that never destroys the ship.
    let (before, after) = resolve(EncounterKind::CargoPod, EncounterChoice::Investigate);
    let goods = after.state.ships[0].cargo_used() > before.state.ships[0].cargo_used();
    let hit = after.state.ships[0].hull_damage > before.state.ships[0].hull_damage;
    assert!(goods != hit);
    assert!(after.state.ships[0].hull_damage < 60);
    // A full hold cannot salvage.
    let (mut session, id) = contact(EncounterKind::DerelictFreighter);
    session.state.ships[0].cargo[Commodity::Food.index()] = 20;
    assert_eq!(
        choose(&mut session, id, EncounterChoice::Investigate),
        ServerMessage::CommandRejected(NavigationError::CargoFull.into())
    );
}

#[test]
fn a_convoy_buys_the_best_cargo_at_a_premium() {
    let (mut session, id) = contact(EncounterKind::MerchantConvoy);
    assert_eq!(
        choose(&mut session, id, EncounterChoice::Trade),
        ServerMessage::CommandRejected(NavigationError::NothingToSell.into())
    );
    session.state.ships[0].cargo[Commodity::Food.index()] = 3;
    session.state.ships[0].cargo[Commodity::Electronics.index()] = 7;
    let before = credits(&session);
    choose(&mut session, id, EncounterChoice::Trade);
    let ship = &session.state.ships[0];
    assert_eq!(ship.cargo[Commodity::Electronics.index()], 2, "five sold");
    assert_eq!(
        ship.cargo[Commodity::Food.index()],
        3,
        "the cheaper good kept"
    );
    assert!(
        credits(&session) > before + 5 * 10_000,
        "electronics list near 120 cr"
    );
}

#[test]
fn law_contacts_charge_fees_or_pay_for_help() {
    let (before, after) = resolve(EncounterKind::CustomsInspection, EncounterChoice::Comply);
    let fee = credits(&before) - credits(&after);
    assert!((2_000..2_100).contains(&fee), "{fee}");
    let (before, after) = resolve(EncounterKind::CustomsInspection, EncounterChoice::Bypass);
    let fine = credits(&before) - credits(&after);
    assert!(fine == 0 || (10_000..10_500).contains(&fine), "{fine}");
    let (before, after) = resolve(EncounterKind::AuthorityCutter, EncounterChoice::Assist);
    assert!(credits(&after) - credits(&before) >= 15_000);
    // A fine never takes the wallet below zero.
    let (mut session, id) = contact(EncounterKind::CustomsInspection);
    session.economy.credits.0 = 500;
    choose(&mut session, id, EncounterChoice::Comply);
    assert_eq!(session.economy.credits.0, 0);
}

#[test]
fn exploration_contacts_pay_for_data_probes_and_fuel() {
    let (before, after) = resolve(EncounterKind::LostProbe, EncounterChoice::Investigate);
    assert!(credits(&after) - credits(&before) >= 12_000);
    let (before, after) = resolve(EncounterKind::Anomaly, EncounterChoice::Investigate);
    let data = credits(&after) - credits(&before);
    assert!(data == 0 || data >= 8_000, "{data}");
    let (mut session, id) = contact(EncounterKind::Comet);
    session.state.ships[0].fuel = 5;
    choose(&mut session, id, EncounterChoice::Investigate);
    assert_eq!(session.state.ships[0].fuel, 15);
    // Choices a contact does not offer are refused.
    let (mut session, id) = contact(EncounterKind::Comet);
    assert_eq!(
        choose(&mut session, id, EncounterChoice::Trade),
        ServerMessage::CommandRejected(NavigationError::InvalidChoice.into())
    );
}

#[test]
fn runs_roll_every_kind_of_contact() {
    let mut seen = std::collections::HashSet::new();
    for seed in 0..200 {
        let mut session = Session::with_seed(seed);
        session.state.navigation.rng_state = seed;
        session.state.movement_rules.encounter_chance_percent = 100;
        plot(
            &mut session,
            ShipId(1),
            MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
        );
        run(&mut session);
        if let Some(encounter) = session.state.navigation.encounters.first() {
            seen.insert(format!("{:?}", encounter.kind));
        }
    }
    assert_eq!(seen.len(), 11, "{seen:?}");
}
