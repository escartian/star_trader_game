//! Discrete travel rules, independent of real-time clocks and player input devices.
use crate::{
    GalaxyPosition, SystemId,
    fleet::{Fleet, FleetId, ShipKind},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShipId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journey {
    pub origin: SystemId,
    pub destination: SystemId,
    pub arrival_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    /// Docked at the system's primary trade station (legacy saves retain this ID).
    Docked(SystemId),
    Orbiting {
        system: SystemId,
        body: crate::BodyId,
    },
    /// On the surface of a rocky planet, moon or asteroid. Landing and lifting
    /// off cost fuel by the body's gravity (`movement::LandingRules`).
    Landed {
        system: SystemId,
        body: crate::BodyId,
    },
    InSystem {
        system: SystemId,
        position: crate::LocalPosition,
    },
    DeepSpace(crate::movement::SpacePosition),
    Travelling(Journey),
    InTransit(FleetId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ship {
    pub id: ShipId,
    pub owner: PlayerId,
    pub name: String,
    pub kind: ShipKind,
    #[serde(with = "crate::economy::cargo_json")]
    pub cargo: [u32; 6],
    pub cargo_capacity: u32,
    pub base_cargo_capacity: u32,
    pub upgraded: bool,
    pub fuel: u32,
    pub fuel_capacity: u32,
    pub range_ly: u32,
    pub location: Location,
    /// Structure points lost in combat; 0 = intact. Max hull comes from the class.
    #[serde(default)]
    pub hull_damage: u32,
    /// Anti-piracy gear fitted (see `combat::gear`), each item at most once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gear: Vec<crate::combat::gear::Gear>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelQuote {
    pub ship: ShipId,
    pub origin: SystemId,
    pub destination: SystemId,
    pub fuel: u32,
    pub duration_ticks: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TravelError {
    UnknownSystem,
    UnknownShip,
    NotOwner,
    AlreadyTravelling,
    NotTravelling,
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
            Self::NotTravelling => "The ship has no journey to continue.",
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelState {
    /// The player's selected species, independent of allegiance.
    #[serde(default)]
    pub player_species: crate::species::Species,
    pub navigation: crate::navigation::NavigationState,
    pub movement_rules: crate::movement::MovementRules,
    pub tick: u64,
    pub revision: u64,
    pub ships: Vec<Ship>,
    pub fleets: Vec<Fleet>,
    pub next_ship_id: u64,
    pub next_fleet_id: u64,
    /// Run limit and how the campaign ended (see `campaign`).
    #[serde(default)]
    pub campaign: crate::campaign::Campaign,
    /// Features unlocked so far (see `progress`); saves from before unlocks
    /// load with everything unlocked.
    #[serde(default)]
    pub progress: crate::progress::Progress,
    /// Job board offers and accepted jobs (see `jobs`); omitted when empty.
    #[serde(default, skip_serializing_if = "crate::jobs::Jobs::is_empty")]
    pub jobs: crate::jobs::Jobs,
}

impl Default for TravelState {
    fn default() -> Self {
        Self {
            player_species: crate::species::Species::Human,
            navigation: crate::navigation::NavigationState::default(),
            movement_rules: crate::movement::MovementRules::default(),
            tick: 0,
            revision: 0,
            ships: vec![Ship {
                id: ShipId(1),
                owner: PlayerId(1),
                name: "Freighter 1".into(),
                kind: ShipKind::Freighter,
                cargo: [0; 6],
                cargo_capacity: 20,
                base_cargo_capacity: 20,
                upgraded: false,
                gear: Vec::new(),
                fuel: 40,
                fuel_capacity: 40,
                range_ly: 20,
                location: Location::Docked(SystemId(1)),
                hull_damage: 0,
            }],
            fleets: vec![Fleet {
                id: FleetId(1),
                name: "Fleet 1".into(),
                owner: PlayerId(1),
                ships: vec![ShipId(1)],
                captain: crate::species::FleetCaptain::default(),
                software: Vec::new(),
            }],
            next_ship_id: 2,
            next_fleet_id: 2,
            campaign: crate::campaign::Campaign::default(),
            progress: crate::progress::Progress::default(),
            jobs: crate::jobs::Jobs::default(),
        }
    }
}

impl TravelState {
    pub fn ship(&self, id: ShipId) -> Result<&Ship, TravelError> {
        self.ships
            .iter()
            .find(|ship| ship.id == id)
            .ok_or(TravelError::UnknownShip)
    }

    pub fn ship_mut(&mut self, id: ShipId) -> Result<&mut Ship, TravelError> {
        self.ships
            .iter_mut()
            .find(|ship| ship.id == id)
            .ok_or(TravelError::UnknownShip)
    }

    pub fn authorize(&self, player: PlayerId, id: ShipId) -> Result<(), TravelError> {
        if self.ship(id)?.owner != player {
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
        self.quote_for(self.ships[0].id, destination, origin, target)
    }

    pub fn quote_for(
        &self,
        id: ShipId,
        destination: SystemId,
        origin: GalaxyPosition,
        target: GalaxyPosition,
    ) -> Result<TravelQuote, TravelError> {
        let ship = self.ship(id)?;
        let Location::Docked(origin_id) = ship.location else {
            return Err(TravelError::AlreadyTravelling);
        };
        if origin_id == destination {
            return Err(TravelError::AlreadyThere);
        }
        let squared = distance_squared(origin, target)?;
        let range = u128::from(ship.range_ly);
        if squared > range * range {
            return Err(TravelError::OutOfRange);
        }
        let floor = squared.isqrt();
        let ceil = floor + u128::from(floor * floor != squared);
        let fuel = u32::try_from(ceil).map_err(|_| TravelError::InvalidCoordinates)?;
        if fuel > ship.fuel {
            return Err(TravelError::InsufficientFuel);
        }
        Ok(TravelQuote {
            ship: id,
            origin: origin_id,
            destination,
            fuel,
            duration_ticks: u64::from(fuel).max(1),
            revision: self.revision,
        })
    }

    /// Receives an authority-generated quote, never an unchecked client quote.
    pub fn depart(&mut self, quote: TravelQuote) -> Result<(), TravelError> {
        self.depart_many(&[quote])
    }

    pub fn depart_many(&mut self, quotes: &[TravelQuote]) -> Result<(), TravelError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        for (index, quote) in quotes.iter().enumerate() {
            if quote.revision != self.revision
                || quotes[..index].iter().any(|other| other.ship == quote.ship)
            {
                return Err(TravelError::StaleQuote);
            }
            let ship = self.ship(quote.ship)?;
            if ship.location != Location::Docked(quote.origin) {
                return Err(TravelError::AlreadyTravelling);
            }
            ship.fuel
                .checked_sub(quote.fuel)
                .ok_or(TravelError::InsufficientFuel)?;
            self.tick
                .checked_add(quote.duration_ticks)
                .ok_or(TravelError::TimeOverflow)?;
        }
        for quote in quotes {
            let arrival_tick = self.tick + quote.duration_ticks;
            let ship = self.ship_mut(quote.ship)?;
            ship.fuel -= quote.fuel;
            ship.location = Location::Travelling(Journey {
                origin: quote.origin,
                destination: quote.destination,
                arrival_tick,
            });
        }
        self.revision = revision;
        Ok(())
    }

    /// Called by a host clock/turn resolver, never directly by an untrusted player.
    pub fn advance(&mut self, ticks: u64) -> Result<Vec<(ShipId, SystemId)>, TravelError> {
        if ticks == 0 {
            return Ok(Vec::new());
        }
        let tick = self
            .tick
            .checked_add(ticks)
            .ok_or(TravelError::TimeOverflow)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let mut arrived = Vec::new();
        for ship in &mut self.ships {
            if let Location::Travelling(journey) = &ship.location
                && journey.arrival_tick <= tick
            {
                let destination = journey.destination;
                ship.location = Location::Docked(destination);
                arrived.push((ship.id, destination));
            }
        }
        self.tick = tick;
        self.revision = revision;
        Ok(arrived)
    }
}

impl Location {
    pub fn system(&self) -> Option<SystemId> {
        match self {
            Self::Docked(system)
            | Self::Orbiting { system, .. }
            | Self::Landed { system, .. }
            | Self::InSystem { system, .. } => Some(*system),
            Self::Travelling(journey) => Some(journey.origin),
            Self::DeepSpace(_) | Self::InTransit(_) => None,
        }
    }
    pub fn stationary(&self) -> bool {
        !matches!(self, Self::Travelling(_) | Self::InTransit(_))
    }
}

impl Ship {
    pub fn cargo_used(&self) -> u64 {
        self.cargo.iter().map(|quantity| u64::from(*quantity)).sum()
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
