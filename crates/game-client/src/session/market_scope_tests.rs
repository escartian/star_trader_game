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
