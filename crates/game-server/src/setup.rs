//! New-game setup: galaxy size, starting credits, inflation, danger and the
//! campaign's length and Trader goal. Every choice becomes session rules at
//! creation, so the save carries it and the game plays by it throughout.
use super::Session;
use game_core::{
    economy::{Credits, EconomyRules},
    generation::GalaxyConfig,
    movement::MovementRules,
};

/// How fast prices rise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InflationLevel {
    None,
    Low,
    #[default]
    Standard,
    High,
}

impl InflationLevel {
    pub const ALL: [Self; 4] = [Self::None, Self::Low, Self::Standard, Self::High];

    /// Price-level growth per tick, in parts per million.
    pub fn ppm_per_tick(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Low => 60,
            Self::Standard => 125,
            Self::High => 250,
        }
    }

    /// Roughly how much prices rise per 100 runs (four ticks per run).
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Low => "Low (≈2.5% / 100 runs)",
            Self::Standard => "Standard (≈5% / 100 runs)",
            Self::High => "High (≈10% / 100 runs)",
        }
    }
}

/// How often contacts happen and how strong raiders are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DangerLevel {
    Calm,
    #[default]
    Standard,
    Perilous,
}

impl DangerLevel {
    pub const ALL: [Self; 3] = [Self::Calm, Self::Standard, Self::Perilous];

    /// (contact chance per run %, NPC raid chance per run %, extra band scouts)
    fn settings(self) -> (u32, u32, u32) {
        match self {
            Self::Calm => (6, 2, 0),
            Self::Standard => (12, 4, 0),
            Self::Perilous => (20, 8, 1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "Calm: fewer contacts and raids",
            Self::Standard => "Standard",
            Self::Perilous => "Perilous: more contacts, bigger raider bands",
        }
    }
}

/// Everything a new game can be set up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameSetup {
    pub galaxy: GalaxyConfig,
    /// Credits in cents.
    pub starting_credits: Credits,
    pub inflation: InflationLevel,
    pub danger: DangerLevel,
    /// End on score at this run; `None` is open-ended.
    pub run_limit: Option<u64>,
    /// Share of galaxy wealth that wins the Trader road, in percent.
    pub wealth_share_percent: u32,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            galaxy: GalaxyConfig::default(),
            starting_credits: Credits(100_000),
            inflation: InflationLevel::default(),
            danger: DangerLevel::default(),
            run_limit: None,
            wealth_share_percent: 10,
        }
    }
}

/// Starting-credit choices offered by the setup screen (credits in cents).
pub const STARTING_CREDITS: [u64; 4] = [50_000, 100_000, 250_000, 500_000];
/// Trader goal choices, in percent of galaxy wealth.
pub const WEALTH_SHARES: [u32; 4] = [5, 10, 15, 25];

impl GameSetup {
    /// The session rules this setup implies, from base economy and movement rules.
    pub fn apply(&self, rules: &mut EconomyRules, movement: &mut MovementRules) {
        rules.starting_credits = self.starting_credits;
        rules.dynamics.inflation_ppm_per_tick = self.inflation.ppm_per_tick();
        rules.campaign.wealth_share_percent = self.wealth_share_percent;
        let (contacts, raids, extra) = self.danger.settings();
        rules.npc_raid_chance_percent = raids;
        movement.encounter_chance_percent = contacts;
        movement.raider_band_extra = extra;
    }
}

impl Session {
    /// A new game from `setup`, on top of base `rules` and `movement` rules.
    pub fn new_game(
        seed: u64,
        mut rules: EconomyRules,
        mut movement: MovementRules,
        setup: &GameSetup,
    ) -> Result<Self, String> {
        setup.apply(&mut rules, &mut movement);
        rules.validate()?;
        movement.validate().map_err(|error| error.to_string())?;
        if setup
            .run_limit
            .is_some_and(|limit| limit == 0 || limit > super::campaign::MAX_RUN_LIMIT)
        {
            return Err("Campaign length must be 1–100,000 runs".into());
        }
        let systems = game_core::generation::generate_galaxy_with(seed, setup.galaxy)?;
        let mut session = Self::from_generated(seed, rules, systems, movement)?;
        session.state.campaign.run_limit = setup.run_limit;
        session.begin_progression();
        Ok(session)
    }
}
