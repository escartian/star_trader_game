//! The campaign: roads to victory, the score, an optional run limit and how
//! the game ends. Pure functions over the player and economy state; the
//! server records the end once (see `Campaign::ended`).
//!
//! - **Trader:** net worth of [`CampaignRules::net_worth_goal`], or
//!   [`CampaignRules::charters_to_win`] port charters.
//! - **Soldier:** destroy the Raider Haven (`navigation::haven`).
//! - **Diplomat / Spy** and **Pirate:** need factions and missions; shown as
//!   locked until those exist.
//! - **Lost:** the player's last ship was destroyed.
//! - **Time up:** with a run limit, the game ends on score at that run.
//!
//! Net worth = credits + each owned hull at its resale price + cargo at the
//! best price any market currently pays.
//!
//! Score = net worth / 100 cr, plus [`CHARTER_POINTS`] per charter, plus
//! [`HAVEN_SHIP_POINTS`] per Haven garrison ship destroyed, plus
//! [`HAVEN_POINTS`] for destroying the Haven. Faction standing will add points
//! once factions exist.
//!
//! [`CampaignRules::net_worth_goal`]: crate::economy::CampaignRules
//! [`CampaignRules::charters_to_win`]: crate::economy::CampaignRules
use crate::economy::{Commodity, Credits, Economy, TradeSide};
use crate::travel::{PlayerId, TravelState};
use serde::{Deserialize, Serialize};

/// Score points per port charter held.
pub const CHARTER_POINTS: u64 = 100;
/// Score points per Haven garrison ship destroyed.
pub const HAVEN_SHIP_POINTS: u64 = 20;
/// Score points for destroying the Raider Haven.
pub const HAVEN_POINTS: u64 = 500;
/// Credits (in cents) per score point of net worth: 100 cr.
pub const NET_WORTH_PER_POINT: u64 = 10_000;
/// Run limits the Victory panel offers (0 = none).
pub const RUN_LIMIT_CHOICES: [u64; 4] = [0, 200, 400, 800];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Road {
    Trader,
    Soldier,
    Diplomat,
    Pirate,
}

impl Road {
    pub const ALL: [Road; 4] = [Road::Trader, Road::Soldier, Road::Diplomat, Road::Pirate];

    pub fn label(self) -> &'static str {
        match self {
            Road::Trader => "Trader",
            Road::Soldier => "Soldier",
            Road::Diplomat => "Diplomat / Spy",
            Road::Pirate => "Pirate",
        }
    }
}

/// How the campaign ended, and on which run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CampaignEnd {
    /// The first road completed. Play may continue afterwards.
    Won { road: Road, run: u64 },
    /// The last ship was destroyed. Nothing more can be done.
    Lost { run: u64 },
    /// The run limit was reached; the score stands.
    TimeUp { run: u64 },
}

impl CampaignEnd {
    /// Whether the game still accepts orders (only after a win).
    pub fn playable(self) -> bool {
        matches!(self, CampaignEnd::Won { .. })
    }

    /// One line for logs and the end screen.
    pub fn label(self) -> String {
        match self {
            CampaignEnd::Won { road, run } => {
                format!("Victory on the {} road, run {run}.", road.label())
            }
            CampaignEnd::Lost { run } => format!("Your last ship was lost on run {run}."),
            CampaignEnd::TimeUp { run } => format!("The campaign ended on score at run {run}."),
        }
    }
}

/// Saved campaign settings and outcome (part of the player state).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    /// End the game on score at this run; `None` plays open-ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<CampaignEnd>,
}

/// Progress on one road, for the Victory panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoadProgress {
    pub road: Road,
    /// Whether the road can be played yet.
    pub open: bool,
    /// 0–100.
    pub percent: u32,
    pub complete: bool,
    /// What the road needs, and how far along it is.
    pub detail: String,
}

/// The score and its parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Score {
    pub net_worth: u64,
    pub charters: u64,
    pub haven: u64,
}

impl Score {
    pub fn total(self) -> u64 {
        self.net_worth + self.charters + self.haven
    }
}

