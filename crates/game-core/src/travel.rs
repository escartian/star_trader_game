//! Discrete travel rules, independent of real-time clocks and player input devices.
use crate::{GalaxyPosition, SystemId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Journey {
    pub origin: SystemId,
    pub destination: SystemId,
    pub arrival_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    Docked(SystemId),
    Travelling(Journey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ship {
    pub id: ShipId,
    pub owner: PlayerId,
    pub fuel: u32,
    pub fuel_capacity: u32,
    pub range_ly: u32,
    pub location: Location,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TravelQuote {
    pub origin: SystemId,
    pub destination: SystemId,
    pub fuel: u32,
    pub duration_ticks: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TravelError {
    UnknownSystem,
    UnknownShip,
    NotOwner,
    AlreadyTravelling,
    AlreadyThere,
    OutOfRange,
    InsufficientFuel,
    InvalidCoordinates,
    TimeOverflow,
    StaleQuote,
}

impl std::fmt::Display for TravelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnknownSystem => "Unknown destination system.",
            Self::UnknownShip => "Unknown ship.",
            Self::NotOwner => "This player does not own the ship.",
            Self::AlreadyTravelling => "The ship is already travelling.",
            Self::AlreadyThere => "The ship is already at this system.",
            Self::OutOfRange => "Destination exceeds the ship's jump range.",
            Self::InsufficientFuel => "Not enough fuel for this journey.",
            Self::InvalidCoordinates => "Coordinates exceed the supported travel calculation.",
            Self::TimeOverflow => "Simulation time or revision limit reached.",
            Self::StaleQuote => "The session changed; review the new journey quote.",
        })
    }
}

/// The authority resolves identity before calling rules; commands never supply an owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TravelState {
    pub tick: u64,
    pub revision: u64,
    pub ship: Ship,
}

impl Default for TravelState {
    fn default() -> Self {
        Self {
            tick: 0,
            revision: 0,
            ship: Ship {
                id: ShipId(1),
                owner: PlayerId(1),
                fuel: 40,
                fuel_capacity: 40,
                range_ly: 20,
                location: Location::Docked(SystemId(1)),
            },
        }
    }
}

impl TravelState {
    pub fn authorize(&self, player: PlayerId, ship: ShipId) -> Result<(), TravelError> {
        if ship != self.ship.id {
            return Err(TravelError::UnknownShip);
        }
        if player != self.ship.owner {
            return Err(TravelError::NotOwner);
        }
        Ok(())
    }

    pub fn quote(
        &self,
        destination: SystemId,
        origin: GalaxyPosition,
        target: GalaxyPosition,
    ) -> Result<TravelQuote, TravelError> {
        let Location::Docked(origin_id) = self.ship.location else {
            return Err(TravelError::AlreadyTravelling);
        };
        if origin_id == destination {
            return Err(TravelError::AlreadyThere);
        }
        let squared = distance_squared(origin, target)?;
        let range = u128::from(self.ship.range_ly);
        if squared > range * range {
            return Err(TravelError::OutOfRange);
        }
        let floor = squared.isqrt();
        let ceil = floor + u128::from(floor * floor != squared);
        let fuel = u32::try_from(ceil).map_err(|_| TravelError::InvalidCoordinates)?;
        if fuel > self.ship.fuel {
            return Err(TravelError::InsufficientFuel);
        }
        Ok(TravelQuote {
            origin: origin_id,
            destination,
            fuel,
            duration_ticks: u64::from(fuel).max(1),
            revision: self.revision,
        })
    }

    /// Receives an authority-generated quote, never an unchecked client quote.
    pub fn depart(&mut self, quote: TravelQuote) -> Result<(), TravelError> {
        if quote.revision != self.revision {
            return Err(TravelError::StaleQuote);
        }
        let arrival_tick = self
            .tick
            .checked_add(quote.duration_ticks)
            .ok_or(TravelError::TimeOverflow)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let fuel = self
            .ship
            .fuel
            .checked_sub(quote.fuel)
            .ok_or(TravelError::InsufficientFuel)?;
        // All fallible work finishes before any authoritative mutation.
        self.ship.fuel = fuel;
        self.ship.location = Location::Travelling(Journey {
            origin: quote.origin,
            destination: quote.destination,
            arrival_tick,
        });
        self.revision = revision;
        Ok(())
    }

    /// Called by a host clock/turn resolver, never directly by an untrusted player.
    pub fn advance(&mut self, ticks: u64) -> Result<Option<SystemId>, TravelError> {
        if ticks == 0 {
            return Ok(None);
        }
        let tick = self
            .tick
            .checked_add(ticks)
            .ok_or(TravelError::TimeOverflow)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let arrived = match &self.ship.location {
            Location::Travelling(journey) if journey.arrival_tick <= tick => {
                Some(journey.destination)
            }
            _ => None,
        };
        self.tick = tick;
        self.revision = revision;
        if let Some(system) = arrived {
            self.ship.location = Location::Docked(system);
        }
        Ok(arrived)
    }
}

fn distance_squared(origin: GalaxyPosition, target: GalaxyPosition) -> Result<u128, TravelError> {
    [
        (origin.x, target.x),
        (origin.y, target.y),
        (origin.z, target.z),
    ]
    .into_iter()
    .try_fold(0_u128, |sum, (a, b)| {
        let delta = (i128::from(a) - i128::from(b)).unsigned_abs();
        delta
            .checked_mul(delta)
            .and_then(|square| sum.checked_add(square))
            .ok_or(TravelError::InvalidCoordinates)
    })
}

#[cfg(test)]
mod tests;
