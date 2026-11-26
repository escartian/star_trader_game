//! Fleet trading: one market order shared by the ships of a fleet at the
//! same port, split among their holds in a chosen [`LoadOrder`].
//!
//! The order is priced and checked as one bulk trade against a *pooled hold*
//! (`pooled`): every partner's cargo and capacity added together, so stock,
//! credits, space and the price curve are judged exactly as for one big
//! ship. Then `plan` decides which ship gives or takes each unit and `apply`
//! moves them. Cargo stays per ship: a ship destroyed in battle loses its own
//! hold, which is why the load order matters.
//!
//! Selling walks the same order backwards: units leave the ships a buy would
//! fill last (the weakest or smallest first), or the fullest ship for an even
//! spread.
use crate::economy::{Commodity, Economy, EconomyError, TradeSide};
use crate::fleet::ShipCatalog;
use crate::travel::{Ship, ShipId, TravelError, TravelState};
use serde::{Deserialize, Serialize};

/// How a fleet order is split among its ships' holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadOrder {
    /// Each ship ends up filled to about the same share of its hold, so a
    /// lost ship costs the least cargo.
    #[default]
    Even,
    /// The ship with the most hull and shields left fills first.
    Toughest,
    /// The ship with the largest hold fills first: the fewest ships carry cargo.
    Largest,
    /// Only the selected ship trades.
    Selected,
}

impl LoadOrder {
    pub const ALL: [LoadOrder; 4] = [Self::Even, Self::Toughest, Self::Largest, Self::Selected];

    pub fn label(self) -> &'static str {
        match self {
            Self::Even => "Spread evenly",
            Self::Toughest => "Toughest ship first",
            Self::Largest => "Largest hold first",
            Self::Selected => "Selected ship only",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Even => {
                "Every ship in the fleet at this port is filled to the same share of its hold. \
                 Losing one ship in battle costs the least cargo."
            }
            Self::Toughest => {
                "Cargo goes to the ship with the most hull and shields left first, and is sold \
                 from the weakest first."
            }
            Self::Largest => {
                "Cargo goes to the largest hold first, and is sold from the smallest first, so \
                 the fewest ships carry it."
            }
            Self::Selected => "Only the selected ship buys and sells.",
        }
    }
}

/// The ships sharing an order placed from `ship`: its fleet's members at the
/// same port, in fleet order (just `ship` for `Selected`, or when it is not
/// at a market).
pub fn partners(
    state: &TravelState,
    economy: &Economy,
    ship: ShipId,
    order: LoadOrder,
) -> Result<Vec<ShipId>, TravelError> {
    let lead = state.ship(ship)?;
    let Ok(port) = economy.current_market(lead).map(|market| market.id) else {
        return Ok(vec![ship]);
    };
    if order == LoadOrder::Selected {
        return Ok(vec![ship]);
    }
    let Ok(fleet) = state.fleet_for_ship(ship) else {
        return Ok(vec![ship]);
    };
    let mut ships = Vec::new();
    for id in &fleet.ships {
        let member = state.ship(*id)?;
        if member.owner == lead.owner
            && economy
                .current_market(member)
                .is_ok_and(|market| market.id == port)
        {
            ships.push(*id);
        }
    }
    Ok(ships)
}

/// A stand-in ship for the partners' holds: the first partner with every
/// partner's cargo and capacity added together. Quote and trade against it,
/// then `apply` the `plan`.
pub fn pooled(state: &TravelState, partners: &[ShipId]) -> Result<Ship, TravelError> {
    let first = partners.first().ok_or(TravelError::UnknownShip)?;
    let mut pool = state.ship(*first)?.clone();
    pool.cargo = [0; 6];
    pool.cargo_capacity = 0;
    for id in partners {
        let ship = state.ship(*id)?;
        for (total, units) in pool.cargo.iter_mut().zip(ship.cargo) {
            *total = total.saturating_add(units);
        }
        pool.cargo_capacity = pool.cargo_capacity.saturating_add(ship.cargo_capacity);
    }
    Ok(pool)
}

