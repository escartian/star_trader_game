//! Buying anti-piracy gear and hiring escorts at a market (`combat::gear`).
//! The payment goes to the market's treasury, like repairs and fuel.
use super::{Credits, Economy, EconomyError};
use crate::{
    combat::gear::{FREIGHTER_CARGO_UNITS, Gear, SCOUT_RANGE_LY},
    travel::Ship,
};

impl Economy {
    /// Today's price of `gear`.
    pub fn gear_price(&self, gear: Gear) -> Credits {
        self.inflate(self.rules.gear_prices.of(gear))
    }

    /// Today's price of one escort contract.
    pub fn escort_price(&self) -> Credits {
        self.inflate(self.rules.gear_prices.escort)
    }

    /// Fits `gear` to `ship` at its current market. Returns the price paid.
    pub fn fit_gear(&mut self, ship: &mut Ship, gear: Gear) -> Result<Credits, EconomyError> {
        if ship.gear.contains(&gear) {
            return Err(EconomyError::AlreadyFitted);
        }
        let size = self
            .rules
            .ship_catalog
            .spec(ship.kind)
            .ok_or(EconomyError::IncompatibleGear)?
            .combat
            .size;
        if !gear.fits(ship.kind, size) {
            return Err(EconomyError::IncompatibleGear);
        }
        let cargo = ship
            .cargo_capacity
            .checked_add(if gear == Gear::FreighterRacks {
                FREIGHTER_CARGO_UNITS
            } else {
                0
            })
            .ok_or(EconomyError::ArithmeticOverflow)?;
        let range = ship
            .range_ly
            .checked_add(if gear == Gear::ScoutDrive {
                SCOUT_RANGE_LY
            } else {
                0
            })
            .ok_or(EconomyError::ArithmeticOverflow)?;
        if range > 1_000_000 {
            return Err(EconomyError::ArithmeticOverflow);
        }
        let cost = self.gear_price(gear);
        self.pay_market(ship, cost)?;
        ship.cargo_capacity = cargo;
        ship.range_ly = range;
        ship.gear.push(gear);
        Ok(cost)
    }

    /// Pays for an escort contract at the market where `lead` (the fleet's
    /// lead ship) is. The caller records the contract. Returns the price paid.
    pub fn hire_escort(&mut self, lead: &Ship) -> Result<Credits, EconomyError> {
        let cost = self.escort_price();
        self.pay_market(lead, cost)?;
        Ok(cost)
    }

    /// Moves `cost` from the wallet to the treasury of the market `ship` is at.
    fn pay_market(&mut self, ship: &Ship, cost: Credits) -> Result<(), EconomyError> {
        let id = self.current_market(ship)?.id;
        self.require_open_port(ship)?;
        let credits = self
            .credits
            .0
            .checked_sub(cost.0)
            .ok_or(EconomyError::InsufficientCredits)?;
        let treasury = self
            .market(id)?
            .credits
            .0
            .checked_add(cost.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.market_mut(id)?.credits = Credits(treasury);
        self.credits = Credits(credits);
        Ok(())
    }
}
