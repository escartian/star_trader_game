use super::*;
use game_core::{
    BodyId,
    fleet::{FleetId, PRIMARY_FLEET, ShipKind},
    movement::MoveTarget,
    travel::{Location, ShipId},
};

#[test]
fn fleet_one_shows_player_captain_after_taking_command_of_fleet_two() {
    let mut authority = game_server::Session::default();
    let systems = authority.systems().to_vec();
    for command in [
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: 0,
        },
        ClientCommand::SplitFleet {
            fleet: PRIMARY_FLEET,
            ships: vec![ShipId(2)],
            expected_revision: 1,
        },
        ClientCommand::SetFlagship {
            fleet: FleetId(2),
            expected_revision: 2,
        },
    ] {
        assert!(matches!(
            authority.handle(command),
            ServerMessage::SessionSnapshot { .. }
        ));
    }
    let mut host = LocalSession::new(authority);
    host.fleet_panel.open = true;
    assert_eq!(
        host.state.flagship(game_server::LOCAL_PLAYER),
        Some(FleetId(2))
    );
    let ctx = egui::Context::default();
    let mut output = egui::FullOutput::default();
    for _ in 0..4 {
        output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1000.0),
                )),
                ..Default::default()
            },
            |ctx| host.show_fleets(ctx, &systems),
        );
    }
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some((&text.galley.job.text, text.pos.y)),
            _ => None,
        })
        .collect();
    let y = |label: &str| {
        texts
            .iter()
            .find(|(text, _)| text.starts_with(label))
            .map(|(_, y)| *y)
            .unwrap_or_else(|| panic!("missing {label}"))
    };
    assert!(y("Fleet 1") < y("Captain (you),") && y("Captain (you),") < y("Fleet 2"));
    assert_eq!(
        texts
            .iter()
            .filter(|(text, _)| text.contains("(you)"))
            .count(),
        1
    );
}

#[test]
fn plotting_does_not_advance_time_and_navigation_runs_do() {
    use game_core::economy::{Commodity, TradeSide};
    let mut host = LocalSession::new(game_server::Session::default());
    let initial = host.state.clone();
    for _ in 0..20 {
        let _ = egui::Context::default().run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| host.guide(ui));
        });
        host.authority.handle(ClientCommand::GetSession);
    }
    assert_eq!(host.state, initial);
    host.command(ClientCommand::Trade {
        ship: ShipId(1),
        commodity: Commodity::Food,
        quantity: 1,
        side: TradeSide::Buy,
        expected_revision: host.state.revision,
    });
    assert_eq!(host.state.tick, 0);
    host.command(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
        expected_revision: host.state.revision,
    });
    assert_eq!(host.state.tick, 0);
    host.request_execute_run();
    assert_eq!(host.state.tick, 4);
    assert_eq!(
        host.state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
    assert_eq!(host.state.ships[0].fuel, 39);
    let arrived = host.state.clone();
    host.command(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
        expected_revision: host.state.revision,
    });
    assert_eq!(host.state, arrived);
}

#[test]
fn save_controls_restore_player_data_and_failed_loads_preserve_live_state() {
    use game_core::economy::{Commodity, TradeSide};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("slot.json");
    let mut host = LocalSession::new(game_server::Session::with_seed(42));
    host.set_save_path(path.clone());
    host.command(ClientCommand::Trade {
        ship: ShipId(1),
        commodity: Commodity::Food,
        quantity: 2,
        side: TradeSide::Buy,
        expected_revision: host.state.revision,
    });
    host.autosave().unwrap();
    let saved = host.economy.clone();
    host.command(ClientCommand::Trade {
        ship: ShipId(1),
        commodity: Commodity::Food,
        quantity: 1,
        side: TradeSide::Buy,
        expected_revision: host.state.revision,
    });
    assert_ne!(host.economy, saved);
    host.load_slot();
    assert_eq!(host.economy, saved);
    assert_eq!(
        host.take_loaded_galaxy().unwrap(),
        game_core::generation::generate_galaxy(42)
    );
    std::fs::write(path, "invalid JSON").unwrap();
    let before = host.state.clone();
    host.load_slot();
    assert_eq!(host.economy, saved);
    assert_eq!(host.state, before);
    assert!(host.notice.contains("Could not load"));
}