/// Hull points left plus shields: what the `Toughest` order ranks by.
fn toughness(ship: &Ship, catalog: &ShipCatalog) -> u32 {
    catalog.spec(ship.kind).map_or(0, |spec| {
        spec.combat.hull.saturating_sub(ship.hull_damage) + spec.combat.shields
    })
}

/// Which ship gives or takes how many units of `quantity` (ships with none
/// left out). Assumes the pooled order was accepted, so the partners have
/// the room (buying) or the goods (selling); any shortfall is left unplanned.
pub fn plan(
    state: &TravelState,
    catalog: &ShipCatalog,
    partners: &[ShipId],
    commodity: Commodity,
    quantity: u32,
    side: TradeSide,
    order: LoadOrder,
) -> Vec<(ShipId, u32)> {
    let mut ships: Vec<&Ship> = partners
        .iter()
        .filter_map(|id| state.ship(*id).ok())
        .collect();
    // Units each ship can give or take, and its fill now (cargo used).
    let room = |ship: &Ship| match side {
        TradeSide::Buy => u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used()) as u32,
        TradeSide::Sell => ship.cargo[commodity.index()],
    };
    let mut moved = vec![0_u32; ships.len()];
    if order == LoadOrder::Even {
        for _ in 0..quantity {
            let fill = |index: usize| {
                let ship = ships[index];
                let used = match side {
                    TradeSide::Buy => ship.cargo_used() + u64::from(moved[index]),
                    TradeSide::Sell => ship.cargo_used() - u64::from(moved[index]),
                };
                (used, u64::from(ship.cargo_capacity.max(1)))
            };
            // Buying fills the emptiest share; selling drains the fullest.
            let candidates = (0..ships.len()).filter(|index| moved[*index] < room(ships[*index]));
            let pick = match side {
                TradeSide::Buy => candidates.min_by(|a, b| {
                    let ((ua, ca), (ub, cb)) = (fill(*a), fill(*b));
                    (ua * cb).cmp(&(ub * ca))
                }),
                TradeSide::Sell => candidates.max_by(|a, b| {
                    let ((ua, ca), (ub, cb)) = (fill(*a), fill(*b));
                    (ua * cb).cmp(&(ub * ca)).then(b.cmp(a))
                }),
            };
            let Some(index) = pick else {
                break;
            };
            moved[index] += 1;
        }
    } else {
        let key = |ship: &Ship| match order {
            LoadOrder::Toughest => toughness(ship, catalog),
            _ => ship.cargo_capacity,
        };
        // Stable sort: equal ships keep fleet order.
        ships.sort_by(|a, b| match side {
            TradeSide::Buy => key(b).cmp(&key(a)),
            TradeSide::Sell => key(a).cmp(&key(b)),
        });
        let mut left = quantity;
        for (index, ship) in ships.iter().enumerate() {
            let take = room(ship).min(left);
            moved[index] = take;
            left -= take;
        }
    }
    ships
        .iter()
        .zip(moved)
        .filter(|(_, units)| *units > 0)
        .map(|(ship, units)| (ship.id, units))
        .collect()
}

/// Moves the planned units into (buying) or out of (selling) each ship. A
/// plan that does not fit (an unknown ship, too little room or cargo) fails
/// with the matching trade error; the host applies it to a candidate state
/// and publishes nothing on failure.
pub fn apply(
    state: &mut TravelState,
    split: &[(ShipId, u32)],
    commodity: Commodity,
    side: TradeSide,
) -> Result<(), EconomyError> {
    let shortfall = || match side {
        TradeSide::Buy => EconomyError::CargoFull,
        TradeSide::Sell => EconomyError::InsufficientCargo,
    };
    for (id, units) in split {
        let ship = state.ship_mut(*id).map_err(|_| shortfall())?;
        let free = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
        let hold = &mut ship.cargo[commodity.index()];
        *hold = match side {
            TradeSide::Buy if u64::from(*units) <= free => *hold + units,
            TradeSide::Sell => hold.checked_sub(*units).ok_or_else(shortfall)?,
            TradeSide::Buy => return Err(shortfall()),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests;
