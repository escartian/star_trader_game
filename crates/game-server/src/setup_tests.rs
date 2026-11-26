use super::{DangerLevel, GameSetup, Session};
use game_core::{economy::Credits, economy::EconomyRules, generation::GalaxyConfig};

fn game(setup: &GameSetup) -> Result<Session, String> {
    Session::new_game(3, EconomyRules::default(), Default::default(), setup)
}

#[test]
fn port_residents_mix_each_run_and_survive_a_save() {
    let mut session = Session::default();
    let before = session.economy.markets[0].residents_percent();
    let population = session.economy.markets[0].total_population;
    crate::test_support::execute_quiet_run(&mut session);
    let after = session.economy.markets[0].residents_percent();
    assert_ne!(before, after);
    assert_eq!(session.economy.markets[0].total_population, population);
    let restored = super::combat_tests::reload(&session);
    assert_eq!(restored.economy.markets[0].residents_percent(), after);
    assert_eq!(restored.economy.markets[0].total_population, population);
}

#[test]
fn every_setup_choice_becomes_a_session_rule() {
    let setup = GameSetup {
        captain_name: "  Ilsa \t  Vey\n".into(),
        species: game_core::species::Species::Miri,
        galaxy: GalaxyConfig {
            systems: 12,
            radius_ly: 40,
            max_objects: 9,
        },
        starting_credits: Credits(500_000),
        inflation_basis_points: 1_000,
        danger: DangerLevel::Perilous,
        run_limit: Some(400),
        wealth_share_percent: 25,
        factions: game_core::factions::FactionSetup {
            in_play: [false, true, true, false, false, false],
            allegiance: Some(game_core::factions::Faction::Empire),
            coverage: game_core::factions::Coverage::Full,
            relations: game_core::factions::RelationMode::Random,
        },
    };
    let session = game(&setup).unwrap();
    assert_eq!(session.state.captain_name, "Ilsa Vey", "cleaned on start");
    assert_eq!(session.state.captain_title(), "Captain Ilsa Vey");
    assert_eq!(
        super::combat_tests::reload(&session).state.captain_name,
        "Ilsa Vey"
    );
    assert_eq!(
        session.state.player_species,
        game_core::species::Species::Miri
    );
    assert_eq!(
        session.state.fleets[0].captain.species,
        game_core::species::Species::Miri
    );
    let factions = &session.economy.factions;
    assert_eq!(
        factions.allegiance,
        Some(game_core::factions::Faction::Empire)
    );
    assert_eq!(
        factions.territory.len(),
        12,
        "full coverage claims every system"
    );
    assert_eq!(session.systems.len(), 12);
    assert_eq!(session.economy.credits, Credits(500_000));
    let rules = &session.economy.rules;
    assert_eq!(rules.dynamics.inflation_ppm_per_tick, 0);
    assert_eq!(rules.dynamics.inflation_basis_points_per_100_runs, 1_000);
    assert!(rules.dynamics.inflation_ppb_per_tick > 0);
    assert_eq!(rules.campaign.wealth_share_percent, 25);
    assert_eq!(rules.npc_raid_chance_percent, 8);
    assert_eq!(session.state.movement_rules.encounter_chance_percent, 20);
    assert_eq!(session.state.campaign.run_limit, Some(400));
    // Perilous bands carry an extra scout from the start.
    assert!(
        session
            .state
            .navigation
            .bands
            .iter()
            .all(|band| band.ships.len() == 3)
    );
    session.export_json().unwrap();
    assert_eq!(
        super::combat_tests::reload(&session).state.player_species,
        setup.species
    );
    let calm = game(&GameSetup {
        danger: DangerLevel::Calm,
        inflation_basis_points: 0,
        ..GameSetup::default()
    })
    .unwrap();
    assert_eq!(calm.state.movement_rules.encounter_chance_percent, 6);
    assert_eq!(calm.economy.rules.dynamics.inflation_ppm_per_tick, 0);
    assert_eq!(
        calm.economy
            .rules
            .dynamics
            .inflation_basis_points_per_100_runs,
        0
    );
    assert!(
        calm.state
            .navigation
            .bands
            .iter()
            .all(|band| band.ships.len() == 2)
    );
}

#[test]
fn impossible_setups_are_rejected() {
    assert_eq!(GameSetup::default().inflation_basis_points, 0);
    assert!(
        game(&GameSetup {
            inflation_basis_points: 10_001,
            ..GameSetup::default()
        })
        .is_err()
    );
    let bad_length = GameSetup {
        run_limit: Some(0),
        ..GameSetup::default()
    };
    assert!(game(&bad_length).is_err());
    let bad_galaxy = GameSetup {
        galaxy: GalaxyConfig {
            systems: 2,
            ..GalaxyConfig::default()
        },
        ..GameSetup::default()
    };
    assert!(game(&bad_galaxy).is_err());
}

#[test]
fn custom_inflation_compounds_to_its_chosen_hundred_run_rate() {
    let mut session = game(&GameSetup {
        inflation_basis_points: 10_000,
        ..GameSetup::default()
    })
    .unwrap();
    session.economy.advance(400);
    assert!(session.economy.price_level_ppm.abs_diff(2_000_000) <= 2);
}

#[test]
fn a_new_game_starts_as_a_captain_and_unlocks_arrive_as_events() {
    use game_core::progress::Feature;
    use game_protocol::{ClientCommand, ServerMessage, SessionEvent};
    let mut session = game(&GameSetup::default()).unwrap();
    assert!(session.state.progress.unlocked.is_empty());
    // A sale counts as a delivery.
    session.state.ships[0].cargo[0] = 2;
    let revision = session.state.revision;
    let sold = session.handle(ClientCommand::Trade {
        ship: game_core::travel::ShipId(1),
        commodity: game_core::economy::Commodity::Food,
        quantity: 1,
        side: game_core::economy::TradeSide::Sell,
        expected_revision: revision,
    });
    assert!(matches!(sold, ServerMessage::SessionSnapshot { .. }));
    assert_eq!(session.state.progress.deliveries, 1);
    // Profit opens the shipyard, announced once.
    session.economy.credits = Credits(session.economy.credits.0 + 100_000);
    let ServerMessage::SessionSnapshot { events, state, .. } =
        session.handle(ClientCommand::GetSession)
    else {
        panic!("snapshot");
    };
    assert!(events.contains(&SessionEvent::Unlocked {
        feature: Feature::Fleets
    }));
    assert!(state.progress.has(Feature::Fleets));
    let ServerMessage::SessionSnapshot { events, .. } = session.handle(ClientCommand::GetSession)
    else {
        panic!("snapshot");
    };
    assert!(events.is_empty(), "announced once");
    crate::combat_tests::reload(&session);
}
