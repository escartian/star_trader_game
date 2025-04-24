use crate::models::position::Position;

#[test]
fn test_distance() {
    let pos1 = Position { x: 0, y: 0, z: 0 };
    let pos2 = Position { x: 3, y: 4, z: 5 };
    let distance = pos1.distance(&pos2);
    assert_eq!(distance, 7.0710678118654755);
} 