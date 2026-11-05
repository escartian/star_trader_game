use super::*;
use game_core::travel::ShipId;

#[test]
fn offline_clock_preserves_fractional_ticks_and_arrives_without_player_turns() {
    let mut host = LocalSession::new(game_server::Session::default());
    let start = host.clock;
    let message = host.authority.handle(ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
        expected_revision: 0,
    });
    host.accept(message);
    host.advance_to(start + Duration::from_millis(14_900));
    assert_eq!(host.state.tick, 14);
    assert!(matches!(host.state.ship.location, Location::Travelling(_)));
    host.advance_to(start + Duration::from_secs(15));
    assert_eq!(host.state.ship.location, Location::Docked(SystemId(2)));
    assert_eq!(host.state.tick, 15);
    assert_eq!(host.state.ship.fuel, 25);
    let original = host.state.clone();
    host.advance_to(start + Duration::from_secs(15));
    assert_eq!(host.state, original);
}

#[test]
fn speed_changes_only_tick_scheduling_and_pause_prevents_catch_up() {
    let mut host = LocalSession::new(game_server::Session::default());
    let mut expected = host.economy.clone();
    expected.advance(4);
    host.saves.speed = 4;
    host.advance_to(host.clock + Duration::from_secs(1));
    assert_eq!(host.state.tick, 4);
    assert_eq!(host.economy, expected);
    host.saves.paused = true;
    host.clock = Instant::now() - Duration::from_secs(100);
    host.update_clock(&egui::Context::default());
    assert_eq!(host.state.tick, 4);
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
