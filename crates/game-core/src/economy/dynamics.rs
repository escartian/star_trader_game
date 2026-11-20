//! Living markets and inflation. Each tick (`Economy::advance`):
//!
//! - **Activity:** every trade adds its value to its market's `activity`, which
//!   decays by [`ACTIVITY_DECAY_DIVISOR`]ths per tick (about 10% per run).
//! - **Size:** a market's economic size (percent of its founding size) steps one
//!   point per tick toward `idle_size_percent + activity / ACTIVITY_PER_POINT`,
//!   within [`MIN_SIZE_PERCENT`]–[`MAX_SIZE_PERCENT`]. Busy ports grow; ports
//!   nobody trades with (because raiders scare traffic off, or nothing pays)
//!   shrink toward the idle size.
//! - **Treasury:** drifts `treasury_drift_percent` of the gap per tick toward
//!   founding treasury × size × price level. The difference is off-screen trade,
//!   taxes and investment: the only way, besides bounties, that money enters or
//!   leaves the galaxy. Trades themselves still only move money.
//! - **Inflation:** a galaxy-wide price level (parts per million) compounds by
//!   `inflation_ppm_per_tick`. It scales goods, fuel, repairs, hold upgrades,
//!   hulls, charters and bounties alike, and the treasuries that pay for them.
//!
//! All integer arithmetic, so a run replays exactly.
use super::{Credits, Economy, MarketId};
use crate::fleet::ShipKind;
use serde::{Deserialize, Serialize};

/// The price level at the start of every game: 1.0 in parts per million.
pub const PRICE_LEVEL_UNIT: u64 = 1_000_000;
/// Inflation stops compounding at 100 times the starting prices.
pub const MAX_PRICE_LEVEL: u64 = 100 * PRICE_LEVEL_UNIT;
pub const MIN_SIZE_PERCENT: u32 = 50;
pub const MAX_SIZE_PERCENT: u32 = 400;
/// Activity (credits in cents of recent trade) per point of market size: a
/// steady full freighter hold each run (about 1,000 cr) keeps a port near 150%.
pub const ACTIVITY_PER_POINT: u64 = 10_000;
/// Activity loses 1/40 of itself each tick.
pub const ACTIVITY_DECAY_DIVISOR: u64 = 40;

/// Tuning for living markets and inflation, carried with the session rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketDynamics {
    /// Share of the gap to its target a treasury closes each tick (0 freezes
    /// treasuries, as before this existed).
    pub treasury_drift_percent: u32,
    /// Size a market settles at with no trade, in percent of its founding size.
    pub idle_size_percent: u32,
    /// Price-level growth per tick, in parts per million (0: no inflation).
    pub inflation_ppm_per_tick: u32,
}

impl Default for MarketDynamics {
    /// About 8% of the treasury gap per run, idle ports at 75% and roughly
    /// 5% inflation per 100 runs (four ticks per run).
    fn default() -> Self {
        Self {
            treasury_drift_percent: 2,
            idle_size_percent: 75,
            inflation_ppm_per_tick: 125,
        }
    }
}

impl MarketDynamics {
    /// No growth, drift or inflation: the static economy of earlier versions.
    pub fn frozen() -> Self {
        Self {
            treasury_drift_percent: 0,
            idle_size_percent: 100,
            inflation_ppm_per_tick: 0,
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.treasury_drift_percent <= 50
            && (MIN_SIZE_PERCENT..=MAX_SIZE_PERCENT).contains(&self.idle_size_percent)
            && self.inflation_ppm_per_tick <= 10_000
    }
}

fn scale(value: u64, numerator: u64, denominator: u64) -> u64 {
    let scaled = u128::from(value) * u128::from(numerator) / u128::from(denominator.max(1));
    scaled.min(u128::from(u64::MAX)) as u64
}

impl Economy {
    /// `base` at today's price level.
    pub fn inflate(&self, base: Credits) -> Credits {
        Credits(scale(base.0, self.price_level_ppm, PRICE_LEVEL_UNIT))
    }

