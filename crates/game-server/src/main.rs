fn main() {
    let mut session = game_server::Session::default();
    println!(
        "{:#?}",
        session.handle(game_protocol::ClientCommand::GetGalaxy)
    );
    println!("Dedicated-server fixture only; no network listener yet.");
    println!("For offline play, run: cargo run -p game-client");
}