/// Credits, hulls at resale and cargo at the best current bid.
pub fn net_worth(state: &TravelState, economy: &Economy, player: PlayerId) -> Credits {
    let resale = u64::from(economy.rules.ship_resale_percent);
    let hull = |kind| {
        economy
            .rules
            .ship_catalog
            .spec(kind)
            .map_or(0, |spec| spec.price.0.saturating_mul(resale) / 100)
    };
    let best_bid = |good: Commodity| {
        economy
            .markets
            .iter()
            .map(|market| {
                let offer = &market.goods[good.index()];
                offer.price(TradeSide::Sell, offer.stock).0
            })
            .max()
            .unwrap_or(0)
    };
    let bids = Commodity::ALL.map(best_bid);
    let owned = state.ships.iter().filter(|ship| ship.owner == player);
    let assets = owned.fold(0_u64, |total, ship| {
        let cargo = ship
            .cargo
            .iter()
            .zip(&bids)
            .fold(0_u64, |sum, (units, bid)| {
                sum.saturating_add(u64::from(*units).saturating_mul(*bid))
            });
        total.saturating_add(hull(ship.kind)).saturating_add(cargo)
    });
    Credits(economy.credits.0.saturating_add(assets))
}

pub fn score(state: &TravelState, economy: &Economy, player: PlayerId) -> Score {
    let charters = economy.chartered_by(player).len() as u64;
    let haven = state.navigation.haven.as_ref().map_or(0, |haven| {
        u64::from(haven.sunk) * HAVEN_SHIP_POINTS + if haven.standing() { 0 } else { HAVEN_POINTS }
    });
    Score {
        net_worth: net_worth(state, economy, player).0 / NET_WORTH_PER_POINT,
        charters: charters * CHARTER_POINTS,
        haven,
    }
}

/// Progress on every road, in [`Road::ALL`] order.
pub fn progress(state: &TravelState, economy: &Economy, player: PlayerId) -> Vec<RoadProgress> {
    let goals = &economy.rules.campaign;
    let worth = net_worth(state, economy, player);
    let charters = economy.chartered_by(player).len() as u64;
    let to_win = u64::from(goals.charters_to_win);
    let share = |have: u64, need: u64| (have.saturating_mul(100) / need.max(1)).min(100) as u32;
    let trader_complete = worth >= goals.net_worth_goal || charters >= to_win;
    let haven = state.navigation.haven.as_ref();
    let haven_complete = haven.is_some_and(|haven| !haven.standing());
    let garrison = haven.map_or((0, 0), |haven| {
        let left = haven.garrison.len() as u64;
        (u64::from(haven.sunk), left + u64::from(haven.sunk))
    });
    let locked = |road| RoadProgress {
        road,
        open: false,
        percent: 0,
        complete: false,
        detail: "Opens with factions and missions.".into(),
    };
    vec![
        RoadProgress {
            road: Road::Trader,
            open: true,
            percent: share(worth.0, goals.net_worth_goal.0).max(share(charters, to_win)),
            complete: trader_complete,
            detail: format!(
                "Net worth {worth} of {} cr, or {charters} of {to_win} port charters.",
                goals.net_worth_goal
            ),
        },
        RoadProgress {
            road: Road::Soldier,
            open: haven.is_some(),
            percent: if haven_complete {
                100
            } else {
                share(garrison.0, garrison.1)
            },
            complete: haven_complete,
            detail: match haven {
                Some(_) if haven_complete => "The Raider Haven is destroyed.".into(),
                Some(_) => format!(
                    "Destroy the Raider Haven: {} of {} garrison ships sunk.",
                    garrison.0, garrison.1
                ),
                None => "No Raider Haven in this galaxy.".into(),
            },
        },
        locked(Road::Diplomat),
        locked(Road::Pirate),
    ]
}

/// The end this state has reached, if any and not yet recorded: a completed
/// road, or the run limit. Defeat is recorded when the last ship is lost.
pub fn check_end(state: &TravelState, economy: &Economy, player: PlayerId) -> Option<CampaignEnd> {
    if state.campaign.ended.is_some() {
        return None;
    }
    let run = state.navigation.run;
    if let Some(road) = progress(state, economy, player)
        .into_iter()
        .find(|road| road.complete)
    {
        return Some(CampaignEnd::Won {
            road: road.road,
            run,
        });
    }
    state
        .campaign
        .run_limit
        .filter(|limit| run >= *limit)
        .map(|_| CampaignEnd::TimeUp { run })
}

#[cfg(test)]
mod tests;
