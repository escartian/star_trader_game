use super::{Commodity, Credits, Economy, EconomyError, TradeQuote, TradeSide};
use crate::travel::Ship;

impl Economy {
    pub fn quote(
        &self,
        ship: &Ship,
        commodity: Commodity,
        quantity: u32,
        side: TradeSide,
        revision: u64,
    ) -> Result<TradeQuote, EconomyError> {
        if quantity == 0 {
            return Err(EconomyError::InvalidQuantity);
        }
        let market = self.current_market(ship)?;
        let good = &market.goods[commodity.index()];
        match side {
            TradeSide::Buy => {
                if quantity > good.stock {
                    return Err(EconomyError::InsufficientStock);
                }
                if ship.cargo_used() + u64::from(quantity) > u64::from(ship.cargo_capacity) {
                    return Err(EconomyError::CargoFull);
                }
            }
            TradeSide::Sell => {
                if quantity > ship.cargo[commodity.index()] {
                    return Err(EconomyError::InsufficientCargo);
                }
                if u64::from(good.stock) + u64::from(quantity) > u64::from(good.capacity()) {
                    return Err(EconomyError::StockFull);
                }
            }
        }
        // Price each unit along the stock curve; a bulk order cannot exploit a
        // single initial price then immediately reverse at a more favorable price.
        let total = (0..quantity).try_fold(0_u64, |total, index| {
            let stock = match side {
                TradeSide::Buy => good.stock - index,
                TradeSide::Sell => good.stock + index,
            };
            total
                .checked_add(good.price(side, stock).0)
                .ok_or(EconomyError::ArithmeticOverflow)
        })?;
        match side {
            TradeSide::Buy if total > self.credits.0 => {
                return Err(EconomyError::InsufficientCredits);
            }
            TradeSide::Buy => {
                market
                    .credits
                    .0
                    .checked_add(total)
                    .ok_or(EconomyError::ArithmeticOverflow)?;
            }
            TradeSide::Sell => {
                if total > market.credits.0 {
                    return Err(EconomyError::MarketInsufficientCredits);
                }
                self.credits
                    .0
                    .checked_add(total)
                    .ok_or(EconomyError::ArithmeticOverflow)?;
            }
        }
        Ok(TradeQuote {
            market: market.id,
            commodity,
            quantity,
            side,
            first_unit_price: good.price(side, good.stock),
            total: Credits(total),
            revision,
        })
    }

    /// The largest order of `commodity` this ship can place on `side` now, or 0.
    /// Every quote constraint (stock, cargo space, cumulative price against credits or
    /// buyer funds, market storage) tightens as quantity grows, so feasibility is
    /// monotonic and a binary search over the cargo/stock bound finds the boundary.
    pub fn max_trade_quantity(&self, ship: &Ship, commodity: Commodity, side: TradeSide) -> u32 {
        let Ok(market) = self.current_market(ship) else {
            return 0;
        };
        let free_space = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
        let upper = match side {
            TradeSide::Buy => market.goods[commodity.index()]
                .stock
                .min(u32::try_from(free_space).unwrap_or(u32::MAX)),
            TradeSide::Sell => ship.cargo[commodity.index()],
        };
        // Invariant: `low` is tradable (0 trivially) and nothing above `high` is.
        let (mut low, mut high) = (0, upper);
        while low < high {
            let middle = low + (high - low).div_ceil(2);
            if self.quote(ship, commodity, middle, side, 0).is_ok() {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        low
    }

    pub fn trade(
        &mut self,
        ship: &mut Ship,
        commodity: Commodity,
        quantity: u32,
        side: TradeSide,
        revision: u64,
    ) -> Result<TradeQuote, EconomyError> {
        let quote = self.quote(ship, commodity, quantity, side, revision)?;
        let cargo = ship.cargo[commodity.index()];
        let (credits, cargo) = match side {
            TradeSide::Buy => (
                self.credits.0 - quote.total.0,
                cargo
                    .checked_add(quantity)
                    .ok_or(EconomyError::ArithmeticOverflow)?,
            ),
            TradeSide::Sell => (self.credits.0 + quote.total.0, cargo - quantity),
        };
        let market = self
            .markets
            .iter_mut()
            .find(|market| market.id == quote.market)
            .ok_or(EconomyError::UnknownMarket)?;
        let stock = &mut market.goods[commodity.index()].stock;
        *stock = match side {
            TradeSide::Buy => *stock - quantity,
            TradeSide::Sell => *stock + quantity,
        };
        market.credits = Credits(match side {
            TradeSide::Buy => market.credits.0 + quote.total.0,
            TradeSide::Sell => market.credits.0 - quote.total.0,
        });
        self.credits = Credits(credits);
        ship.cargo[commodity.index()] = cargo;
        // The holder's own trades earn the port share here; NPC trades pay it
        // once the holder's wallet is back in place (`npc::decisions`).
        if self.charter_holder(quote.market) == Some(ship.owner) {
            self.pay_port_share(quote.market, quote.total);
        }
        Ok(quote)
    }
}
