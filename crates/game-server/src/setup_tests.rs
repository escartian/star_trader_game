use super::{DangerLevel, GameSetup, InflationLevel, Session};
use game_core::{economy::Credits, economy::EconomyRules, generation::GalaxyConfig};

fn game(setup: &GameSetup) -> Result<Session, String> {
    Session::new_game(3, EconomyRules::default(), Default::default(), setup)
}

#[test]
fn every_setup_choice_becomes_a_session_rule() {
    let setup = GameSetup {
        galaxy: GalaxyConfig {
            systems: 12,
            radius_ly: 40,
            max_objects: 9,
        },
        starting_credits: Credits(500_000),
        inflation: InflationLevel::High,
        danger: DangerLevel::Perilous,
        run_limit: Some(400),
        wealth_share_percent: 25,
    };
    let session = game(&setup).unwrap();
    assert_eq!(session.systems.len(), 12);
    assert_eq!(session.economy.credits, Credits(500_000));
    let rules = &session.economy.rules;
    assert_eq!(rules.dynamics.inflation_ppm_per_tick, 250);
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
    let calm = game(&GameSetup {
        danger: DangerLevel::Calm,
        inflation: InflationLevel::None,
        ..GameSetup::default()
    })
    .unwrap();
    assert_eq!(calm.state.movement_rules.encounter_chance_percent, 6);
    assert_eq!(calm.economy.rules.dynamics.inflation_ppm_per_tick, 0);
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
