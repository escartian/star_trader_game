//! Finite hull stock and treasury transfers, independent of fleet/UI state.
use super::{Credits, Economy, EconomyError};
use crate::{
    fleet::{ShipKind, ShipSpec},
    travel::Ship,
};

impl Economy {
    pub fn buy_hull(&mut self, ship: &Ship, kind: ShipKind) -> Result<ShipSpec, EconomyError> {
        let market = self.current_market(ship)?;
        self.require_open_port(ship)?;
        let id = market.id;
        if market.ship_stock[kind.index()] == 0 {
            return Err(EconomyError::ShipOutOfStock);
        }
        let spec = self
            .rules
            .ship_catalog
            .ships
            .iter()
            .find(|spec| spec.kind == kind)
            .ok_or(EconomyError::UnknownMarket)?
            .clone();
        let price = self.inflate(spec.price);
        let credits = self
            .credits
            .0
            .checked_sub(price.0)
            .ok_or(EconomyError::InsufficientCredits)?;
        let market_credits = market
            .credits
            .0
            .checked_add(price.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        let market = self.market_mut(id)?;
        market.credits = Credits(market_credits);
        market.ship_stock[kind.index()] -= 1;
        self.credits = Credits(credits);
        Ok(spec)
    }

    pub fn quote_ship_sale(&self, ship: &Ship) -> Result<Credits, EconomyError> {
        if ship.cargo_used() != 0 {
            return Err(EconomyError::NonEmptyHold);
        }
        let market = self.current_market(ship)?;
        self.require_open_port(ship)?;
        if market.ship_stock[ship.kind.index()] >= 1000 {
            return Err(EconomyError::StockFull);
        }
        let price = self
            .ship_resale(ship.kind)
            .ok_or(EconomyError::UnknownMarket)?;
        if market.credits < price {
            return Err(EconomyError::MarketInsufficientCredits);
        }
        self.credits
            .0
            .checked_add(price.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        Ok(price)
    }

    pub fn sell_hull(&mut self, ship: &Ship) -> Result<Credits, EconomyError> {
        let price = self.quote_ship_sale(ship)?;
        let id = self.current_market(ship)?.id;
        let market = self.market_mut(id)?;
        market.credits.0 -= price.0;
        market.ship_stock[ship.kind.index()] += 1;
        self.credits.0 += price.0;
        Ok(price)
    }
}
