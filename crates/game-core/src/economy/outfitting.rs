//! Buying anti-piracy gear and hiring escorts at a market (`combat::gear`).
//! The payment goes to the market's treasury, like repairs and fuel.
use super::{Credits, Economy, EconomyError};
use crate::{
    combat::gear::{FREIGHTER_CARGO_UNITS, Gear, SCOUT_RANGE_LY},
    travel::Ship,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipUpgrade {
    Hold,
    Gear(Gear),
}

impl ShipUpgrade {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hold => "Cargo hold upgrade",
            Self::Gear(gear) => gear.label(),
        }
    }
}

impl Economy {
    /// Preview a shipyard refit without cloning the economy or changing state.
    pub fn quote_refit(
        &self,
        ship: &Ship,
        remove: ShipUpgrade,
        install: Option<ShipUpgrade>,
    ) -> Result<(Credits, Credits), EconomyError> {
        if install == Some(remove) {
            return Err(EconomyError::InvalidQuantity);
        }
        let market = self.current_market(ship)?;
        if !market.ship_stock.iter().any(|stock| *stock > 0) {
            return Err(EconomyError::NoShipyard);
        }
        self.require_open_port(ship)?;
        let fitted = |upgrade| match upgrade {
            ShipUpgrade::Hold => ship.upgraded,
            ShipUpgrade::Gear(gear) => ship.gear.contains(&gear),
        };
        if !fitted(remove) {
            return Err(EconomyError::NotFitted);
        }
        let mut capacity = ship.cargo_capacity;
        let mut range = ship.range_ly;
        match remove {
            ShipUpgrade::Hold => {
                capacity = capacity
                    .checked_sub(self.rules.hold_upgrade_capacity)
                    .ok_or(EconomyError::ArithmeticOverflow)?
            }
            ShipUpgrade::Gear(Gear::FreighterRacks) => {
                capacity = capacity
                    .checked_sub(FREIGHTER_CARGO_UNITS)
                    .ok_or(EconomyError::ArithmeticOverflow)?
            }
            ShipUpgrade::Gear(Gear::ScoutDrive) => {
                range = range
                    .checked_sub(SCOUT_RANGE_LY)
                    .ok_or(EconomyError::ArithmeticOverflow)?
            }
            ShipUpgrade::Gear(_) => {}
        }
        let cost = match install {
            Some(ShipUpgrade::Hold) => {
                if ship.upgraded && remove != ShipUpgrade::Hold {
                    return Err(EconomyError::AlreadyUpgraded);
                }
                capacity = capacity
                    .checked_add(self.rules.hold_upgrade_capacity)
                    .ok_or(EconomyError::ArithmeticOverflow)?;
                self.hold_upgrade_price()
            }
            Some(ShipUpgrade::Gear(gear)) => {
                if fitted(ShipUpgrade::Gear(gear)) && remove != ShipUpgrade::Gear(gear) {
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
                if gear == Gear::FreighterRacks {
                    capacity = capacity
                        .checked_add(FREIGHTER_CARGO_UNITS)
                        .ok_or(EconomyError::ArithmeticOverflow)?;
                }
                if gear == Gear::ScoutDrive {
                    range = range
                        .checked_add(SCOUT_RANGE_LY)
                        .ok_or(EconomyError::ArithmeticOverflow)?;
                }
                self.gear_price(gear)
            }
            None => Credits(0),
        };
        if ship.cargo_used() > u64::from(capacity) {
            return Err(EconomyError::CargoFull);
        }
        if range > 1_000_000 {
            return Err(EconomyError::ArithmeticOverflow);
        }
        let refund = self.upgrade_refund(remove);
        let treasury = market
            .credits
            .0
            .checked_sub(refund.0)
            .ok_or(EconomyError::MarketInsufficientCredits)?;
        let wallet = self
            .credits
            .0
            .checked_add(refund.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        wallet
            .checked_sub(cost.0)
            .ok_or(EconomyError::InsufficientCredits)?;
        treasury
            .checked_add(cost.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        Ok((refund, cost))
    }

    pub fn upgrade_refund(&self, upgrade: ShipUpgrade) -> Credits {
        let price = match upgrade {
            ShipUpgrade::Hold => self.hold_upgrade_price(),
            ShipUpgrade::Gear(gear) => self.gear_price(gear),
        };
        Credits(price.0 * u64::from(self.rules.ship_resale_percent) / 100)
    }

    /// Remove one upgrade at a shipyard, optionally fitting its replacement
    /// in the same transaction. The yard refunds the old item's resale value.
    /// Cargo must fit after any capacity reduction; a failed swap changes nothing.
    pub fn refit_ship(
        &mut self,
        ship: &mut Ship,
        remove: ShipUpgrade,
        install: Option<ShipUpgrade>,
    ) -> Result<(Credits, Credits), EconomyError> {
        self.quote_refit(ship, remove, install)?;
        let mut economy = self.clone();
        let mut candidate = ship.clone();
        let refund = economy.remove_upgrade(&mut candidate, remove)?;
        let cost = match install {
            Some(ShipUpgrade::Hold) => {
                let cost = economy.hold_upgrade_price();
                economy.upgrade_hold(&mut candidate)?;
                cost
            }
            Some(ShipUpgrade::Gear(gear)) => economy.fit_gear(&mut candidate, gear)?,
            None => Credits(0),
        };
        if candidate.cargo_used() > u64::from(candidate.cargo_capacity) {
            return Err(EconomyError::CargoFull);
        }
        *self = economy;
        *ship = candidate;
        Ok((refund, cost))
    }

    fn remove_upgrade(
        &mut self,
        ship: &mut Ship,
        upgrade: ShipUpgrade,
    ) -> Result<Credits, EconomyError> {
        let market = self.current_market(ship)?;
        if !market.ship_stock.iter().any(|stock| *stock > 0) {
            return Err(EconomyError::NoShipyard);
        }
        let id = market.id;
        self.require_open_port(ship)?;
        let new_capacity = match upgrade {
            ShipUpgrade::Hold if ship.upgraded => ship
                .cargo_capacity
                .checked_sub(self.rules.hold_upgrade_capacity),
            ShipUpgrade::Gear(Gear::FreighterRacks)
                if ship.gear.contains(&Gear::FreighterRacks) =>
            {
                ship.cargo_capacity.checked_sub(FREIGHTER_CARGO_UNITS)
            }
            ShipUpgrade::Gear(gear) if ship.gear.contains(&gear) => Some(ship.cargo_capacity),
            _ => return Err(EconomyError::NotFitted),
        }
        .ok_or(EconomyError::ArithmeticOverflow)?;
        let refund = self.upgrade_refund(upgrade);
        let treasury = self
            .market(id)?
            .credits
            .0
            .checked_sub(refund.0)
            .ok_or(EconomyError::MarketInsufficientCredits)?;
        let wallet = self
            .credits
            .0
            .checked_add(refund.0)
            .ok_or(EconomyError::ArithmeticOverflow)?;
        self.market_mut(id)?.credits = Credits(treasury);
        self.credits = Credits(wallet);
        ship.cargo_capacity = new_capacity;
        match upgrade {
            ShipUpgrade::Hold => ship.upgraded = false,
            ShipUpgrade::Gear(gear) => {
                ship.gear.retain(|fitted| *fitted != gear);
                if gear == Gear::ScoutDrive {
                    ship.range_ly = ship
                        .range_ly
                        .checked_sub(SCOUT_RANGE_LY)
                        .ok_or(EconomyError::ArithmeticOverflow)?;
                }
            }
        }
        Ok(refund)
    }

    /// Today's price of `gear`.
    pub fn gear_price(&self, gear: Gear) -> Credits {
        self.inflate(self.rules.gear_prices.of(gear))
    }

    /// Today's price of one escort contract.
    /// Price of fleet `software` at today's price level.
    pub fn software_price(&self, software: crate::fleet::software::FleetSoftware) -> Credits {
        self.inflate(self.rules.software_prices.of(software))
    }

    /// Installs `software` on `fleet`, paid at the market `lead` is at.
    pub fn install_software(
        &mut self,
        lead: &Ship,
        fleet: &mut crate::fleet::Fleet,
        software: crate::fleet::software::FleetSoftware,
    ) -> Result<Credits, EconomyError> {
        if fleet.software.contains(&software) {
            return Err(EconomyError::SoftwareInstalled);
        }
        let cost = self.software_price(software);
        self.pay_market(lead, cost)?;
        fleet.software.push(software);
        Ok(cost)
    }

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
