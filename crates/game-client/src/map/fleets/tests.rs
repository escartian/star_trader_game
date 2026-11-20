use super::*;
use game_core::travel::TravelState;

/// Three fleets of one ship each, cloned from the default freighter.
fn fleets() -> (Vec<Fleet>, Vec<Ship>) {
    let base = TravelState::default();
    let ships: Vec<Ship> = (1..=3)
        .map(|id| Ship {
            id: ShipId(id),
            ..base.ships[0].clone()
        })
        .collect();
    let fleets = (1..=3)
        .map(|id| Fleet {
            id: FleetId(id),
            name: format!("Fleet {id}"),
            owner: base.fleets[0].owner,
            ships: vec![ShipId(id)],
        })
        .collect();
    (fleets, ships)
}

fn entry<'a>(fleet: &'a Fleet, ship: &'a Ship, anchor: Pos2, berth: Berth) -> StackEntry<'a> {
    StackEntry {
        fleet,
        ship,
        anchor,
        berth,
        contact: false,
        heading: None,
        ring: 0.0,
    }
}

/// Paints inside an egui frame (text needs the frame's fonts).
fn paint(entries: &[StackEntry], selected: Option<FleetId>) -> Vec<FleetHit> {
    let ctx = egui::Context::default();
    let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut hits = Vec::new();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        let art = Artwork::preload(ctx);
        let painter = Painter::new(ctx.clone(), egui::LayerId::background(), Rect::EVERYTHING);
        hits = paint_stacks(&painter, &art, view, entries, selected, &[]);
    });
    hits
}

#[test]
fn fleets_at_one_body_merge_into_a_single_chip_that_cycles_on_tap() {
    let (fleets, ships) = fleets();
    let body = Pos2::new(300.0, 300.0);
    let berth = Berth::AtBody {
        clearance: 12.0,
        landed: false,
    };
    let entries: Vec<_> = (0..3)
        .map(|index| entry(&fleets[index], &ships[index], body, berth))
        .collect();
    let hits = paint(&entries, Some(FleetId(1)));
    assert_eq!(hits.len(), 1, "one stack, one tap target");
    assert_eq!(hits[0].fleet, FleetId(2), "a tap selects the next fleet");
    assert_eq!(
        paint(&entries, Some(FleetId(3)))[0].fleet,
        FleetId(1),
        "and wraps"
    );
    assert_eq!(paint(&entries, None)[0].fleet, FleetId(1));
    // The chip sits beside the body, not on it.
    assert!(!hits[0].bounds.shrink(12.0).contains(body));
}

#[test]
fn fleets_in_flight_or_at_different_bodies_keep_separate_chips_on_their_spot() {
    let (fleets, ships) = fleets();
    let resting = Berth::AtBody {
        clearance: 12.0,
        landed: false,
    };
    let flying = Pos2::new(500.0, 200.0);
    let entries = [
        entry(&fleets[0], &ships[0], Pos2::new(200.0, 300.0), resting),
        entry(&fleets[1], &ships[1], Pos2::new(400.0, 300.0), resting),
        entry(&fleets[2], &ships[2], flying, Berth::Free),
    ];
    let hits = paint(&entries, None);
    assert_eq!(hits.len(), 3);
    let in_flight = hits.iter().find(|hit| hit.fleet == FleetId(3)).unwrap();
    assert_eq!(in_flight.bounds.center(), flying, "drawn on its position");
    assert_eq!(pick(&hits, flying).unwrap().fleet, FleetId(3));
}

#[test]
fn two_stacks_at_nearby_bodies_do_not_cover_each_other() {
    let (fleets, ships) = fleets();
    let resting = Berth::AtBody {
        clearance: 12.0,
        landed: false,
    };
    let entries = [
        entry(&fleets[0], &ships[0], Pos2::new(300.0, 300.0), resting),
        entry(&fleets[1], &ships[1], Pos2::new(306.0, 300.0), resting),
    ];
    let hits = paint(&entries, None);
    assert_eq!(hits.len(), 2);
    let chip =
        |hit: &FleetHit| Rect::from_center_size(hit.bounds.center(), Vec2::splat(CHIP * 2.0));
    assert!(!chip(&hits[0]).intersects(chip(&hits[1])));
}

#[test]
fn taps_nearer_a_system_than_a_resting_chip_go_to_the_system() {
    let chip = Pos2::new(100.0, 100.0);
    let hits = [FleetHit {
        fleet: FleetId(1),
        ship: ShipId(1),
        bounds: Rect::from_center_size(chip, Vec2::splat(HIT)),
    }];
    // Inside the chip's hit box, but closer to the system centre beside it.
    let edge = chip + Vec2::new(15.0, -15.0);
    assert!(pick_over_targets(&hits, edge, Some(5.0)).is_none());
    // On the chip itself the fleet wins; with no target nearby, the box decides.
    assert!(pick_over_targets(&hits, chip, Some(29.0)).is_some());
    assert!(pick_over_targets(&hits, edge, None).is_some());
}
