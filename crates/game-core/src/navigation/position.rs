//! Integer course progress for map adapters; never advances the simulation.
use crate::{
    StarSystem,
    movement::{SpacePosition, local_position},
    travel::{Location, ShipId, TravelState},
};

pub fn space_position(location: &Location, systems: &[StarSystem]) -> Option<SpacePosition> {
    if let Location::DeepSpace(position) = location {
        return Some(*position);
    }
    let system = location.system()?;
    let center =
        SpacePosition::from_galaxy(systems.iter().find(|known| known.id == system)?.position)
            .ok()?;
    let local = local_position(location, system, systems).ok()?;
    Some(SpacePosition {
        x_km: center.x_km.checked_add(local.x_km)?,
        y_km: center.y_km.checked_add(local.y_km)?,
        z_km: center.z_km.checked_add(local.z_km)?,
    })
}

impl TravelState {
    pub fn navigation_position(
        &self,
        ship: ShipId,
        systems: &[StarSystem],
    ) -> Option<SpacePosition> {
        let ship = self.ship(ship).ok()?;
        let Location::InTransit(fleet) = ship.location else {
            return space_position(&ship.location, systems);
        };
        let order = self.navigation.order(fleet)?;
        let start = space_position(&order.plan.origin, systems)?;
        let end = space_position(&order.plan.destination, systems)?;
        let elapsed = order
            .plan
            .duration_ticks
            .checked_sub(order.remaining_ticks)?;
        let interpolate = |a: i64, b: i64| {
            i64::try_from(
                i128::from(a)
                    + (i128::from(b) - i128::from(a)) * i128::from(elapsed)
                        / i128::from(order.plan.duration_ticks),
            )
            .ok()
        };
        Some(SpacePosition {
            x_km: interpolate(start.x_km, end.x_km)?,
            y_km: interpolate(start.y_km, end.y_km)?,
            z_km: interpolate(start.z_km, end.z_km)?,
        })
    }
}
