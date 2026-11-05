use super::*;
use game_core::travel::ShipId;

#[test]
fn offline_time_advances_only_for_successful_movement() {
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
    host.command(ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
        expected_revision: host.state.revision,
    });
    assert_eq!(host.state.tick, 15);
    assert_eq!(host.state.ships[0].location, Location::Docked(SystemId(2)));
    assert_eq!(host.state.ships[0].fuel, 25);
    let arrived = host.state.clone();
    host.command(ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
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
