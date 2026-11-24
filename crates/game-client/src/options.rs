use game_core::generation::GalaxyConfig;
use std::path::PathBuf;

/// Extra flags listed in help only when the developer tooling is compiled in.
#[cfg(feature = "dev-tools")]
const DEV_OPTIONS: &str = ", --capture PATH, --capture-view VIEW, --capture-size WxH, --scenario multi-fleet|fleet-battle[:NvM]";
#[cfg(not(feature = "dev-tools"))]
const DEV_OPTIONS: &str = " (capture and scenario flags need a dev-tools build)";

pub struct Options {
    pub headless: bool,
    pub resume: bool,
    pub new: bool,
    #[cfg(feature = "dev-tools")]
    pub capture: Option<PathBuf>,
    #[cfg(feature = "dev-tools")]
    pub capture_view: String,
    /// Capture size in pixels, e.g. a phone's 390 × 844.
    #[cfg(feature = "dev-tools")]
    pub capture_size: (u32, u32),
    pub seed: u64,
    pub seed_explicit: bool,
    pub galaxy: GalaxyConfig,
    pub galaxy_explicit: bool,
    pub save: PathBuf,
    pub save_explicit: bool,
    pub economy: Option<PathBuf>,
    /// Named test scenario to build and save before starting ("multi-fleet", "fleet-battle" or "fleet-battle:NvM").
    #[cfg(feature = "dev-tools")]
    pub scenario: Option<String>,
}

impl Options {
    /// True for offscreen capture runs, which never touch saved settings.
    pub fn capturing(&self) -> bool {
        #[cfg(feature = "dev-tools")]
        return self.capture.is_some();
        #[cfg(not(feature = "dev-tools"))]
        false
    }

    pub fn parse() -> Result<Self, String> {
        let mut options = Self {
            headless: false,
            resume: false,
            new: false,
            #[cfg(feature = "dev-tools")]
            capture: None,
            #[cfg(feature = "dev-tools")]
            capture_view: "galaxy".into(),
            #[cfg(feature = "dev-tools")]
            capture_size: crate::capture::DEFAULT_SIZE,
            seed: 0,
            seed_explicit: false,
            galaxy: GalaxyConfig::default(),
            galaxy_explicit: false,
            save: PathBuf::from("data/slot-1.json"),
            save_explicit: false,
            economy: None,
            #[cfg(feature = "dev-tools")]
            scenario: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--headless" => options.headless = true,
                "--continue" => options.resume = true,
                "--new" => options.new = true,
                #[cfg(feature = "dev-tools")]
                "--scenario" => {
                    options.new = true;
                    options.scenario =
                        Some(args.next().ok_or("--scenario needs a name: multi-fleet, fleet-battle or fleet-battle:NvM")?);
                }
                #[cfg(feature = "dev-tools")]
                "--capture" => {
                    options.capture = Some(args.next().ok_or("--capture needs a PNG path")?.into())
                }
                #[cfg(feature = "dev-tools")]
                "--capture-size" => {
                    let size = args.next().ok_or("--capture-size needs WIDTHxHEIGHT, e.g. 390x844")?;
                    let (width, height) = size
                        .split_once('x')
                        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                        .filter(|(w, h): &(u32, u32)| (200..=4096).contains(w) && (200..=4096).contains(h))
                        .ok_or("--capture-size needs WIDTHxHEIGHT between 200 and 4096")?;
                    options.capture_size = (width, height);
                }
                #[cfg(feature = "dev-tools")]
                "--capture-view" => {
                    options.capture_view = args
                        .next()
                        .ok_or("--capture-view needs galaxy, system, market, fleets, saves, navigation, deep-space, entry or orbit")?
                }
                "--seed" => {
                    options.new = true;
                    options.seed_explicit = true;
                    options.seed = args
                        .next()
                        .ok_or("--seed needs a number")?
                        .parse()
                        .map_err(|_| "Invalid seed")?
                }
                "--systems" => {
                    options.new = true;
                    options.galaxy_explicit = true;
                    options.galaxy.systems = args
                        .next()
                        .ok_or("--systems needs a number from 4 to 150")?
                        .parse()
                        .map_err(|_| "Invalid system count")?;
                }
                "--radius" => {
                    options.new = true;
                    options.galaxy_explicit = true;
                    options.galaxy.radius_ly = args
                        .next()
                        .ok_or("--radius needs light-years from 20 to 95")?
                        .parse()
                        .map_err(|_| "Invalid galaxy radius")?;
                }
                "--objects" => {
                    options.new = true;
                    options.galaxy_explicit = true;
                    options.galaxy.max_objects = args
                        .next()
                        .ok_or("--objects needs a number from 6 to 15")?
                        .parse()
                        .map_err(|_| "Invalid system object limit")?;
                }
                "--save" => {
                    options.save_explicit = true;
                    options.save = args.next().ok_or("--save needs a file path")?.into();
                }
                "--economy" => {
                    options.new = true;
                    options.economy = Some(args.next().ok_or("--economy needs a JSON file")?.into())
                }
                _ => {
                    return Err(format!(
                        "Unknown option {argument}. Options: --headless, --continue, --new, --seed NUMBER, --systems 4..150, --radius 20..95, --objects 6..15, --save PATH, --economy PATH{DEV_OPTIONS}"
                    ));
                }
            }
        }
        if options.resume && options.new {
            return Err("--continue cannot be combined with new-map options".into());
        }
        options.galaxy.validate()?;
        Ok(options)
    }
}
