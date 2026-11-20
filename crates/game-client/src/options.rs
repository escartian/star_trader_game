use std::path::PathBuf;

/// Extra flags listed in help only when the developer tooling is compiled in.
#[cfg(feature = "dev-tools")]
const DEV_OPTIONS: &str =
    ", --capture PATH, --capture-view VIEW, --scenario multi-fleet|fleet-battle";
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
    pub seed: u64,
    pub save: PathBuf,
    pub save_explicit: bool,
    pub economy: Option<PathBuf>,
    /// Named test scenario to build and save before starting ("multi-fleet" or "fleet-battle").
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
            seed: 0,
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
                        Some(args.next().ok_or("--scenario needs a name: multi-fleet or fleet-battle")?);
                }
                #[cfg(feature = "dev-tools")]
                "--capture" => {
                    options.capture = Some(args.next().ok_or("--capture needs a PNG path")?.into())
                }
                #[cfg(feature = "dev-tools")]
                "--capture-view" => {
                    options.capture_view = args
                        .next()
                        .ok_or("--capture-view needs galaxy, system, market, fleets, saves, navigation, deep-space, entry or orbit")?
                }
                "--seed" => {
                    options.new = true;
                    options.seed = args
                        .next()
                        .ok_or("--seed needs a number")?
                        .parse()
                        .map_err(|_| "Invalid seed")?
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
                        "Unknown option {argument}. Options: --headless, --continue, --new, --seed NUMBER, --save PATH, --economy PATH{DEV_OPTIONS}"
                    ));
                }
            }
        }
        if options.resume && options.new {
            return Err("--continue cannot be combined with --new, --seed or --economy".into());
        }
        Ok(options)
    }
}
