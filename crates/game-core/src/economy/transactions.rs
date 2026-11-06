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
        Ok(quote)
    }
}
