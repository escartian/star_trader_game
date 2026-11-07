use super::{Fleet, FleetError, FleetId, ShipSpec};
use crate::{
    SystemId,
    economy::Commodity,
    travel::{Location, PlayerId, Ship, ShipId, TravelState},
};
use std::collections::HashSet;

impl TravelState {
    pub fn fleet_for_ship(&self, ship: ShipId) -> Result<&Fleet, FleetError> {
        self.fleets
            .iter()
            .find(|fleet| fleet.ships.contains(&ship))
            .ok_or(FleetError::UnknownFleet)
    }

    /// All members must share one stationary location in any coordinate space.
    pub fn fleet_position(&self, id: FleetId, player: PlayerId) -> Result<&Location, FleetError> {
        let fleet = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == id)
            .ok_or(FleetError::UnknownFleet)?;
        if fleet.owner != player {
            return Err(FleetError::NotOwner);
        }
        let mut location = None;
        for id in &fleet.ships {
            let ship = self.ship(*id).map_err(|_| FleetError::NotOwner)?;
            if ship.owner != player {
                return Err(FleetError::NotOwner);
            }
            if !ship.location.stationary()
                || location.is_some_and(|previous| previous != &ship.location)
            {
                return Err(FleetError::DifferentLocations);
            }
            location = Some(&ship.location);
        }
        location.ok_or(FleetError::UnknownFleet)
    }

    pub fn fleet_location(&self, id: FleetId, player: PlayerId) -> Result<SystemId, FleetError> {
        match self.fleet_position(id, player)? {
            Location::Docked(system) => Ok(*system),
            _ => Err(FleetError::NotDocked),
        }
    }

    pub fn split_fleet(
        &mut self,
        player: PlayerId,
        id: FleetId,
        ships: &[ShipId],
    ) -> Result<FleetId, FleetError> {
        self.fleet_position(id, player)?;
        if self.next_fleet_id >= crate::npc::NPC_ID_BASE {
            return Err(FleetError::ArithmeticOverflow);
        }
        let index = self
            .fleets
            .iter()
            .position(|fleet| fleet.id == id)
            .ok_or(FleetError::UnknownFleet)?;
        let selection: HashSet<_> = ships.iter().copied().collect();
        if selection.is_empty()
            || selection.len() != ships.len()
            || selection.len() >= self.fleets[index].ships.len()
            || ships
                .iter()
                .any(|ship| !self.fleets[index].ships.contains(ship))
        {
            return Err(FleetError::InvalidSelection);
        }
        let next = self
            .next_fleet_id
            .checked_add(1)
            .ok_or(FleetError::ArithmeticOverflow)?;
        let new_id = FleetId(self.next_fleet_id);
        self.fleets[index]
            .ships
            .retain(|ship| !selection.contains(ship));
        self.fleets.push(Fleet {
            id: new_id,
            name: format!("Fleet {}", new_id.0),
            owner: player,
            ships: ships.to_vec(),
        });
        self.next_fleet_id = next;
        Ok(new_id)
    }

    pub fn merge_fleets(
        &mut self,
        player: PlayerId,
        source: FleetId,
        target: FleetId,
    ) -> Result<(), FleetError> {
        if source == target {
            return Err(FleetError::SameFleet);
        }
        if self.fleet_position(source, player)? != self.fleet_position(target, player)? {
            return Err(FleetError::DifferentLocations);
        }
        let source_index = self
            .fleets
            .iter()
            .position(|fleet| fleet.id == source)
            .ok_or(FleetError::UnknownFleet)?;
        let members = self.fleets[source_index].ships.clone();
        let target_fleet = self
            .fleets
            .iter_mut()
            .find(|fleet| fleet.id == target)
            .ok_or(FleetError::UnknownFleet)?;
        target_fleet.ships.extend(members);
        self.fleets.remove(source_index);
        Ok(())
    }

    pub fn add_ship(
        &mut self,
        player: PlayerId,
        fleet: FleetId,
        spec: &ShipSpec,
    ) -> Result<ShipId, FleetError> {
        let location = self.fleet_position(fleet, player)?.clone();
        if self.ships.len() >= 64 || self.next_ship_id >= crate::npc::NPC_ID_BASE {
            return Err(FleetError::ShipLimit);
        }
        let next = self
            .next_ship_id
            .checked_add(1)
            .ok_or(FleetError::ArithmeticOverflow)?;
        let index = self
            .fleets
            .iter()
            .position(|entry| entry.id == fleet)
            .ok_or(FleetError::UnknownFleet)?;
        let id = ShipId(self.next_ship_id);
        self.ships.push(Ship {
            id,
            owner: player,
            name: format!("{} {}", spec.kind.label(), id.0),
            kind: spec.kind,
            cargo: [0; 6],
            cargo_capacity: spec.cargo_capacity,
            base_cargo_capacity: spec.cargo_capacity,
            upgraded: false,
            fuel: spec.fuel_capacity,
            fuel_capacity: spec.fuel_capacity,
            range_ly: spec.range_ly,
            location,
            hull_damage: 0,
        });
        self.fleets[index].ships.push(id);
        self.next_ship_id = next;
        Ok(id)
    }

    pub fn transfer_cargo(
        &mut self,
        player: PlayerId,
        source: ShipId,
        target: ShipId,
        commodity: Commodity,
        quantity: u32,
    ) -> Result<(), FleetError> {
        if source == target || quantity == 0 {
            return Err(FleetError::InvalidQuantity);
        }
        let origin = self.ship(source).map_err(|_| FleetError::NotOwner)?;
        let destination = self.ship(target).map_err(|_| FleetError::NotOwner)?;
        if origin.owner != player || destination.owner != player {
            return Err(FleetError::NotOwner);
        }
        if !origin.location.stationary() || origin.location != destination.location {
            return Err(FleetError::DifferentLocations);
        }
        let remaining = origin.cargo[commodity.index()]
            .checked_sub(quantity)
            .ok_or(FleetError::MissingCargo)?;
        if destination.cargo_used() + u64::from(quantity) > u64::from(destination.cargo_capacity) {
            return Err(FleetError::CargoFull);
        }
        let received = destination.cargo[commodity.index()]
            .checked_add(quantity)
            .ok_or(FleetError::ArithmeticOverflow)?;
        self.ship_mut(source)
            .map_err(|_| FleetError::NotOwner)?
            .cargo[commodity.index()] = remaining;
        self.ship_mut(target)
            .map_err(|_| FleetError::NotOwner)?
            .cargo[commodity.index()] = received;
        Ok(())
    }
}
