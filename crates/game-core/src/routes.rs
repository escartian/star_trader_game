//! Repeating trade routes. A route fleet loops between two markets: at each stop it
//! sells its cargo, refuels, buys the most profitable cargo for the other stop and
//! plots the trip there. Route legs are ordinary courses, so they roll contacts and
//! pause for decisions like any fleet. A stop with nothing profitable to carry
//! pauses the route; one that cannot continue (for example, no money for fuel)
//! ends it. Either way the fleet then needs orders, which stops an Advance.
mod cargo;
#[cfg(test)]
mod tests;

use crate::{
    StarSystem,
    economy::{Commodity, Credits, Economy, MarketId, TradeSide},
    fleet::{FleetError, FleetId, FleetMovementError},
    navigation::{NavigationError, NavigationEvent},
    travel::{PlayerId, Ship, ShipId, TravelState},
};
use serde::{Deserialize, Serialize};

/// A fleet's standing order to loop between two markets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeRoute {
    pub fleet: FleetId,
    /// The markets the fleet alternates between; always two different markets.
    pub stops: [MarketId; 2],
    /// Idle at a stop because nothing was profitable to carry; resuming re-checks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub paused: bool,
}

/// How a stop ended.
enum Stop {
    /// Traded and plotted the next leg; carries the log summary.
    Departed(String),
    /// Nothing aboard and nothing profitable to buy, so no leg was plotted.
    Idle(String),
}

impl TravelState {
    pub fn trade_route(&self, fleet: FleetId) -> Option<&TradeRoute> {
        self.navigation
            .routes
            .iter()
            .find(|route| route.fleet == fleet)
    }

    /// Removes a fleet's route; returns whether it had one.
    pub fn end_trade_route(&mut self, fleet: FleetId) -> bool {
        let before = self.navigation.routes.len();
        self.navigation.routes.retain(|route| route.fleet != fleet);
        self.navigation.routes.len() != before
    }
}

/// Starts (or replaces, or resumes) a route between the fleet's current market and
/// `destination`. Cargo already aboard is kept; the remaining space takes the most
/// profitable cargo for `destination`, and the first leg is plotted. No time passes.
/// Rejected when the fleet would leave with an empty hold.
pub fn start_route(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    fleet: FleetId,
    destination: MarketId,
    systems: &[StarSystem],
) -> Result<NavigationEvent, FleetMovementError> {
    let lead = lead_ship(state, player, fleet)?;
    if state
        .navigation
        .order(fleet)
        .is_some_and(|order| order.started)
    {
        return Err(NavigationError::Underway.into());
    }
    let here = economy
        .current_market(state.ship(lead)?)
        .map_err(|_| NavigationError::NotAtMarket)?
        .id;
    if here == destination || economy.market(destination).is_err() {
        return Err(NavigationError::InvalidRouteStop.into());
    }
    let Stop::Departed(summary) =
        visit_stop(state, economy, player, fleet, destination, false, systems)?
    else {
        return Err(NavigationError::NothingProfitable.into());
    };
    state.end_trade_route(fleet);
    state.navigation.routes.push(TradeRoute {
        fleet,
        stops: [here, destination],
        paused: false,
    });
    Ok(NavigationEvent::RouteServiced { fleet, summary })
}

/// Ends a fleet's route at the player's request. A leg already plotted or underway
/// still completes; the fleet then needs orders.
pub fn cancel_route(
    state: &mut TravelState,
    player: PlayerId,
    fleet: FleetId,
) -> Result<NavigationEvent, FleetMovementError> {
    lead_ship(state, player, fleet)?;
    if !state.end_trade_route(fleet) {
        return Err(NavigationError::NoRoute.into());
    }
    Ok(NavigationEvent::RouteEnded {
        fleet,
        reason: "Route ended; the fleet completes its current leg, then waits for orders.".into(),
    })
}

/// After a run: each route fleet that is idle at one of its stops trades there and
/// sets off for the other stop. A fleet that cannot continue leaves its route.
pub fn service_routes(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let mut routes = state.navigation.routes.clone();
    routes.sort_by_key(|route| route.fleet.0);
    let mut events = Vec::new();
    for route in routes {
        let fleet = route.fleet;
        let contact = state
            .navigation
            .encounters
            .iter()
            .any(|encounter| encounter.fleet == fleet);
        if route.paused || contact || state.navigation.order(fleet).is_some() {
            continue;
        }
        let lead = lead_ship(state, player, fleet)?;
        let here = economy
            .current_market(state.ship(lead)?)
            .ok()
            .map(|market| market.id);
        let next = if here == Some(route.stops[0]) {
            route.stops[1]
        } else if here == Some(route.stops[1]) {
            route.stops[0]
        } else {
            state.end_trade_route(fleet);
            events.push(NavigationEvent::RouteEnded {
                fleet,
                reason: "Route ended: the fleet is no longer at one of its stops.".into(),
            });
            continue;
        };
        match visit_stop(state, economy, player, fleet, next, true, systems) {
            Ok(Stop::Departed(summary)) => {
                events.push(NavigationEvent::RouteServiced { fleet, summary });
            }
            Ok(Stop::Idle(summary)) => {
                if let Some(route) = state
                    .navigation
                    .routes
                    .iter_mut()
                    .find(|route| route.fleet == fleet)
                {
                    route.paused = true;
                }
                events.push(NavigationEvent::RoutePaused {
                    fleet,
                    reason: summary,
                });
            }
            Err(error) => {
                state.end_trade_route(fleet);
                events.push(NavigationEvent::RouteEnded {
                    fleet,
                    reason: format!("Route ended: {error}"),
                });
            }
        }
    }
    Ok(events)
}

