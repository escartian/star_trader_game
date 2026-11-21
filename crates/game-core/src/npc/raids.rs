//! NPC traders meet raiders too. After each run in which a trader was under way
//! it may be attacked (`npc_raid_chance_percent`) by one scout, or two one time
//! in three. The fight is the same tactical battle the player plays,
//! auto-resolved for both sides, so the battle AI's retreats apply:
//! - fought off: the trader keeps its cargo;
//! - escaped: the trader dumped its cargo to get away;
//! - destroyed: cargo lost, and the trader is refitted at its home port, out of
//!   service for [`REFIT_RUNS`] runs.
//!
//! Damage sticks until the trader repairs at a market.
use super::{Trader, market_location};
use crate::{
    StarSystem,
    combat::{CombatantSetup, Side, tactical},
    economy::{Economy, MarketId},
    fleet::{FleetError, FleetMovementError, ShipKind},
};

/// Runs a destroyed trader spends being refitted before it trades again.
pub const REFIT_RUNS: u32 = 8;

impl Economy {
    /// After a run in which `trader` was under way: perhaps a raider attack.
    pub(super) fn raid(
        &mut self,
        trader: &mut Trader,
        systems: &[StarSystem],
    ) -> Result<(), FleetMovementError> {
        let roll = trader.state.navigation.rolls();
        if roll % 100 >= u64::from(self.rules.npc_raid_chance_percent) {
            return Ok(());
        }
        let raiders = if (roll >> 32).is_multiple_of(3) { 2 } else { 1 };
        let seed = trader.state.navigation.rolls();
        let hulls = vec![ShipKind::Scout; raiders];
        self.fight_raiders(trader, &hulls, seed, "raiders", systems)?;
        Ok(())
    }

    /// Auto-resolves `trader` against raider hulls `raiders` (`foe` names them
    /// in the trader's news) and applies the outcome. Returns the raider hulls
    /// destroyed.
    pub(super) fn fight_raiders(
        &mut self,
        trader: &mut Trader,
        raiders: &[ShipKind],
        seed: u64,
        foe: &str,
        systems: &[StarSystem],
    ) -> Result<Vec<ShipKind>, FleetMovementError> {
        let ship = trader.state.ships[0].clone();
        let profile = |kind| {
            self.rules
                .ship_catalog
                .spec(kind)
                .map(|spec| spec.combat.clone())
                .ok_or(FleetError::UnknownClass)
        };
        let mut setups = vec![CombatantSetup {
            name: trader.name.clone(),
            side: Side::Player,
            ship: Some(ship.id),
            kind: ship.kind,
            profile: profile(ship.kind)?,
            hull_damage: ship.hull_damage,
        }];
        for kind in raiders {
            setups.push(CombatantSetup {
                name: format!("Raider {}", kind.label()),
                side: Side::Hostile,
                ship: None,
                kind: *kind,
                profile: profile(*kind)?,
                hull_damage: 0,
            });
        }
        let result = tactical::auto_resolve(&setups, &self.rules.combat_catalog, seed)
            .map_err(|_| FleetError::InvalidSelection)?;
        let destroyed = result
            .combatants
            .iter()
            .filter(|unit| unit.side == Side::Hostile && unit.destroyed)
            .map(|unit| unit.kind)
            .collect();
        let me = &result.combatants[0];
        if me.destroyed {
            let lost = ship.cargo.iter().sum::<u32>();
            let port = self.refit(trader, systems);
            trader.last_action = format!(
                "Destroyed by {foe}; {lost} cargo lost. Refitting at {port} for {REFIT_RUNS} runs"
            );
            return Ok(destroyed);
        }
        trader.state.ships[0].hull_damage = me.final_damage;
        let hull = format!("hull {}/{}", me.hull - me.final_damage, me.hull);
        trader.last_action = match result.winner {
            Some(Side::Player) => format!("Fought off {foe} ({hull})"),
            _ => {
                let dumped = ship.cargo.iter().sum::<u32>();
                trader.state.ships[0].cargo = [0; 6];
                format!("Escaped {foe} by dumping {dumped} cargo ({hull})")
            }
        };
        Ok(destroyed)
    }

    /// Replaces a destroyed trader's ship at its home port: empty hold, full
    /// fuel, no damage, no course. Returns the port's name.
    pub(crate) fn refit(&self, trader: &mut Trader, systems: &[StarSystem]) -> String {
        let home = self.home_market(trader.id);
        let ship = &mut trader.state.ships[0];
        ship.cargo = [0; 6];
        ship.hull_damage = 0;
        ship.fuel = ship.fuel_capacity;
        if let Some(home) = home {
            ship.location = market_location(home, systems);
        }
        trader.state.navigation.orders.clear();
        trader.refit_runs = REFIT_RUNS;
        home.and_then(|home| {
            systems
                .iter()
                .find(|system| system.id == home.system)?
                .bodies
                .iter()
                .find(|body| body.id == home.body)
                .map(|body| body.name.clone())
        })
        .unwrap_or_else(|| "its home port".into())
    }

    /// The planet market a trader started at (see `spawn_traders`).
    fn home_market(&self, trader: u64) -> Option<MarketId> {
        let index = usize::try_from(trader.checked_sub(1)?).ok()?;
        self.markets
            .iter()
            .filter(|market| !market.is_station)
            .nth(index)
            .map(|market| market.id)
    }
}
