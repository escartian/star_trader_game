//! Trade charters: a port authority's licence, bought once at a market.
//!
//! - **Port share:** the holder receives [`CampaignRules::port_share_percent`]
//!   of the value of every trade at a chartered port, NPC traders' included,
//!   paid out of the port's treasury (a transfer, so credits are conserved).
//! - **Shipping lanes:** two ports chartered by the same holder are joined by a
//!   patrolled lane. Trade routes between them are never struck by raiders.
//! - Three charters win the Trader road (see `campaign`).
//!
//! The charter price goes to the port's treasury.
use super::{Credits, Economy, EconomyError, MarketId};
use crate::travel::{PlayerId, Ship};
use serde::{Deserialize, Serialize};

/// Campaign tuning carried with the session rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRules {
    /// Share of all galaxy wealth (see `campaign::galaxy_wealth`) your net
    /// worth must reach to win the Trader road, in percent. A share rather
    /// than a sum, so the goal scales with galaxy size and economic change.
    #[serde(default = "default_wealth_share")]
    pub wealth_share_percent: u32,
    /// The fixed goal of the first version; read from older saves, ignored.
    #[serde(default, rename = "net_worth_goal", skip_serializing)]
    pub legacy_net_worth_goal: Option<Credits>,
    /// Price of one port charter (credits in cents).
    pub charter_price: Credits,
    /// Charters that win the Trader road.
    pub charters_to_win: u32,
    /// Share of each trade's value paid to a port's charter holder.
    pub port_share_percent: u32,
}

impl Default for CampaignRules {
    fn default() -> Self {
        Self {
            wealth_share_percent: default_wealth_share(),
            legacy_net_worth_goal: None,
            charter_price: Credits(500_000),
            charters_to_win: 3,
            port_share_percent: 2,
        }
    }
}

fn default_wealth_share() -> u32 {
    10
}

impl CampaignRules {
    pub(super) fn valid(&self) -> bool {
        (1..=100).contains(&self.wealth_share_percent)
            && (1..=1_000_000_000_000).contains(&self.charter_price.0)
            && (1..=64).contains(&self.charters_to_win)
            && self.port_share_percent <= 20
    }
}

/// A port charter and the player who holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charter {
    pub market: MarketId,
    pub owner: PlayerId,
}

impl Economy {
    /// The holder of `market`'s charter, if anyone holds it.
    pub fn charter_holder(&self, market: MarketId) -> Option<PlayerId> {
        self.charters
            .iter()
            .find(|charter| charter.market == market)
            .map(|charter| charter.owner)
    }

    /// Markets chartered by `owner`, in purchase order.
    pub fn chartered_by(&self, owner: PlayerId) -> Vec<MarketId> {
        self.charters
            .iter()
            .filter(|charter| charter.owner == owner)
            .map(|charter| charter.market)
            .collect()
    }

    /// The price of the charter of the market where `ship` is, if one is for sale.
    pub fn quote_charter(&self, ship: &Ship) -> Result<Credits, EconomyError> {
        let market = self.current_market(ship)?;
        if self.charter_holder(market.id).is_some() {
            return Err(EconomyError::AlreadyChartered);
        }
        let price = self.charter_price();
        if self.credits < price {
            return Err(EconomyError::InsufficientCredits);
        }
        Ok(price)
    }

    /// Buys the charter of the market where `ship` is, for its owner.
    pub fn buy_charter(&mut self, ship: &Ship) -> Result<Credits, EconomyError> {
        let price = self.quote_charter(ship)?;
        let id = self.current_market(ship)?.id;
        let market = self.market_mut(id)?;
        market.credits = Credits(
            market
                .credits
                .0
                .checked_add(price.0)
                .ok_or(EconomyError::ArithmeticOverflow)?,
        );
        self.credits = Credits(self.credits.0 - price.0);
        self.charters.push(Charter {
            market: id,
            owner: ship.owner,
        });
        Ok(price)
    }

    /// Pays the port share of a trade worth `value` at `market` into the
    /// current wallet, from the port's treasury (never more than it holds).
    /// Callers make sure the current wallet is the charter holder's.
    pub(crate) fn pay_port_share(&mut self, market: MarketId, value: Credits) -> Credits {
        let percent = u64::from(self.rules.campaign.port_share_percent);
        let Ok(port) = self.market_mut(market) else {
            return Credits(0);
        };
        let share = (value.0.saturating_mul(percent) / 100).min(port.credits.0);
        port.credits = Credits(port.credits.0 - share);
        self.credits = Credits(self.credits.0.saturating_add(share));
        Credits(share)
    }

    /// Whether a route between `stops` runs on `owner`'s shipping lane: both
    /// ports chartered by `owner`.
    pub fn on_lane(&self, owner: PlayerId, stops: [MarketId; 2]) -> bool {
        stops
            .iter()
            .all(|stop| self.charter_holder(*stop) == Some(owner))
    }

    /// Rejects duplicate charters and charters of unknown markets.
    pub fn validate_charters(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for charter in &self.charters {
            if !seen.insert(charter.market) || self.market(charter.market).is_err() {
                return Err("Invalid trade charter".into());
            }
        }
        Ok(())
    }
}
