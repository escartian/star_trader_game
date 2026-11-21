use super::{strength, yield_percent};
use crate::{economy::EconomyRules, fleet::ShipKind, npc::Personality};

#[test]
fn strength_counts_hull_left_and_shields_times_weapons() {
    let catalog = EconomyRules::default().ship_catalog;
    // Freighter: (60 + 10) × 1 weapon; patrol: (90 + 30) × 3.
    assert_eq!(strength([(ShipKind::Freighter, 0)], &catalog), 70);
    assert_eq!(strength([(ShipKind::Freighter, 40)], &catalog), 30);
    assert_eq!(strength([(ShipKind::Patrol, 0)], &catalog), 360);
    let pair = [(ShipKind::Freighter, 0), (ShipKind::Scout, 0)];
    assert_eq!(strength(pair, &catalog), 70 + 45);
}

#[test]
fn opportunists_hold_out_longer_than_cautious_traders() {
    assert!(yield_percent(Personality::Opportunist) > yield_percent(Personality::Cautious));
}
