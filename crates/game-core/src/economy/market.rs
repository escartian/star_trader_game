use super::{Credits, Economy, Market, MarketGood, TradeSide};
use crate::{BodyKind, StarSystem};

impl Economy {
    pub fn new(systems: &[StarSystem]) -> Self {
        Self::with_rules(systems, super::EconomyRules::default())
            .expect("embedded rules are validated")
    }

    pub fn with_rules(systems: &[StarSystem], rules: super::EconomyRules) -> Result<Self, String> {
        rules.validate()?;
        let mut markets = Vec::new();
        for (index, system) in systems.iter().enumerate() {
            let station = system
                .bodies
                .iter()
                .find(|body| body.kind == BodyKind::Station)
                .ok_or("System is missing its primary trade station")?;
            markets.push(Market::from_template(
                super::MarketId {
                    system: system.id,
                    body: station.id,
                },
                true,
                &rules.markets[index % rules.markets.len()],
                rules.market_starting_credits,
                rules.shipyard_starting_stock,
            ));
            for body in &system.bodies {
                if let Some(planet) = rules.planet_catalog.template_for_body(index, body) {
                    let mut market = Market::from_template(
                        super::MarketId {
                            system: system.id,
                            body: body.id,
                        },
                        false,
                        &rules.markets[planet.market_template % rules.markets.len()],
                        planet.starting_credits,
                        planet.ship_stock,
                    );
                    // Local surplus and demand: scale base prices per commodity.
                    for (good, percent) in market.goods.iter_mut().zip(planet.price_percent) {
                        good.base_price = Credits(
                            (u128::from(good.base_price.0) * u128::from(percent) / 100).max(1)
                                as u64,
                        );
                    }
                    // Specialization creates an actual production advantage, independent
                    // of rendering and without increasing initial commodity stock.
                    market.goods[planet.profile.specialization.index()].production = market.goods
                        [planet.profile.specialization.index()]
                    .production
                    .saturating_add(2)
                    .min(1000);
                    markets.push(market);
                }
            }
        }
        Ok(Self {
            traders: Vec::new(),
            credits: rules.starting_credits,
            markets,
            rules,
            price_level_ppm: super::PRICE_LEVEL_UNIT,
            charters: Vec::new(),
            factions: Default::default(),
        })
    }

    /// Simultaneous net production/consumption. Bulk advancement equals individual
    /// ticks, including at capacity boundaries; no UI refresh randomness is involved.
    /// Then, tick by tick, inflation and market income and upkeep (`dynamics`).
    pub fn advance(&mut self, ticks: u64) {
        for market in &mut self.markets {
            for good in &mut market.goods {
                let net = i128::from(good.production) - i128::from(good.consumption);
                let next = i128::from(good.stock) + net * i128::from(ticks);
                good.stock = next.clamp(0, i128::from(good.capacity())) as u32;
            }
        }
        for _ in 0..ticks {
            self.advance_dynamics();
        }
    }
}

impl Market {
    pub fn from_template(
        id: super::MarketId,
        is_station: bool,
        template: &super::MarketTemplate,
        credits: Credits,
        ship_stock: [u32; crate::fleet::SHIP_KIND_COUNT],
    ) -> Self {
        Self {
            id,
            is_station,
            credits,
            activity: 0,
            ship_stock,
            goods: std::array::from_fn(|commodity| MarketGood {
                stock: template.stock,
                target: template.target,
                base_price: template.base_prices[commodity],
                production: template.production[commodity],
                consumption: template.consumption[commodity],
                price_level_ppm: super::PRICE_LEVEL_UNIT,
            }),
        }
    }
}

impl MarketGood {
    /// Stock-sensitive midpoint is bounded to 50–150% of the base price at
    /// today's price level.
    /// Bids are rounded down and asks up, with a 10% spread on either side.
    pub fn price(&self, side: TradeSide, stock: u32) -> Credits {
        let target = u128::from(self.target.max(1));
        // Bids use post-deposit stock. This pairs each sold unit with the same
        // stock boundary as buying it back, even for small JSON-configured markets.
        let stock = (u128::from(stock) + u128::from(side == TradeSide::Sell)).min(target * 2);
        let base = u128::from(self.base_price.0) * u128::from(self.price_level_ppm)
            / u128::from(super::PRICE_LEVEL_UNIT);
        let numerator = base * (target * 3 - stock);
        let denominator = target * 2 * 100;
        let value = match side {
            TradeSide::Buy => (numerator * 110).div_ceil(denominator),
            TradeSide::Sell => numerator * 90 / denominator,
        };
        Credits(value.min(u128::from(u64::MAX)) as u64)
    }
}