    pub fn fuel_price(&self) -> Credits {
        self.inflate(self.rules.fuel_unit_price)
    }

    pub fn repair_price(&self) -> Credits {
        self.inflate(self.rules.repair_point_price)
    }

    pub fn hold_upgrade_price(&self) -> Credits {
        self.inflate(self.rules.hold_upgrade_price)
    }

    pub fn charter_price(&self) -> Credits {
        self.inflate(self.rules.campaign.charter_price)
    }

    /// A new hull's shipyard price today.
    pub fn ship_price(&self, kind: ShipKind) -> Option<Credits> {
        let spec = self.rules.ship_catalog.spec(kind)?;
        Some(self.inflate(spec.price))
    }

    /// What a shipyard pays for a used hull today.
    pub fn ship_resale(&self, kind: ShipKind) -> Option<Credits> {
        let price = self.ship_price(kind)?;
        Some(Credits(scale(
            price.0,
            u64::from(self.rules.ship_resale_percent),
            100,
        )))
    }

    /// Adds a trade worth `value` to its market's recent activity.
    pub(super) fn record_activity(&mut self, market: MarketId, value: Credits) {
        if let Ok(market) = self.market_mut(market) {
            market.activity = market.activity.saturating_add(value.0);
        }
    }

    /// One tick of market growth, treasury drift and inflation.
    pub(super) fn advance_dynamics(&mut self) {
        let dynamics = self.rules.dynamics.clone();
        let level = scale(
            self.price_level_ppm,
            PRICE_LEVEL_UNIT + u64::from(dynamics.inflation_ppm_per_tick),
            PRICE_LEVEL_UNIT,
        )
        .min(MAX_PRICE_LEVEL);
        self.price_level_ppm = level;
        for market in &mut self.markets {
            // Saves from before living markets: today's treasury is the founding one.
            if market.founding_credits.0 == 0 {
                market.founding_credits = market.credits;
            }
            market.activity -= market.activity / ACTIVITY_DECAY_DIVISOR;
            let wanted =
                u64::from(dynamics.idle_size_percent) + market.activity / ACTIVITY_PER_POINT;
            let wanted =
                wanted.clamp(u64::from(MIN_SIZE_PERCENT), u64::from(MAX_SIZE_PERCENT)) as u32;
            market.size_percent = match market.size_percent.cmp(&wanted) {
                std::cmp::Ordering::Less => market.size_percent + 1,
                std::cmp::Ordering::Greater => market.size_percent - 1,
                std::cmp::Ordering::Equal => market.size_percent,
            };
            let target = scale(
                scale(
                    market.founding_credits.0,
                    u64::from(market.size_percent),
                    100,
                ),
                level,
                PRICE_LEVEL_UNIT,
            );
            let current = market.credits.0;
            let gap = target.abs_diff(current);
            let step = scale(gap, u64::from(dynamics.treasury_drift_percent), 100);
            // Close at least a cent of a remaining gap, so treasuries settle.
            let step = if dynamics.treasury_drift_percent > 0 {
                step.max(gap.min(1))
            } else {
                0
            };
            market.credits = Credits(if target > current {
                current + step
            } else {
                current - step
            });
            for good in &mut market.goods {
                good.price_level_ppm = level;
            }
        }
    }

    /// Rejects markets outside the size range or out of step with the price level.
    pub fn validate_dynamics(&self) -> Result<(), String> {
        let level = self.price_level_ppm;
        let bad = !(PRICE_LEVEL_UNIT..=MAX_PRICE_LEVEL).contains(&level)
            || self.markets.iter().any(|market| {
                !(MIN_SIZE_PERCENT..=MAX_SIZE_PERCENT).contains(&market.size_percent)
                    || market
                        .goods
                        .iter()
                        .any(|good| good.price_level_ppm != level)
            });
        if bad {
            return Err("Invalid market size or price level".into());
        }
        Ok(())
    }
}
