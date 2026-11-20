mod app_icon;
mod battle;
#[cfg(feature = "dev-tools")]
mod capture;
mod desktop;
mod map;
mod options;
mod preferences;
mod seed;
mod session;
mod theme;
mod units;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = options::Options::parse()?;
    let settings_path = std::path::PathBuf::from(preferences::DEFAULT_PATH);
    let loaded_settings = if options.capturing() {
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
            && options.economy.is_none()
            && (options.save.is_file() || std::path::Path::new(&backup).is_file()));
    let mut startup_notice = settings_notice;
    #[cfg(feature = "dev-tools")]
    let scenario = options.scenario.clone();
    #[cfg(not(feature = "dev-tools"))]
    let scenario: Option<String> = None;
    let mut startup_error = None;
    let mut session = if let Some(name) = &scenario {
        dev_scenario(name, &options)?
    } else if should_resume {
        match game_server::Session::load(&options.save) {
            Ok((session, recovery)) => {
                if let Some(message) = recovery {
                    eprintln!("{message}");
                    startup_notice = Some(message);
                }
                session
            }
            Err(error) if !options.resume && !options.headless && !options.capturing() => {
                startup_error = Some(error);
                fresh_session(&options)?
            }
            Err(error) => return Err(error.into()),
        }
    } else {
        fresh_session(&options)?
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
    let persist_settings = if !options.capturing() {
        Some(settings_path)
    } else {
        None
    };
    map.apply_preferences(settings, persist_settings)?;
    if !options.new && !options.resume && !options.headless && !options.capturing() {
        map.open_save_chooser(options.save, startup_error);
    }
    if let Some(message) = startup_notice {
        map.notify(message);
    }
    #[cfg(feature = "dev-tools")]
    if let Some(path) = options.capture {
        map.prepare_capture(&options.capture_view)?;
        return pollster::block_on(capture::save(&mut map, &path, options.capture_size));
    }
    desktop::run(map)
}

fn fresh_session(
    options: &options::Options,
) -> Result<game_server::Session, Box<dyn std::error::Error>> {
    let economy_path = options.economy.clone().or_else(|| {
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
        rules.ship_catalog = game_core::fleet::ShipCatalog::from_json(&std::fs::read_to_string(
            "config/ships.json",
        )?)?;
    }
    if std::path::Path::new("config/combat.json").is_file() {
        rules.combat_catalog = game_core::combat::CombatCatalog::from_json(
            &std::fs::read_to_string("config/combat.json")?,
        )?;
        rules.validate()?;
    }
    if std::path::Path::new("config/planets.json").is_file() {
        rules.planet_catalog = game_core::planet::PlanetCatalog::from_json(
            &std::fs::read_to_string("config/planets.json")?,
        )?;
    }
    let seed = if options.seed_explicit || options.capturing() {
        options.seed
    } else {
        seed::new_seed()
    };
    let mut authority = if options.capturing() && !options.galaxy_explicit {
        game_server::Session::with_economy(seed, rules)?
    } else {
        game_server::Session::with_galaxy(seed, rules, options.galaxy)?
    };
    if std::path::Path::new("config/movement.json").is_file() {
        authority.set_movement_rules(game_core::movement::MovementRules::from_json(
            &std::fs::read_to_string("config/movement.json")?,
        )?)?;
    }
    Ok(authority)
}

/// Build a named test scenario and save it immediately, so `--continue` can reload it.
#[cfg(feature = "dev-tools")]
fn dev_scenario(
    name: &str,
    options: &options::Options,
) -> Result<game_server::Session, Box<dyn std::error::Error>> {
    let session = match name {
        "multi-fleet" => game_server::Session::multi_fleet_scenario(options.seed)?,
        "fleet-battle" => game_server::Session::fleet_battle_scenario(options.seed)?,
        other => {
            return Err(
                format!("Unknown scenario {other}; available: multi-fleet, fleet-battle").into(),
            );
        }
    };
    if !options.capturing() {
        session.save(&options.save)?;
        println!("Saved scenario {name} to {}", options.save.display());
    }
    Ok(session)
}

#[cfg(not(feature = "dev-tools"))]
fn dev_scenario(
    _: &str,
    _: &options::Options,
) -> Result<game_server::Session, Box<dyn std::error::Error>> {
    unreachable!("scenarios are parsed only in dev-tools builds")
}
