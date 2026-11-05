mod desktop;
mod map;
mod session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = game_server::Session::default();
    let snapshot = session.handle(game_protocol::ClientCommand::GetGalaxy);
    if std::env::args().any(|arg| arg == "--headless") {
        println!("Offline session (in-process authority): {snapshot:#?}");
        return Ok(());
    }
    let game_protocol::ServerMessage::GalaxySnapshot { systems, .. } = snapshot else {
        return Err("Unable to start local galaxy session".into());
    };
    desktop::run(map::Map::new(systems, session))
}
