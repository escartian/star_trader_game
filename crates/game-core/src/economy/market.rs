use super::{Credits, Economy, Market, MarketGood, TradeSide};
use crate::SystemId;

impl Economy {
    pub fn new(systems: impl IntoIterator<Item = SystemId>) -> Self {
        Self::with_rules(systems, super::EconomyRules::default())
            .expect("embedded rules are validated")
    }

    pub fn with_rules(
        systems: impl IntoIterator<Item = SystemId>,
        rules: super::EconomyRules,
    ) -> Result<Self, String> {
        rules.validate()?;
        let markets = systems
            .into_iter()
            .enumerate()
            .map(|(index, system)| {
                let template = &rules.markets[index % rules.markets.len()];
                Market {
                    system,
                    goods: std::array::from_fn(|commodity| MarketGood {
                        stock: template.stock,
                        target: template.target,
                        base_price: template.base_prices[commodity],
                        production: template.production[commodity],
                        consumption: template.consumption[commodity],
                    }),
                }
            })
            .collect();
        Ok(Self {
            credits: rules.starting_credits,
            markets,
            rules,
        })
    }

    /// Simultaneous net production/consumption. Bulk advancement equals individual
    /// ticks, including at capacity boundaries; no UI refresh randomness is involved.
    pub fn advance(&mut self, ticks: u64) {
        for market in &mut self.markets {
            for good in &mut market.goods {
                let net = i128::from(good.production) - i128::from(good.consumption);
                let next = i128::from(good.stock) + net * i128::from(ticks);
                good.stock = next.clamp(0, i128::from(good.capacity())) as u32;
            }
        }
    }
}

impl MarketGood {
    /// Stock-sensitive midpoint is bounded to 50–150% of the base price.
    /// Bids are rounded down and asks up, with a 10% spread on either side.
    pub fn price(&self, side: TradeSide, stock: u32) -> Credits {
        let target = u128::from(self.target.max(1));
        // Bids use post-deposit stock. This pairs each sold unit with the same
        // stock boundary as buying it back, even for small JSON-configured markets.
        let stock = (u128::from(stock) + u128::from(side == TradeSide::Sell)).min(target * 2);
        let numerator = u128::from(self.base_price.0) * (target * 3 - stock);
        let denominator = target * 2 * 100;
        let value = match side {
            TradeSide::Buy => (numerator * 110).div_ceil(denominator),
            TradeSide::Sell => numerator * 90 / denominator,
        };
        Credits(value.min(u128::from(u64::MAX)) as u64)
    }
}
