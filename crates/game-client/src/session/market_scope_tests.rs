use super::TradeScope;
use crate::session::LocalSession;

/// Sale ports the scope allows from the selected ship's market.
fn candidates(host: &mut LocalSession, scope: TradeScope) -> Vec<game_core::economy::MarketId> {
    let ship = host.selected_ship().clone();
    host.market.scope = scope;
    host.refresh_reach(&ship, &game_core::demo_galaxy());
    let here = host.economy.current_market(&ship).unwrap().clone();
    host.economy
        .markets
        .iter()
        .filter(|market| host.in_scope(&here, market))
        .map(|market| market.id)
        .collect()
}

#[test]
fn scopes_narrow_from_the_galaxy_to_fuel_range_and_this_system() {
    let mut host = LocalSession::new(game_server::Session::default());
    let here = host
        .economy
        .current_market(host.selected_ship())
        .unwrap()
        .id;
    let galaxy = candidates(&mut host, TradeScope::Galaxy);
    let system = candidates(&mut host, TradeScope::System);
    let range = candidates(&mut host, TradeScope::InRange);
    assert_eq!(galaxy.len() + 1, host.economy.markets.len(), "all but here");
    assert!(!galaxy.contains(&here));
    assert!(system.iter().all(|id| id.system == here.system));
    assert!(!range.is_empty() && range.iter().all(|id| galaxy.contains(id)));
    // With empty tanks nothing is in range; the cache refreshes per revision.
    host.state.ships[0].fuel = 0;
    host.state.revision += 1;
    assert!(candidates(&mut host, TradeScope::InRange).is_empty());
}

#[test]
fn the_galaxy_scope_prefers_a_near_port_paying_almost_the_best() {
    use game_core::economy::{Commodity, Credits};
    let mut host = LocalSession::new(game_server::Session::default());
    let systems = game_core::demo_galaxy();
    let here = host
        .economy
        .current_market(host.selected_ship())
        .unwrap()
        .id;
    let origin = systems
        .iter()
        .find(|s| s.id == here.system)
        .unwrap()
        .position;
    let far = |id: game_core::SystemId| {
        let p = systems.iter().find(|s| s.id == id).unwrap().position;
        (p.x - origin.x).pow(2) + (p.y - origin.y).pow(2) + (p.z - origin.z).pow(2)
    };
    let mut others: Vec<_> = host
        .economy
        .markets
        .iter()
        .map(|market| market.id)
        .filter(|id| id.system != here.system)
        .collect();
    others.sort_by_key(|id| far(id.system));
    let (near, distant) = (others[0], *others.last().unwrap());
    assert!(far(near.system) < far(distant.system));
    let food = Commodity::Food.index();
    for market in &mut host.economy.markets {
        let good = &mut market.goods[food];
        good.base_price = Credits(match market.id {
            id if id == distant => 1_000,
            id if id == near => 980,
            _ => 10,
        });
    }
    host.market.scope = super::TradeScope::Galaxy;
    let market = host
        .economy
        .current_market(host.selected_ship())
        .unwrap()
        .clone();
    let sale = host.best_sale(&market, Commodity::Food, &systems).unwrap();
    assert_eq!(sale.market.id, near, "2% less, much closer");
    // A near port paying far less loses to the distant best.
    for market in &mut host.economy.markets {
        if market.id == near {
            market.goods[food].base_price = Credits(500);
        }
    }
    let sale = host.best_sale(&market, Commodity::Food, &systems).unwrap();
    assert_eq!(sale.market.id, distant);
}
