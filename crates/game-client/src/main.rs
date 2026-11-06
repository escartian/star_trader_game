mod capture;
mod desktop;
mod map;
mod options;
mod preferences;
mod session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = options::Options::parse()?;
    let settings_path = std::path::PathBuf::from(preferences::DEFAULT_PATH);
    let loaded_settings = if options.capture.is_some() {
        Ok(preferences::Preferences::default())
    } else {
        preferences::Preferences::load(&settings_path)
    };
    let (mut settings, settings_notice) = match loaded_settings {
        Ok(settings) => (settings, None),
        Err(error) => (
            preferences::Preferences::default(),
            Some(format!("Client settings reset to defaults: {error}")),
        ),
    };
    if !options.save_explicit {
        options.save = settings.save_path.clone();
    }
    settings.save_path = options.save.clone();
    settings.validate()?;
    let mut backup = options.save.as_os_str().to_os_string();
    backup.push(".bak");
    let should_resume = options.resume
        || (!options.new
            && options.seed == 0
            && options.economy.is_none()
            && (options.save.is_file() || std::path::Path::new(&backup).is_file()));
    let mut startup_notice = settings_notice;
    let mut session = if should_resume {
        let (session, recovery) = game_server::Session::load(&options.save)?;
        if let Some(message) = recovery {
            eprintln!("{message}");
            startup_notice = Some(message);
        }
        session
    } else {
        let economy_path = options.economy.or_else(|| {
            std::path::Path::new("config/economy.json")
                .is_file()
                .then(|| "config/economy.json".into())
        });
        let mut rules = if let Some(path) = economy_path {
            game_core::economy::EconomyRules::from_json(&std::fs::read_to_string(path)?)?
        } else {
            game_core::economy::EconomyRules::default()
        };
        if std::path::Path::new("config/ships.json").is_file() {
            rules.ship_catalog = game_core::fleet::ShipCatalog::from_json(
                &std::fs::read_to_string("config/ships.json")?,
            )?;
        }
        if std::path::Path::new("config/planets.json").is_file() {
            rules.planet_catalog = game_core::planet::PlanetCatalog::from_json(
                &std::fs::read_to_string("config/planets.json")?,
            )?;
        }
        let mut authority = game_server::Session::with_economy(options.seed, rules)?;
        if std::path::Path::new("config/movement.json").is_file() {
            authority.set_movement_rules(game_core::movement::MovementRules::from_json(
                &std::fs::read_to_string("config/movement.json")?,
            )?)?;
        }
        authority
    };
    if options.headless {
        println!("{}", session.export_json()?);
        return Ok(());
    }
    let game_protocol::ServerMessage::GalaxySnapshot { systems, .. } =
        session.handle(game_protocol::ClientCommand::GetGalaxy)
    else {
        return Err("Unable to start local galaxy session".into());
    };
    let mut map = map::Map::new(systems, session);
    let persist_settings = if options.capture.is_none() {
        Some(settings_path)
    } else {
        None
    };
    map.apply_preferences(settings, persist_settings)?;
    if let Some(message) = startup_notice {
        map.notify(message);
    }
    if let Some(path) = options.capture {
        map.prepare_capture(&options.capture_view)?;
        return pollster::block_on(capture::save(&mut map, &path));
    }
    desktop::run(map)
}
