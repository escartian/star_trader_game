use super::{Personality, Trader, market_location};
use crate::{
    StarSystem,
    economy::{Commodity, Economy, EconomyError, TradeSide},
    fleet::FleetMovementError,
    planet::Danger,
};

impl Economy {
    /// Only called once per committed global run. Each trader gets one independent
    /// wallet; all payments still reach the same finite public market treasuries.
    pub fn advance_traders(&mut self, systems: &[StarSystem]) -> Result<(), FleetMovementError> {
        let mut traders = self.traders.clone();
        traders.sort_by_key(|trader| trader.id);
        for trader in &mut traders {
            if trader.state.navigation.orders.is_empty() {
                self.decide(trader, systems)?;
            }
            let player = trader.state.ships[0].owner;
            if trader.state.navigation.orders.is_empty() {
                trader.state.hold_fleet(player, trader.state.fleets[0].id)?;
            }
            trader.state.execute_navigation_run(player, systems)?;
            if trader.state.navigation.orders.is_empty() && trader.state.ships[0].cargo_used() > 0 {
                trader.last_action = "Arrived with cargo; preparing local sale".into();
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
        trader.last_action = format!(
            "Hauling {quantity} {} to system {} / body {}",
            good.label(),
            market.system.0,
            market.body.0
        );
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
        result.map(|_| ())
    }

    fn best_route(
        &self,
        trader: &Trader,
        systems: &[StarSystem],
    ) -> Option<(crate::economy::MarketId, Commodity, u32)> {
        let ship = &trader.state.ships[0];
        let origin = self.current_market(ship).ok()?.id;
        let mut quotes = self.clone();
        quotes.credits = trader.credits;
        let mut best = None;
        let mut best_profit = 0_u64;
        for market in &self.markets {
            if market.id == origin {
                continue;
            }
            let body = systems
                .iter()
                .find(|system| system.id == market.id.system)?
                .bodies
                .iter()
                .find(|body| body.id == market.id.body)?;
            if trader.personality == Personality::Cautious
                && body.planet.as_ref().is_some_and(|profile| {
                    matches!(
                        profile.danger,
                        Danger::Dangerous | Danger::VeryDangerous | Danger::Insidious
                    )
                })
            {
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
                let quantity = 4.min(ship.cargo_capacity);
                let Ok(buy) = quotes.quote(ship, good, quantity, TradeSide::Buy, 0) else {
                    continue;
                };
                let mut arrival = ship.clone();
                arrival.location = market_location(market.id, systems);
                arrival.cargo[good.index()] = quantity;
                let Ok(sell) = quotes.quote(&arrival, good, quantity, TradeSide::Sell, 0) else {
                    continue;
                };
                let cost = buy.total.0 + fuel * self.rules.fuel_unit_price.0;
                let threshold = if trader.personality == Personality::Cautious {
                    cost / 10
                } else {
                    0
                };
                if let Some(profit) = sell.total.0.checked_sub(cost)
                    && profit > threshold
                    && profit > best_profit
                {
                    best_profit = profit;
                    best = Some((market.id, good, quantity));
                }
            }
        }
        best
    }
}
