use std::path::PathBuf;

pub struct Options {
    pub headless: bool,
    pub resume: bool,
    pub new: bool,
    pub capture: Option<PathBuf>,
    pub capture_view: String,
    pub seed: u64,
    pub save: PathBuf,
    pub save_explicit: bool,
    pub economy: Option<PathBuf>,
}

impl Options {
    pub fn parse() -> Result<Self, String> {
        let mut options = Self {
            headless: false,
            resume: false,
            new: false,
            capture: None,
            capture_view: "galaxy".into(),
            seed: 0,
            save: PathBuf::from("data/slot-1.json"),
            save_explicit: false,
            economy: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--headless" => options.headless = true,
                "--continue" => options.resume = true,
                "--new" => options.new = true,
                "--capture" => {
                    options.capture = Some(args.next().ok_or("--capture needs a PNG path")?.into())
                }
                "--capture-view" => {
                    options.capture_view = args
                        .next()
                        .ok_or("--capture-view needs galaxy, system, market or saves")?
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
                        "Unknown option {argument}. Options: --headless, --continue, --new, --seed NUMBER, --save PATH, --economy PATH, --capture PATH, --capture-view VIEW"
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
