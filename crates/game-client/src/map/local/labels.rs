use egui::{Pos2, Rect, Vec2};

/// Reserve every marker before placing text, so a moon cannot sit under its
/// planet's label. Selection has priority when a very small viewport runs out of room.
pub(super) fn place(
    point: Pos2,
    radius: f32,
    size: Vec2,
    bounds: Rect,
    labels: &[Rect],
    markers: &[Rect],
    selected: bool,
) -> Option<Rect> {
    let gap = radius + 10.0;
    let offsets = [
        Vec2::new(gap, -size.y - 5.0),
        Vec2::new(gap, 8.0),
        Vec2::new(-gap - size.x, -size.y - 5.0),
        Vec2::new(-gap - size.x, 8.0),
    ];
    let candidates: Vec<_> = offsets
        .into_iter()
        .map(|offset| {
            let min = point + offset;
            let min = Pos2::new(
                min.x.clamp(
                    bounds.left() + 4.0,
                    (bounds.right() - size.x - 4.0).max(bounds.left() + 4.0),
                ),
                min.y.clamp(
                    bounds.top() + 4.0,
                    (bounds.bottom() - size.y - 4.0).max(bounds.top() + 4.0),
                ),
            );
            Rect::from_min_size(min, size)
        })
        .collect();
    candidates
        .iter()
        .copied()
        .find(|candidate| {
            bounds.contains_rect(*candidate)
                && !labels
                    .iter()
                    .any(|label| label.expand(6.0).intersects(*candidate))
                && !markers.iter().any(|marker| marker.intersects(*candidate))
        })
        .or_else(|| selected.then_some(candidates[0]))
}
