use super::{Credits, Economy, EconomyError};
use crate::travel::Ship;

impl Economy {
    pub fn refuel(&mut self, ship: &mut Ship, units: u32) -> Result<(), EconomyError> {
        let id = self.current_market(ship)?.id;
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
        let market_credits = self
            .market(id)?
            .credits
            .0
            .checked_add(cost)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.market_mut(id)?.credits = Credits(market_credits);
        self.credits = Credits(credits);
        ship.fuel = fuel;
        Ok(())
    }

    /// Fuel units that fill the tank as far as credits allow: the order a "Full"
    /// refuel sends. Errors explain why nothing can be bought (no market, full tank,
    /// no credits for one unit).
    pub fn max_refuel_units(&self, ship: &Ship) -> Result<u32, EconomyError> {
        self.current_market(ship)?;
        let missing = ship.fuel_capacity.saturating_sub(ship.fuel);
        if missing == 0 {
            return Err(EconomyError::FuelFull);
        }
        let price = self.rules.fuel_unit_price.0;
        let affordable = self.credits.0.checked_div(price).unwrap_or(u64::MAX);
        match u32::try_from(affordable).map_or(missing, |units| units.min(missing)) {
            0 => Err(EconomyError::InsufficientCredits),
            units => Ok(units),
        }
    }

    pub fn upgrade_hold(&mut self, ship: &mut Ship) -> Result<(), EconomyError> {
        let id = self.current_market(ship)?.id;
        if ship.upgraded {
            return Err(EconomyError::AlreadyUpgraded);
        }
        let cost = self.rules.hold_upgrade_price.0;
        let credits = self
            .credits
            .0
            .checked_sub(cost)
            .ok_or(EconomyError::InsufficientCredits)?;
        let capacity = ship
            .cargo_capacity
            .checked_add(self.rules.hold_upgrade_capacity)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        let market_credits = self
            .market(id)?
            .credits
            .0
            .checked_add(cost)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.market_mut(id)?.credits = Credits(market_credits);
        self.credits = Credits(credits);
        ship.cargo_capacity = capacity;
        ship.upgraded = true;
        Ok(())
    }

    /// Cost to restore every lost hull point at the current market.
    pub fn repair_quote(&self, ship: &Ship) -> Result<Credits, EconomyError> {
        self.current_market(ship)?;
        if ship.hull_damage == 0 {
            return Err(EconomyError::NoDamage);
        }
        u64::from(ship.hull_damage)
            .checked_mul(self.rules.repair_point_price.0)
            .map(Credits)
            .ok_or(EconomyError::ArithmeticOverflow)
    }

    /// Full hull repair; the market treasury receives the payment.
    pub fn repair(&mut self, ship: &mut Ship) -> Result<Credits, EconomyError> {
        let cost = self.repair_quote(ship)?;
        let id = self.current_market(ship)?.id;
        let credits = self
            .credits
            .0
            .checked_sub(cost.0)
            .ok_or(EconomyError::InsufficientCredits)?;
        let market_credits = self
            .market(id)?
            .credits
            .0
            .checked_add(cost.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.market_mut(id)?.credits = Credits(market_credits);
        self.credits = Credits(credits);
        ship.hull_damage = 0;
        Ok(cost)
    }
}