#[test]
fn remembered_slot_and_controls_restore_without_altering_world_state() {
    use crate::preferences::Preferences;
    let directory = tempfile::tempdir().unwrap();
    let settings_path = directory.path().join("settings.json");
    let game_path = directory.path().join("slot-3.json");
    let mut host = LocalSession::new(game_server::Session::default());
    let settings = Preferences {
        save_path: game_path.clone(),
        slot: 3,
        guide: false,
        ..Preferences::default()
    };
    host.apply_preferences(settings.clone(), Some(settings_path.clone()))
        .unwrap();
    host.save_game().unwrap();
    let restored_settings = Preferences::load(&settings_path).unwrap();
    assert_eq!(restored_settings, settings);
    let (session, _) = game_server::Session::load(&restored_settings.save_path).unwrap();
    let mut restored = LocalSession::new(session);
    restored
        .apply_preferences(restored_settings, Some(settings_path))
        .unwrap();
    assert_eq!(restored.saves.path, game_path);
    assert_eq!(restored.saves.slot, 3);
    assert!(!restored.saves.guide);
    assert_eq!(restored.state, host.state);
    assert_eq!(restored.economy, host.economy);
    let invalid = Preferences {
        slot: 0,
        ..settings
    };
    let before = restored.preferences();
    assert!(restored.apply_preferences(invalid, None).is_err());
    assert_eq!(restored.preferences(), before);
}

#[test]
fn dismissing_the_guide_hides_it_and_remembers_the_choice() {
    use crate::preferences::Preferences;
    let directory = tempfile::tempdir().unwrap();
    let settings_path = directory.path().join("settings.json");
    let mut host = LocalSession::new(game_server::Session::default());
    host.apply_preferences(Preferences::default(), Some(settings_path.clone()))
        .unwrap();
    assert!(host.saves.guide);
    host.dismiss_guide();
    assert!(!host.saves.guide);
    assert!(!Preferences::load(&settings_path).unwrap().guide);
    assert!(
        host.notice.contains("Guide in Menu, Session & saves"),
        "{}",
        host.notice
    );
}

#[test]
fn selected_ship_trades_into_its_own_hold_and_restores_from_preferences() {
    use game_core::{
        economy::{Commodity, TradeSide},
        fleet::ShipKind,
    };
    let mut host = LocalSession::new(game_server::Session::default());
    host.command(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    });
    assert_eq!(host.selected_ship().id, ShipId(2));
    host.command(ClientCommand::Trade {
        ship: host.selected_ship().id,
        commodity: Commodity::Food,
        quantity: 2,
        side: TradeSide::Buy,
        expected_revision: host.state.revision,
    });
    assert_eq!(host.state.ships[0].cargo[0], 0);
    assert_eq!(host.state.ships[1].cargo[0], 2);
    assert_eq!(host.state.tick, 0);
    let settings = host.preferences();
    let mut restored = LocalSession::new(host.authority.clone());
    restored.apply_preferences(settings, None).unwrap();
    assert_eq!(restored.selected_ship().id, ShipId(2));
    restored.select_ship(ShipId(999));
    assert_eq!(restored.selected_ship().id, ShipId(2));
    let _ = egui::Context::default().run(egui::RawInput::default(), |ctx| {
        restored.open_fleets();
        restored.show_fleets(ctx, &game_core::generation::generate_galaxy(0));
    });
    assert_eq!(restored.state.tick, 0);
}

#[test]
fn selling_the_selected_ship_keeps_selection_and_preferences_valid() {
    let mut host = LocalSession::new(game_server::Session::default());
    host.command(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: game_core::fleet::ShipKind::Scout,
        expected_revision: 0,
    });
    assert_eq!(host.selected_ship().id, ShipId(2));
    host.command(ClientCommand::SellShip {
        ship: ShipId(2),
        discard_overflow: false,
        expected_revision: 1,
    });
    assert_eq!(host.selected_ship().id, ShipId(1));
    assert_eq!(host.preferences().active_ship, ShipId(1));
    assert_eq!(host.state.ships.len(), 1);
    assert_eq!(host.state.tick, 0);
    let saved = host.state.clone();
    host.command(ClientCommand::SellShip {
        ship: ShipId(2),
        discard_overflow: false,
        expected_revision: 1,
    });
    assert_eq!(host.state, saved);
}

