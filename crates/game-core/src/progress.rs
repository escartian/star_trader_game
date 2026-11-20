//! Progressive disclosure. Every rule works from the first run; the client
//! shows a feature's controls once it becomes relevant, each with a one-line
//! guide card. The host records unlocks here as play reaches them:
//!
//! 1. **Captain** (always): market, refuel, plot a course, Advance.
//! 2. [`Feature::Fleets`]: [`FLEETS_PROFIT`] of profit (net worth gained since
//!    the start, at today's prices): shipyard, more ships, fleets.
//! 3. [`Feature::TradeRoutes`]: two fleets, or [`ROUTE_DELIVERIES`] sales.
//! 4. [`Feature::Battles`]: the first raider contact.
//! 5. [`Feature::Landing`]: the first orbit of a moon or asteroid.
//! 6. [`Feature::Victory`]: the first bounty, or net worth at half the Trader
//!    goal's share of galaxy wealth: the Victory panel, charters, the Haven.
use crate::{
    BodyKind, StarSystem, campaign,
    economy::{Credits, Economy},
    navigation::EncounterKind,
    travel::{Location, PlayerId, TravelState},
};
use serde::{Deserialize, Serialize};

/// Profit (credits in cents, at a new game's prices) that opens the shipyard:
/// 1,000 cr, doubling the default starting money. (The plan proposed 2,000 cr,
/// but in the small default economy that already reaches half the Trader goal,
/// which would open the Victory panel at the same moment.)
pub const FLEETS_PROFIT: Credits = Credits(100_000);
/// Sales that open trade routes without a second fleet.
pub const ROUTE_DELIVERIES: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Fleets,
    TradeRoutes,
    Battles,
    Landing,
    Victory,
}

impl Feature {
    pub const ALL: [Feature; 5] = [
        Feature::Fleets,
        Feature::TradeRoutes,
        Feature::Battles,
        Feature::Landing,
        Feature::Victory,
    ];

    /// The guide card's title and one line.
    pub fn guide(self) -> (&'static str, &'static str) {
        match self {
            Feature::Fleets => (
                "Shipyard and fleets",
                "Profits open the shipyard: buy hulls in Fleets & shipyard (Menu), split them into fleets and give the others standing orders.",
            ),
            Feature::TradeRoutes => (
                "Trade routes",
                "At a market, pick another market on the map and choose TRADE ROUTE: the fleet loops between them, buying and selling on its own.",
            ),
            Feature::Battles => (
                "Raider battles",
                "Auto-resolve lets the AI fight it out. Fight opens the board: tap a green hex to move, a red-ringed raider to aim, then Fire and End turn.",
            ),
            Feature::Landing => (
                "Landing and mining",
                "In orbit of a moon, asteroid or rocky planet, Land on the fleet's card; a landed fleet that waits mines ore each run.",
            ),
            Feature::Victory => (
                "Roads to victory",
                "Victory (Menu) shows every road: buy port charters, grow your share of galaxy wealth, or destroy the Raider Haven.",
            ),
        }
    }
}

/// Saved progress through the unlocks (part of the player state).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub unlocked: Vec<Feature>,
    /// Sales made by the player (counted by the host).
    #[serde(default)]
    pub deliveries: u32,
    /// Net worth when the game began, the baseline for profit.
    #[serde(default)]
    pub start_net_worth: Credits,
}

impl Default for Progress {
    /// Everything unlocked: saves from before unlocks, tests and captures.
    fn default() -> Self {
        Self {
            unlocked: Feature::ALL.to_vec(),
            deliveries: 0,
            start_net_worth: Credits(0),
        }
    }
}

impl Progress {
    /// A new captain: nothing beyond trading and travel yet.
    pub fn captain(start_net_worth: Credits) -> Self {
        Self {
            unlocked: Vec::new(),
            deliveries: 0,
            start_net_worth,
        }
    }

    pub fn has(&self, feature: Feature) -> bool {
        self.unlocked.contains(&feature)
    }
}

/// Features this state now qualifies for that are not yet unlocked.
pub fn newly_unlocked(
    state: &TravelState,
    economy: &Economy,
    systems: &[StarSystem],
    player: PlayerId,
) -> Vec<Feature> {
    let progress = &state.progress;
    let owned = || {
        state
            .fleets
            .iter()
            .filter(move |fleet| fleet.owner == player)
    };
    let worth = campaign::net_worth(state, economy, player);
    let profit = worth.0.saturating_sub(progress.start_net_worth.0);
    let small_body_orbit = state
        .ships
        .iter()
        .filter(|ship| ship.owner == player)
        .any(|ship| {
            let (Location::Orbiting { system, body } | Location::Landed { system, body }) =
                ship.location
            else {
                return false;
            };
            systems
                .iter()
                .find(|known| known.id == system)
                .and_then(|known| known.bodies.iter().find(|known| known.id == body))
                .is_some_and(|body| matches!(body.kind, BodyKind::Moon | BodyKind::Asteroid))
        });
    let raiders_met = state.navigation.encounters.iter().any(|encounter| {
        encounter.kind == EncounterKind::Raiders && owned().any(|fleet| fleet.id == encounter.fleet)
    }) || state.navigation.last_combat.is_some();
    let bounty = state
        .navigation
        .last_combat
        .as_ref()
        .is_some_and(|report| report.bounty.0 > 0);
    let half_goal = u64::from(economy.rules.campaign.wealth_share_percent) * 50;
    let wealthy = campaign::wealth_share_basis_points(state, economy, player) >= half_goal;
    let reached = |feature| match feature {
        Feature::Fleets => profit >= economy.inflate(FLEETS_PROFIT).0,
        Feature::TradeRoutes => owned().count() >= 2 || progress.deliveries >= ROUTE_DELIVERIES,
        Feature::Battles => raiders_met,
        Feature::Landing => small_body_orbit,
        Feature::Victory => bounty || wealthy,
    };
    Feature::ALL
        .into_iter()
        .filter(|feature| !progress.has(*feature) && reached(*feature))
        .collect()
}

#[cfg(test)]
mod tests;
