//! Living markets and inflation. A market's future depends only on its current
//! state and its recent past; nothing remembers how it was founded. Each tick
//! (`Economy::advance`):
//!
//! - **Activity:** every trade adds its value to its market's `activity`, which
//!   fades by 1/[`ACTIVITY_DECAY_DIVISOR`] per tick (about 10% per run): the
//!   market's recent past.
//! - **Income:** `local_income_percent` of the value of the port's own
//!   production and consumption at today's prices (its local industry), plus
//!   `trade_income_ppm` of its recent activity (business from trade).
//! - **Upkeep:** `upkeep_ppm_per_tick` of the current treasury.
//!
//! Treasuries therefore settle where income meets upkeep: a port that trades
//! more grows richer, a quiet one falls back to what its own industry supports,
//! and a port whose trade picks up or dies away moves from wherever it is now.
//! Income and upkeep are off-screen trade, taxes and spending: with bounties,
//! the only way money enters or leaves the galaxy. Trades still only move money.
//!
//! - **Inflation:** the galaxy's price level (parts per million) compounds from
//!   its current value by `inflation_ppm_per_tick`. Goods, fuel, repairs, hold
//!   upgrades, hulls, charters and bounties all scale with it.
//!
//! All integer arithmetic, so a run replays exactly.
use super::{Credits, Economy, Market, MarketId};
use crate::fleet::ShipKind;
use serde::{Deserialize, Serialize};

/// The price level of a new game: 1.0 in parts per million.
pub const PRICE_LEVEL_UNIT: u64 = 1_000_000;
/// Inflation stops compounding at 100 times a new game's prices.
pub const MAX_PRICE_LEVEL: u64 = 100 * PRICE_LEVEL_UNIT;
/// Activity loses 1/40 of itself each tick.
pub const ACTIVITY_DECAY_DIVISOR: u64 = 40;
const PER_MILLION: u64 = 1_000_000;

/// Tuning for living markets and inflation, carried with the session rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketDynamics {
    /// Share of the value of a port's own output (production plus
    /// consumption, at today's list prices) it earns per tick, in percent.
    pub local_income_percent: u32,
    /// Share of a port's recent trade activity it earns per tick, in ppm.
    pub trade_income_ppm: u32,
    /// Share of its treasury a port spends per tick, in ppm.
    pub upkeep_ppm_per_tick: u32,
    /// Price-level growth per tick, in ppm (0: no inflation).
    pub inflation_ppm_per_tick: u32,
}

impl Default for MarketDynamics {
    /// Upkeep of 0.5% per tick; local industry and trade income sized so a
    /// port of the default template settles near 4,600 cr with no trade and
    /// gains roughly half again with a full freighter hold traded each run;
    /// prices rise about 5% per 100 runs (four ticks per run).
    fn default() -> Self {
        Self {
            local_income_percent: 4,
            trade_income_ppm: 1_500,
            upkeep_ppm_per_tick: 5_000,
            inflation_ppm_per_tick: 125,
        }
    }
}

impl MarketDynamics {
    /// No income, upkeep or inflation: the static economy of earlier versions.
    pub fn frozen() -> Self {
        Self {
            local_income_percent: 0,
            trade_income_ppm: 0,
            upkeep_ppm_per_tick: 0,
            inflation_ppm_per_tick: 0,
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.local_income_percent <= 100
            && self.trade_income_ppm <= 100_000
            && self.upkeep_ppm_per_tick <= 100_000
            && self.inflation_ppm_per_tick <= 10_000
    }
}

/// One tick of a market's money: what it earns and what it spends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarketFlow {
    /// From its own industry (credits in cents per tick).
    pub local_income: Credits,
    /// From recent trade (credits in cents per tick).
    pub trade_income: Credits,
    pub upkeep: Credits,
}

impl MarketFlow {
    pub fn income(self) -> Credits {
        Credits(self.local_income.0.saturating_add(self.trade_income.0))
    }

    /// Positive while the treasury grows.
    pub fn net(self) -> i128 {
        i128::from(self.income().0) - i128::from(self.upkeep.0)
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

    /// What `market` earns and spends this tick, from its current state.
    pub fn market_flow(&self, market: &Market) -> MarketFlow {
        let dynamics = &self.rules.dynamics;
        let output = market.goods.iter().fold(0_u64, |sum, good| {
            let units = u64::from(good.production) + u64::from(good.consumption);
            let price = scale(good.base_price.0, good.price_level_ppm, PRICE_LEVEL_UNIT);
            sum.saturating_add(units.saturating_mul(price))
        });
        MarketFlow {
            local_income: Credits(scale(output, u64::from(dynamics.local_income_percent), 100)),
            trade_income: Credits(scale(
                market.activity,
                u64::from(dynamics.trade_income_ppm),
                PER_MILLION,
            )),
            upkeep: Credits(scale(
                market.credits.0,
                u64::from(dynamics.upkeep_ppm_per_tick),
                PER_MILLION,
            )),
        }
    }

    /// Adds a trade worth `value` to its market's recent activity.
    pub(super) fn record_activity(&mut self, market: MarketId, value: Credits) {
        if let Ok(market) = self.market_mut(market) {
            market.activity = market.activity.saturating_add(value.0);
        }
    }

    /// One tick of inflation, market income and upkeep, and fading activity.
    pub(super) fn advance_dynamics(&mut self) {
        let level = scale(
            self.price_level_ppm,
            PRICE_LEVEL_UNIT + u64::from(self.rules.dynamics.inflation_ppm_per_tick),
            PRICE_LEVEL_UNIT,
        )
        .min(MAX_PRICE_LEVEL);
        self.price_level_ppm = level;
        for index in 0..self.markets.len() {
            for good in &mut self.markets[index].goods {
                good.price_level_ppm = level;
            }
            let flow = self.market_flow(&self.markets[index]);
            let market = &mut self.markets[index];
            market.credits = Credits(
                market
                    .credits
                    .0
                    .saturating_add(flow.income().0)
                    .saturating_sub(flow.upkeep.0),
            );
            // At least a cent, so a market with no trade reaches zero activity.
            let fade = (market.activity / ACTIVITY_DECAY_DIVISOR).max(market.activity.min(1));
            market.activity -= fade;
        }
    }

    /// Rejects goods out of step with the galaxy's price level.
    pub fn validate_dynamics(&self) -> Result<(), String> {
        let level = self.price_level_ppm;
        let bad = !(PRICE_LEVEL_UNIT..=MAX_PRICE_LEVEL).contains(&level)
            || self.markets.iter().any(|market| {
                market
                    .goods
                    .iter()
                    .any(|good| good.price_level_ppm != level)
            });
        if bad {
            return Err("Invalid market price level".into());
        }
        Ok(())
    }
}