#[test]
fn executing_a_run_selects_the_flagship_when_it_has_no_orders() {
    use game_core::fleet::{FleetId, ShipKind};
    let mut host = LocalSession::new(game_server::Session::default());
    host.command(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    });
    host.command(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: 1,
    });
    // Viewing the secondary fleet; the flagship (Fleet 1) has no orders.
    host.select_ship(ShipId(2));
    let before = host.state.clone();
    host.request_execute_run();
    assert_eq!(host.state, before);
    assert_eq!(host.selected_ship().id, ShipId(1));
    assert!(host.take_focus_request());
    assert!(host.navigation.open);
    assert!(host.notice.contains("Wait order"));
    // Once the flagship waits, the run goes ahead; Fleet 2 never had to.
    host.command(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: host.state.revision,
    });
    host.request_execute_run();
    assert_eq!(host.state.navigation.run, 1);
    assert_eq!(host.state.tick, 4);
}

#[test]
fn trade_options_list_only_fleets_at_a_market_in_the_viewed_system() {
    let session = LocalSession::new(game_server::Session::multi_fleet_scenario(1).unwrap());
    let home = game_core::SystemId(1);
    let options = session.trade_options(home);
    let fleets: Vec<_> = options.iter().map(|option| option.fleet.as_str()).collect();
    // Docked Home Guard (selected, listed first) and Scout Wing at the planet market;
    // the moon has no market and the convoy is underway.
    assert_eq!(fleets, ["Home Guard", "Scout Wing"]);
    assert!(options[0].selected && !options[1].selected);
    assert!(session.trade_options(game_core::SystemId(2)).is_empty());
}

#[test]
fn tapping_a_destination_plots_the_course_without_moving() {
    let mut host = LocalSession::new(game_server::Session::default());
    let before = host.state.ships[0].clone();
    host.select_destination(game_core::SystemId(2), None);
    assert_eq!(host.state.navigation.orders.len(), 1, "{}", host.notice);
    assert_eq!(
        host.state.ships[0], before,
        "plotting does not move or refuel"
    );
    assert_eq!(host.state.tick, 0);
    // A new pick replaces the course rather than adding one.
    host.select_destination(game_core::SystemId(3), None);
    assert_eq!(host.state.navigation.orders.len(), 1);
}

#[test]
fn a_new_captain_sees_every_control_and_a_tip_card_per_milestone() {
    use game_core::progress::Feature;
    let mut authority = game_server::Session::default();
    authority.begin_progression();
    let mut host = LocalSession::new(authority);
    assert!(host.state.progress.unlocked.is_empty(), "no milestone yet");
    // A milestone queues its tip card (and logs it); dismissing removes it.
    let message = host.announce_unlock(Feature::Battles);
    assert!(message.contains("Raider battles"));
    assert!(host.unlock_card_open(Feature::Battles));
    host.announce_unlock(Feature::Battles);
    assert_eq!(
        host.unlock_cards,
        [Feature::Battles],
        "one card per feature"
    );
    // The menu lists everything from the start.
    let ctx = egui::Context::default();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| host.menu_sheet(ui));
    });
    let texts: Vec<String> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    for entry in ["Trade", "Fleets", "Victory"] {
        assert!(texts.iter().any(|text| text == entry), "{entry}: {texts:?}");
    }
}

#[test]
fn map_layers_start_shown_and_toggle_independently() {
    use crate::preferences::MapLayer;
    let mut host = LocalSession::new(game_server::Session::default());
    let options = host.client_options();
    assert!(options.show_traffic && options.show_territory);
    host.toggle_map_layer(MapLayer::Traffic);
    assert!(!host.client_options().show_traffic);
    assert!(host.client_options().show_territory);
    host.toggle_map_layer(MapLayer::Territory);
    host.toggle_map_layer(MapLayer::Traffic);
    let options = host.client_options();
    assert!(options.show_traffic && !options.show_territory);
    // Older preference files without the fields load with both layers shown.
    let old: crate::preferences::ClientOptions = serde_json::from_str("{}").unwrap();
    assert!(old.show_traffic && old.show_territory);
}
