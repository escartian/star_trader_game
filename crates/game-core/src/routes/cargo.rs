//! Most-profitable cargo for one route leg, estimated from current prices.
use crate::economy::{Commodity, Market, TradeSide};

/// Units of each commodity (indexed by `Commodity::index`) to buy at `here` for
/// sale at `there`, within `space` cargo units and `credits` (hundredths).
///
/// Adds one unit at a time, always the unit with the largest positive margin: what
/// `there` pays at its rising stock minus what `here` charges at its falling stock,
/// the same per-unit curves trade quotes use. Each good's margin only shrinks as
/// units are added, so this greedy choice maximizes estimated profit for the space.
/// It stops when space or credits run out, `there` could not store or pay for
/// another unit, or no unit is profitable. Prices at arrival may differ.
pub(super) fn plan(here: &Market, there: &Market, space: u64, credits: u64) -> [u32; 6] {
    let mut planned = [0_u32; 6];
    let (mut space, mut credits, mut buyer_funds) = (space, credits, there.credits.0);
    while space > 0 {
        let next = Commodity::ALL
            .into_iter()
            .filter_map(|good| {
                let units = planned[good.index()];
                let local = &here.goods[good.index()];
                let remote = &there.goods[good.index()];
                let remote_stock = remote.stock.checked_add(units)?;
                if units >= local.stock || remote_stock >= remote.capacity() {
                    return None;
                }
                let cost = local.price(TradeSide::Buy, local.stock - units).0;
                let revenue = remote.price(TradeSide::Sell, remote_stock).0;
                // Lazy: the margin is only computed once it is known to be positive.
                (revenue > cost && cost <= credits && revenue <= buyer_funds)
                    .then(|| (good, revenue - cost, cost, revenue))
            })
            .max_by_key(|(_, margin, _, _)| *margin);
        let Some((good, _, cost, revenue)) = next else {
            break;
        };
        planned[good.index()] += 1;
        space -= 1;
        credits -= cost;
        buyer_funds -= revenue;
    }
    planned
}
