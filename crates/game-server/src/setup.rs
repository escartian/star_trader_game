//! New-game setup: galaxy size, starting credits, inflation, danger, the
//! campaign's length and Trader goal, and the factions in play. Every choice becomes session rules at
//! creation, so the save carries it and the game plays by it throughout.
use super::Session;
use game_core::{
    economy::{Credits, EconomyRules, custom_inflation_ppb},
    factions::FactionSetup,
    generation::GalaxyConfig,
    movement::MovementRules,
    species::Species,
};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSetup {
    /// The player captain's name as typed; cleaned when the game starts.
    pub captain_name: String,
    pub species: Species,
    pub galaxy: GalaxyConfig,
    /// Credits in cents.
    pub starting_credits: Credits,
    /// Inflation per 100 navigation runs, in hundredths of a percent.
    pub inflation_basis_points: u32,
    pub danger: DangerLevel,
    /// End on score at this run; `None` is open-ended.
    pub run_limit: Option<u64>,
    /// Share of galaxy wealth that wins the Trader road, in percent.
    pub wealth_share_percent: u32,
    /// Factions in play, your allegiance, territory and relations.
    pub factions: FactionSetup,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            captain_name: String::new(),
            species: Species::Human,
            galaxy: GalaxyConfig::default(),
            starting_credits: Credits(100_000),
            inflation_basis_points: 0,
            danger: DangerLevel::default(),
            run_limit: None,
            wealth_share_percent: 10,
            factions: FactionSetup::default(),
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
        rules.dynamics.inflation_ppm_per_tick = 0;
        rules.dynamics.inflation_basis_points_per_100_runs = self.inflation_basis_points;
        rules.dynamics.inflation_ticks_per_100_runs =
            u32::try_from(movement.ticks_per_navigation_run)
                .unwrap_or(u32::MAX)
                .saturating_mul(100);
        rules.dynamics.inflation_ppb_per_tick = custom_inflation_ppb(
            self.inflation_basis_points,
            rules.dynamics.inflation_ticks_per_100_runs,
        );
        rules.campaign.wealth_share_percent = self.wealth_share_percent;
        let (contacts, raids, extra) = self.danger.settings();
        rules.npc_raid_chance_percent = raids;
        movement.encounter_chance_percent = contacts;
        movement.raider_band_extra = extra;
    }
}

impl Session {
    /// Lays out territory, relations and opening standing, enlists each NPC
    /// trader with the holder of its home port, and places a police patrol in
    /// every claimed system; a world generated without a setup
    /// (tests, captures) has no factions until this runs.
    pub fn set_up_factions(&mut self, seed: u64, setup: &FactionSetup) -> Result<(), String> {
        let catalog = &self.economy.rules.faction_catalog;
        self.economy.factions = setup.generate(seed, &self.systems, catalog)?;
        self.economy.affiliate_traders();
        self.state
            .spawn_police(&self.economy.factions, &self.systems);
        Ok(())
    }

    /// A new game from `setup`, on top of base `rules` and `movement` rules.
    pub fn new_game(
        seed: u64,
        mut rules: EconomyRules,
        mut movement: MovementRules,
        setup: &GameSetup,
    ) -> Result<Self, String> {
        setup.apply(&mut rules, &mut movement);
        if setup.inflation_basis_points > 10_000 {
            return Err("Inflation must be 0.00–100.00% per 100 runs".into());
        }
        rules.validate()?;
        setup.factions.validate()?;
        movement.validate().map_err(|error| error.to_string())?;
        if setup
            .run_limit
            .is_some_and(|limit| limit == 0 || limit > super::campaign::MAX_RUN_LIMIT)
        {
            return Err("Campaign length must be 1–100,000 runs".into());
        }
        let systems = game_core::generation::generate_galaxy_with(seed, setup.galaxy)?;
        let mut session = Self::from_generated(seed, rules, systems, movement)?;
        session.state.player_species = setup.species;
        session.state.captain_name = game_core::travel::clean_captain_name(&setup.captain_name);
        session.state.fleets[0].captain.species = setup.species;
        session.state.campaign.run_limit = setup.run_limit;
        session.set_up_factions(seed, &setup.factions)?;
        session.begin_progression();
        Ok(session)
    }
}