fn lead_ship(
    state: &TravelState,
    player: PlayerId,
    fleet: FleetId,
) -> Result<ShipId, FleetMovementError> {
    let record = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .ok_or(FleetError::UnknownFleet)?;
    if record.owner != player {
        return Err(FleetError::NotOwner.into());
    }
    Ok(*record.ships.first().ok_or(FleetError::InvalidSelection)?)
}

/// One stop: optionally sell everything aboard, refuel every ship, then, once the
/// next leg is known to be reachable, buy the planned cargo and plot the leg.
/// Ordered so a failure leaves the fleet with sales made and tanks filled, but no
/// cargo bought for a trip it cannot make. A fleet left with an empty hold stays
/// put instead of making an unpaid trip.
fn visit_stop(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    fleet: FleetId,
    next: MarketId,
    sell: bool,
    systems: &[StarSystem],
) -> Result<Stop, FleetMovementError> {
    let lead = lead_ship(state, player, fleet)?;
    let ships = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .ok_or(FleetError::UnknownFleet)?
        .ships
        .clone();
    let here = economy
        .current_market(state.ship(lead)?)
        .map_err(|_| NavigationError::NotAtMarket)?
        .id;
    let revision = state.revision;
    let mut log = StopLog::default();
    for &id in &ships {
        let ship = state.ship_mut(id)?;
        if sell {
            for good in Commodity::ALL {
                let quantity = economy.max_trade_quantity(ship, good, TradeSide::Sell);
                if quantity > 0
                    && let Ok(quote) =
                        economy.trade(ship, good, quantity, TradeSide::Sell, revision)
                {
                    log.sold[good.index()] += quantity;
                    log.income += quote.total.0;
                }
            }
        }
        if let Ok(units) = economy.max_refuel_units(ship)
            && economy.refuel(ship, units).is_ok()
        {
            log.fuel += units;
            log.fuel_cost += u64::from(units) * economy.rules.fuel_unit_price.0;
        }
    }
    state.plan_destination(player, lead, next.system, Some(next.body), systems)?;
    let space = ships
        .iter()
        .map(|id| state.ship(*id).map(free_space))
        .sum::<Result<u64, _>>()?;
    let mut plan = cargo::plan(
        economy
            .market(here)
            .map_err(|_| NavigationError::NotAtMarket)?,
        economy
            .market(next)
            .map_err(|_| NavigationError::InvalidRouteStop)?,
        space,
        economy.credits.0,
    );
    for &id in &ships {
        let ship = state.ship_mut(id)?;
        for good in Commodity::ALL {
            let room = u32::try_from(free_space(ship)).unwrap_or(u32::MAX);
            let quantity = plan[good.index()].min(room);
            if quantity > 0
                && let Ok(quote) = economy.trade(ship, good, quantity, TradeSide::Buy, revision)
            {
                plan[good.index()] -= quantity;
                log.bought[good.index()] += quantity;
                log.spent += quote.total.0;
            }
        }
    }
    let (here, next_label) = (market_label(here, systems), market_label(next, systems));
    let carrying = ships
        .iter()
        .map(|id| state.ship(*id).map(Ship::cargo_used))
        .sum::<Result<u64, _>>()?;
    if carrying == 0 {
        return Ok(Stop::Idle(log.summary(&here, &next_label, false)));
    }
    state.plot_destination(player, lead, next.system, Some(next.body), systems)?;
    Ok(Stop::Departed(log.summary(&here, &next_label, true)))
}

fn free_space(ship: &Ship) -> u64 {
    u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used())
}

fn market_label(market: MarketId, systems: &[StarSystem]) -> String {
    systems
        .iter()
        .find(|system| system.id == market.system)
        .and_then(|system| system.bodies.iter().find(|body| body.id == market.body))
        .map_or_else(
            || format!("system {} / body {}", market.system.0, market.body.0),
            |body| body.name.clone(),
        )
}

/// What one stop traded, for the event log. Money is in hundredths of a credit.
#[derive(Default)]
struct StopLog {
    sold: [u32; 6],
    income: u64,
    bought: [u32; 6],
    spent: u64,
    fuel: u32,
    fuel_cost: u64,
}

impl StopLog {
    /// `departed` is false when the stop paused the route instead of plotting a leg.
    fn summary(&self, here: &str, next: &str, departed: bool) -> String {
        let goods = |units: &[u32; 6]| {
            Commodity::ALL
                .into_iter()
                .filter(|good| units[good.index()] > 0)
                .map(|good| format!("{} {}", units[good.index()], good.label()))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut parts = Vec::new();
        if self.sold.iter().any(|units| *units > 0) {
            parts.push(format!(
                "sold {} for {}",
                goods(&self.sold),
                Credits(self.income)
            ));
        }
        if self.fuel > 0 {
            parts.push(format!(
                "refuelled {} for {}",
                self.fuel,
                Credits(self.fuel_cost)
            ));
        }
        if self.bought.iter().any(|units| *units > 0) {
            parts.push(format!(
                "bought {} for {}",
                goods(&self.bought),
                Credits(self.spent)
            ));
        } else if departed {
            parts.push("found no profitable cargo".into());
        }
        if departed {
            format!("At {here}: {}; bound for {next}.", parts.join("; "))
        } else {
            parts.push(format!(
                "nothing profitable to carry to {next}, so the route is paused"
            ));
            format!("At {here}: {}.", parts.join("; "))
        }
    }
}
