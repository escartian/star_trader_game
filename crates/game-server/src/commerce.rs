use super::Session;
use game_core::fleet_trade;
use game_core::travel::{PlayerId, TravelError};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn commerce(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let (ship, expected_revision) = match command {
            ClientCommand::Trade {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::FleetTrade {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::Refuel {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::UpgradeHold {
                ship,
                expected_revision,
            }
            | ClientCommand::RepairShip {
                ship,
                expected_revision,
            }
            | ClientCommand::FitGear {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::RefitShip {
                ship,
                expected_revision,
                ..
            }
            | ClientCommand::HireEscort {
                ship,
                expected_revision,
            }
            | ClientCommand::PayFine {
                ship,
                expected_revision,
            } => (ship, expected_revision),
            _ => unreachable!("only commerce commands reach this handler"),
        };
        self.state.authorize(player, ship)?;
        if expected_revision != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        // A candidate transaction spans credits, ship fuel, cargo and market stock.
        // Publish it only after every rule succeeds, preserving failure atomicity.
        let mut state = self.state.clone();
        let mut economy = self.economy.clone();
        let event = match command {
            ClientCommand::Trade {
                commodity,
                quantity,
                side,
                ..
            } => {
                let quote = economy.trade(
                    state.ship_mut(ship)?,
                    commodity,
                    quantity,
                    side,
                    expected_revision,
                )?;
                // Sales count toward unlocking trade routes.
                if side == game_core::economy::TradeSide::Sell {
                    state.progress.deliveries = state.progress.deliveries.saturating_add(1);
                }
                SessionEvent::Traded(quote)
            }
            ClientCommand::FleetTrade {
                commodity,
                quantity,
                side,
                order,
                ..
            } => {
                // Priced as one bulk order against the pooled hold, then split.
                let ships = fleet_trade::partners(&state, &economy, ship, order)?;
                let mut pool = fleet_trade::pooled(&state, &ships)?;
                let quote =
                    economy.trade(&mut pool, commodity, quantity, side, expected_revision)?;
                let catalog = &economy.rules.ship_catalog;
                let split =
                    fleet_trade::plan(&state, catalog, &ships, commodity, quantity, side, order);
                let planned: u32 = split.iter().map(|(_, units)| units).sum();
                if planned != quantity {
                    return Err(game_core::economy::EconomyError::CargoFull.into());
                }
                fleet_trade::apply(&mut state, &split, commodity, side)?;
                if side == game_core::economy::TradeSide::Sell {
                    state.progress.deliveries = state.progress.deliveries.saturating_add(1);
                }
                SessionEvent::FleetTraded { quote, split }
            }
            ClientCommand::Refuel { units, .. } => {
                economy.refuel(state.ship_mut(ship)?, units)?;
                SessionEvent::Refuelled { units }
            }
            ClientCommand::UpgradeHold { .. } => {
                economy.upgrade_hold(state.ship_mut(ship)?)?;
                SessionEvent::HoldUpgraded
            }
            ClientCommand::RepairShip { .. } => {
                let cost = economy.repair(state.ship_mut(ship)?)?;
                SessionEvent::Repaired { ship, cost }
            }
            ClientCommand::FitGear { gear, .. } => {
                let cost = economy.fit_gear(state.ship_mut(ship)?, gear)?;
                SessionEvent::GearFitted { ship, gear, cost }
            }
            ClientCommand::RefitShip {
                remove, install, ..
            } => {
                let (refund, cost) = economy.refit_ship(state.ship_mut(ship)?, remove, install)?;
                SessionEvent::ShipRefitted {
                    ship,
                    removed: remove,
                    installed: install,
                    refund,
                    cost,
                }
            }
            ClientCommand::HireEscort { .. } => {
                // The fleet's lead ship pays at its market; renewing restores
                // the full contract.
                let fleet = state.fleet_for_ship(ship)?.id;
                let lead = *state
                    .fleet_for_ship(ship)?
                    .ships
                    .first()
                    .ok_or(TravelError::UnknownShip)?;
                let cost = economy.hire_escort(state.ship(lead)?)?;
                let escorts = &mut state.navigation.escorts;
                escorts.retain(|escort| escort.fleet != fleet);
                escorts.push(game_core::navigation::Escort {
                    fleet,
                    runs_left: game_core::combat::gear::ESCORT_RUNS,
                });
                SessionEvent::EscortHired { cost }
            }
            ClientCommand::PayFine { .. } => {
                let (faction, cost) =
                    game_core::police::pay_fine_at_port(&mut economy, state.ship(ship)?)?;
                SessionEvent::FinePaid { faction, cost }
            }
            _ => unreachable!("command was validated above"),
        };
        state.revision = revision;
        self.state = state;
        self.economy = economy;
        Ok(self.snapshot(vec![event]))
    }
}
