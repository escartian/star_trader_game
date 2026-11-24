use super::{Trader, market_location};
use crate::{
    StarSystem, SystemId,
    economy::{Commodity, Economy, EconomyError, TradeSide},
    factions::Relation,
    fleet::FleetMovementError,
    planet::Danger,
    travel::TravelState,
};

/// Market systems an NPC trader weighs when choosing its next haul: the
/// nearest ones to where it stands, its own included.
pub const ROUTE_SYSTEMS: usize = 6;

/// How a faction trader values a haul into its own or allied space, as a
/// percentage of the real profit.
pub const FRIENDLY_PROFIT_PERCENT: u64 = 125;

impl Economy {
    /// Only called once per committed global run. Each trader gets one independent
    /// wallet; all payments still reach the same finite public market treasuries.
    pub fn advance_traders(&mut self, systems: &[StarSystem]) -> Result<(), FleetMovementError> {
        self.advance_traders_hunted(systems, None)
    }

    /// [`Self::advance_traders`], with the player's raider bands (`hunters`)
    /// striking the traders they stalk (see `band_raids`).
    pub fn advance_traders_hunted(
        &mut self,
        systems: &[StarSystem],
        mut hunters: Option<&mut TravelState>,
    ) -> Result<(), FleetMovementError> {
        let mut traders = self.traders.clone();
        traders.sort_by_key(|trader| trader.id);
        for trader in &mut traders {
            let refitting = trader.refit_runs > 0;
            if trader.state.navigation.orders.is_empty() && !refitting {
                self.decide(trader, systems)?;
            }
            let player = trader.state.ships[0].owner;
            if trader.state.navigation.orders.is_empty() {
                trader.state.hold_fleet(player, trader.state.fleets[0].id)?;
            }
            let underway = !trader.state.navigation.orders.is_empty();
            trader.state.execute_navigation_run(player, systems)?;
            trader.escort_runs = trader.escort_runs.saturating_sub(1);
            if refitting {
                trader.refit_runs -= 1;
                trader.last_action = match trader.refit_runs {
                    0 => "Refitted after a raid; back in service".into(),
                    left => format!("Refitting after a raid ({left} runs left)"),
                };
                continue;
            }
            if trader.state.navigation.orders.is_empty() && trader.state.ships[0].cargo_used() > 0 {
                trader.last_action = "Arrived with cargo; preparing local sale".into();
            }
            if let Some(hunters) = hunters.as_deref_mut()
                && self.band_raid(trader, hunters, systems)?
            {
                continue;
            }
            if underway {
                self.raid(trader, systems)?;
            }
        }
        self.traders = traders;
        Ok(())
    }

    fn decide(
        &mut self,
        trader: &mut Trader,
        systems: &[StarSystem],
    ) -> Result<(), FleetMovementError> {
        for good in Commodity::ALL {
            let quantity = trader.state.ships[0].cargo[good.index()];
            if quantity > 0 {
                // Try smaller orders when local funds/storage cannot cover the lot.
                for amount in (1..=quantity).rev() {
                    if self
                        .trader_trade(trader, good, amount, TradeSide::Sell)
                        .is_ok()
                    {
                        trader.last_action = format!("Sold {amount} {}", good.label());
                        break;
                    }
                }
            }
        }
        if trader.state.ships[0].cargo_used() > 0 {
            trader.last_action = "Waiting for buyer funds or storage".into();
            return Ok(());
        }
        if trader.state.ships[0].hull_damage > 0 {
            // Raid damage is repaired at the first market, paid from the wallet.
            let player_wallet = std::mem::replace(&mut self.credits, trader.credits);
            let _ = self.repair(&mut trader.state.ships[0]);
            trader.credits = self.credits;
            self.credits = player_wallet;
        }
        let missing = trader.state.ships[0].fuel_capacity - trader.state.ships[0].fuel;
        if missing > 0 {
            let player_wallet = std::mem::replace(&mut self.credits, trader.credits);
            let result = self.refuel(&mut trader.state.ships[0], missing);
            trader.credits = self.credits;
            self.credits = player_wallet;
            if result.is_err() {
                trader.last_action = "Waiting for refuelling funds".into();
                return Ok(());
            }
        }
        // Profits beyond a trading reserve buy gear and escorts.
        let outfitted = self.outfit(trader);
        let Some((market, good, quantity)) = self.best_route(trader, systems) else {
            trader.last_action = "Holding: no profitable route".into();
            return Ok(());
        };
        if self
            .trader_trade(trader, good, quantity, TradeSide::Buy)
            .is_err()
        {
            trader.last_action = "Local stock changed; holding".into();
            return Ok(());
        }
        let ship = &trader.state.ships[0];
        trader.state.plot_destination(
            ship.owner,
            ship.id,
            market.system,
            Some(market.body),
            systems,
        )?;
        // Players read this in the traffic roster, so name the destination body.
        let destination = systems
            .iter()
            .find(|system| system.id == market.system)
            .and_then(|system| system.bodies.iter().find(|body| body.id == market.body))
            .map_or_else(
                || format!("system {} / body {}", market.system.0, market.body.0),
                |body| body.name.clone(),
            );
        trader.last_action = format!("Hauling {quantity} {} to {destination}", good.label());
        if let Some(news) = outfitted {
            trader.last_action = format!("{news}. {}", trader.last_action);
        }
        Ok(())
    }

