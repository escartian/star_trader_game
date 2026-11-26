//! NPC traders arm themselves from their profits (`combat::gear`).
//!
//! At a market, once repaired and refuelled, a trader whose wallet holds more
//! than [`RESERVE_MULTIPLE`] × its starting credits fits afterburners, then
//! point defense, one item per stop. With more than [`ESCORT_MULTIPLE`] ×
//! its starting credits and no escort, it hires one for `ESCORT_RUNS` runs.
//! Payments go to the market's treasury. So traders that do well grow harder
//! to rob as the game goes on.
use super::Trader;
use crate::{
    combat::gear::{ESCORT_RUNS, Gear},
    economy::Economy,
};

/// Wallet, in multiples of starting credits, above which a trader buys gear.
pub const RESERVE_MULTIPLE: u64 = 2;
/// Wallet, in multiples of starting credits, above which it hires an escort.
pub const ESCORT_MULTIPLE: u64 = 3;

impl Economy {
    /// Buys at most one gear item, and perhaps an escort, for `trader` at its
    /// current market. Returns a news line for what it bought.
    pub(super) fn outfit(&mut self, trader: &mut Trader) -> Option<String> {
        let start = self.rules.npc_starting_credits.0;
        let mut news = Vec::new();
        let wanted = Gear::NPC
            .into_iter()
            .find(|gear| !trader.state.ships[0].gear.contains(gear));
        if let Some(gear) = wanted
            && trader.credits.0 >= start.saturating_mul(RESERVE_MULTIPLE) + self.gear_price(gear).0
        {
            let wallet = std::mem::replace(&mut self.credits, trader.credits);
            let bought = self.fit_gear(&mut trader.state.ships[0], gear).is_ok();
            trader.credits = std::mem::replace(&mut self.credits, wallet);
            if bought {
                news.push(gear.label().to_lowercase());
            }
        }
        if trader.escort_runs == 0
            && trader.credits.0 >= start.saturating_mul(ESCORT_MULTIPLE) + self.escort_price().0
        {
            let wallet = std::mem::replace(&mut self.credits, trader.credits);
            let hired = self.hire_escort(&trader.state.ships[0]).is_ok();
            trader.credits = std::mem::replace(&mut self.credits, wallet);
            if hired {
                trader.escort_runs = ESCORT_RUNS;
                news.push("an escort".into());
            }
        }
        (!news.is_empty()).then(|| format!("Bought {} against pirates", news.join(" and ")))
    }
}
