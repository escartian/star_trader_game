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
            TradeSide::Sell => {
                self.credits
                    .0
                    .checked_add(total)
                    .ok_or(EconomyError::ArithmeticOverflow)?;
            }
            _ => {}
        }
        Ok(TradeQuote {
            system: market.system,
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
            .find(|market| market.system == quote.system)
            .ok_or(EconomyError::UnknownMarket)?;
        let stock = &mut market.goods[commodity.index()].stock;
        *stock = match side {
            TradeSide::Buy => *stock - quantity,
            TradeSide::Sell => *stock + quantity,
        };
        self.credits = Credits(credits);
        ship.cargo[commodity.index()] = cargo;
        Ok(quote)
    }

    pub fn refuel(&mut self, ship: &mut Ship, units: u32) -> Result<(), EconomyError> {
        self.current_market(ship)?;
        if units == 0 {
            return Err(EconomyError::InvalidQuantity);
        }
        let fuel = ship
            .fuel
            .checked_add(units)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        if fuel > ship.fuel_capacity {
            return Err(EconomyError::FuelFull);
        }
        let cost = u64::from(units)
            .checked_mul(self.rules.fuel_unit_price.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        let credits = self
            .credits
            .0
            .checked_sub(cost)
            .ok_or(EconomyError::InsufficientCredits)?;
        self.credits = Credits(credits);
        ship.fuel = fuel;
        Ok(())
    }

    pub fn upgrade_hold(&mut self, ship: &mut Ship) -> Result<(), EconomyError> {
        self.current_market(ship)?;
        if ship.upgraded {
            return Err(EconomyError::AlreadyUpgraded);
        }
        let credits = self
            .credits
            .0
            .checked_sub(self.rules.hold_upgrade_price.0)
            .ok_or(EconomyError::InsufficientCredits)?;
        let capacity = ship
            .cargo_capacity
            .checked_add(self.rules.hold_upgrade_capacity)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.credits = Credits(credits);
        ship.cargo_capacity = capacity;
        ship.upgraded = true;
        Ok(())
    }
}