    fn trader_trade(
        &mut self,
        trader: &mut Trader,
        good: Commodity,
        quantity: u32,
        side: TradeSide,
    ) -> Result<(), EconomyError> {
        // Restore the player wallet before returning even if a trade rejects.
        let player_wallet = std::mem::replace(&mut self.credits, trader.credits);
        let result = self.trade(&mut trader.state.ships[0], good, quantity, side, 0);
        trader.credits = self.credits;
        self.credits = player_wallet;
        // A chartered port pays the (single) player its share of NPC trades.
        let quote = result?;
        if self.charter_holder(quote.market).is_some() {
            self.pay_port_share(quote.market, quote.total);
        }
        Ok(())
    }

    /// The most profitable haul from here, quoted against the trader's own
    /// wallet (swapped in for the search, then restored).
    fn best_route(
        &mut self,
        trader: &Trader,
        systems: &[StarSystem],
    ) -> Option<(crate::economy::MarketId, Commodity, u32)> {
        let player_wallet = std::mem::replace(&mut self.credits, trader.credits);
        let best = self.route_search(trader, systems);
        self.credits = player_wallet;
        best
    }

    /// [`Self::best_route`]'s search; `self.credits` holds the trader's wallet.
    /// The trader's personality (`config/npc.json`) sets the load, the least
    /// margin, and how it treats dangerous ports. A faction trader never sails
    /// for a rival's port (a loyalist only for its own or allied ones) and
    /// values a haul into its own or allied space at
    /// [`FRIENDLY_PROFIT_PERCENT`] of its profit.
    fn route_search(
        &self,
        trader: &Trader,
        systems: &[StarSystem],
    ) -> Option<(crate::economy::MarketId, Commodity, u32)> {
        let ship = &trader.state.ships[0];
        let temper = self.rules.personality_catalog.get(trader.personality);
        let origin = self.current_market(ship).ok()?.id;
        let nearby = self.nearby_market_systems(origin.system, systems);
        let mut best = None;
        let mut best_profit = 0_u64;
        for market in &self.markets {
            if market.id == origin || !nearby.contains(&market.id.system) {
                continue;
            }
            let body = systems
                .iter()
                .find(|system| system.id == market.id.system)?
                .bodies
                .iter()
                .find(|body| body.id == market.id.body)?;
            let dangerous = body.planet.as_ref().is_some_and(|profile| {
                matches!(
                    profile.danger,
                    Danger::Dangerous | Danger::VeryDangerous | Danger::Insidious
                )
            });
            if dangerous && temper.avoids_danger {
                continue;
            }
            let welcome = match (trader.affiliation, self.factions.owner(market.id.system)) {
                (Some(own), Some(holder)) if own == holder => Relation::Allied,
                (Some(own), Some(holder)) => self.factions.relation(own, holder),
                _ => Relation::Neutral,
            };
            let outsider = trader.affiliation.is_some() && welcome != Relation::Allied;
            if welcome == Relation::Rival || (temper.friendly_only && outsider) {
                continue;
            }
            let Ok(course) = trader.state.plan_destination(
                ship.owner,
                ship.id,
                market.id.system,
                Some(market.id.body),
                systems,
            ) else {
                continue;
            };
            let fuel = course
                .legs
                .iter()
                .map(|leg| u64::from(leg.fuel_per_ship))
                .sum::<u64>();
            for good in Commodity::ALL {
                let quantity = temper.haul_units.min(ship.cargo_capacity);
                let Ok(buy) = self.quote(ship, good, quantity, TradeSide::Buy, 0) else {
                    continue;
                };
                let mut arrival = ship.clone();
                arrival.location = market_location(market.id, systems);
                arrival.cargo[good.index()] = quantity;
                let Ok(sell) = self.quote(&arrival, good, quantity, TradeSide::Sell, 0) else {
                    continue;
                };
                let cost = buy.total.0 + fuel * self.fuel_price().0;
                let Some(profit) =
                    sell.total.0.checked_sub(cost).filter(|profit| {
                        *profit > cost.saturating_mul(temper.margin_percent) / 100
                    })
                else {
                    continue;
                };
                let mut valued = profit;
                if welcome == Relation::Allied {
                    valued = valued.saturating_mul(FRIENDLY_PROFIT_PERCENT) / 100;
                }
                if dangerous {
                    valued = valued.saturating_mul(temper.danger_profit_percent) / 100;
                }
                if valued > best_profit {
                    best_profit = valued;
                    best = Some((market.id, good, quantity));
                }
            }
        }
        best
    }

    /// The [`ROUTE_SYSTEMS`] market systems nearest `origin` (itself
    /// included), by straight-line distance; ties go to the lower id. NPC
    /// traders only weigh hauls between these, which keeps a run's route
    /// planning bounded however large the galaxy.
    fn nearby_market_systems(&self, origin: SystemId, systems: &[StarSystem]) -> Vec<SystemId> {
        let Some(from) = systems.iter().find(|system| system.id == origin) else {
            return vec![origin];
        };
        let mut candidates: Vec<(u128, SystemId)> = systems
            .iter()
            .filter(|system| {
                self.markets
                    .iter()
                    .any(|market| market.id.system == system.id)
            })
            .map(|system| {
                let offset = |a: i64, b: i64| i128::from(a - b).unsigned_abs().pow(2);
                let (to, at) = (&system.position, &from.position);
                (
                    offset(to.x, at.x) + offset(to.y, at.y) + offset(to.z, at.z),
                    system.id,
                )
            })
            .collect();
        candidates.sort_by_key(|(distance, id)| (*distance, id.0));
        candidates
            .into_iter()
            .take(ROUTE_SYSTEMS)
            .map(|(_, id)| id)
            .collect()
    }
}
