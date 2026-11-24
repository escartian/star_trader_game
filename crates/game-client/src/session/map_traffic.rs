//! NPC traders as the maps draw them: where each one is (gliding after a
//! run, like own ships), how it stands toward you, whom it flies for and
//! which star system it is heading to. Display only.
use super::LocalSession;
use egui::Color32;
use game_core::{
    StarSystem, SystemId,
    fleet::ShipKind,
    movement::{MoveTarget, SpacePosition},
    npc::Trader,
    travel::Location,
};

/// One NPC trader on the system map.
#[derive(Clone, Debug)]
pub struct MapTrader {
    pub kind: ShipKind,
    /// Where to draw it: its berth while resting, else its gliding position
    /// as `InSystem` kilometres from the star.
    pub location: Location,
    /// Friend-or-foe ring (`theme::iff`).
    pub ring: Color32,
    /// Its faction's colour (grey if independent).
    pub pip: Color32,
}

impl LocalSession {
    /// Traders in (or gliding through) `system`, for its map.
    pub fn traders_for_map(&self, system: &StarSystem, systems: &[StarSystem]) -> Vec<MapTrader> {
        let Ok(center) = SpacePosition::from_galaxy(system.position) else {
            return Vec::new();
        };
        let half = self.state.movement_rules.system_half_extent_km as u64;
        let gliding = self.gliding();
        self.economy
            .traders
            .iter()
            .filter_map(|trader| {
                let ship = trader.state.ships.first()?;
                let resting = ship.location.stationary();
                let location = if resting && !gliding {
                    (ship.location.system() == Some(system.id)).then(|| ship.location.clone())?
                } else {
                    let at = self.trader_display_km(trader, systems)?;
                    let local = game_core::LocalPosition {
                        x_km: at.x_km - center.x_km,
                        y_km: at.y_km - center.y_km,
                        z_km: at.z_km - center.z_km,
                    };
                    let inside = [local.x_km, local.y_km, local.z_km]
                        .iter()
                        .all(|value| value.unsigned_abs() <= half);
                    if !inside {
                        return None;
                    }
                    // In flight, or gliding into a berth until the glide ends.
                    Location::InSystem {
                        system: system.id,
                        position: local,
                    }
                };
                Some(MapTrader {
                    kind: ship.kind,
                    location,
                    ring: self.trader_stance(trader).color(),
                    pip: self.trader_color(trader),
                })
            })
            .collect()
    }

    /// The star system a trader is bound for, when it is not the one it is in.
    pub fn trader_heading(&self, trader: &Trader) -> Option<SystemId> {
        let order = trader.state.navigation.orders.first()?;
        let last = order.remaining_waypoints.last().unwrap_or(&order.target);
        let bound = match last {
            MoveTarget::Body { system, .. }
            | MoveTarget::Local { system, .. }
            | MoveTarget::EnterSystem(system) => *system,
            MoveTarget::LeaveSystem(_) | MoveTarget::DeepSpace(_) => return None,
        };
        let here = trader.state.ships.first()?.location.system();
        (here != Some(bound)).then_some(bound)
    }
}

#[cfg(test)]
mod tests {
    use super::LocalSession;

    #[test]
    fn a_trader_resting_in_a_system_appears_on_that_systems_map_only() {
        let mut host = LocalSession::new(game_server::Session::default());
        let systems = host.authority.systems().to_vec();
        let here = host.state.ships[0].location.clone();
        let system = here.system().unwrap();
        for trader in &mut host.economy.traders {
            trader.state.ships[0].location =
                game_core::travel::Location::DeepSpace(game_core::movement::SpacePosition {
                    x_km: 0,
                    y_km: 0,
                    z_km: 0,
                });
            trader.state.navigation.orders.clear();
        }
        host.economy.traders[0].state.ships[0].location = here.clone();
        let home = systems.iter().find(|known| known.id == system).unwrap();
        let shown = host.traders_for_map(home, &systems);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].location, here);
        let other = systems.iter().find(|known| known.id != system).unwrap();
        assert!(host.traders_for_map(other, &systems).is_empty());
    }
}
