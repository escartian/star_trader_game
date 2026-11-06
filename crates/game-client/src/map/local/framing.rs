//! Fit arrival corridors and destinations together instead of chasing a boundary point.
use super::{LocalView, display_position};
use egui::{Rect, Vec2};
use game_core::travel::Ship;
use glam::DVec3;

impl LocalView {
    pub(super) fn fit_overview(&mut self, rect: Rect, ships: &[Ship]) {
        let mut points = vec![DVec3::ZERO];
        points.extend(
            self.system
                .bodies
                .iter()
                .map(|body| display_position(&self.system, body)),
        );
        points.extend(
            ships
                .iter()
                .filter(|ship| ship.id == self.active_ship)
                .filter_map(|ship| self.ship_anchor(&ship.location)),
        );
        points.extend(
            self.preview_route
                .iter()
                .map(|position| super::kilometers(*position) / super::KM_PER_VIEW_UNIT),
        );
        let rotation = self.camera.rotation();
        let mut low = DVec3::splat(f64::INFINITY);
        let mut high = DVec3::splat(f64::NEG_INFINITY);
        for point in points {
            let projected = rotation * point;
            low = low.min(projected);
            high = high.max(projected);
        }
        let center = (low + high) * 0.5;
        self.camera
            .focus_on(rotation.inverse() * DVec3::new(center.x, center.y, 0.0));
        // Reserve room for symbolic bodies, offset fleet cards and the departure control.
        let span = high - low;
        let usable = (rect.size() - Vec2::new(230.0, 180.0)).max(Vec2::splat(60.0));
        let pixels_per_unit =
            (f64::from(usable.x) / span.x.max(1.0)).min(f64::from(usable.y) / span.y.max(1.0));
        self.camera.zoom = (pixels_per_unit * 38.0 / f64::from(rect.width().min(rect.height())))
            .clamp(0.3, 2.2) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{LocalPosition, travel::Location};

    #[test]
    fn entry_framing_keeps_star_destinations_and_inbound_fleet_visible() {
        let mut view = LocalView::new(game_core::demo_galaxy().remove(0));
        let mut ship = game_core::travel::TravelState::default().ships.remove(0);
        ship.location = Location::InSystem {
            system: view.system.id,
            position: LocalPosition {
                x_km: 200_000_000,
                y_km: 10_000_000,
                z_km: 35_000_000,
            },
        };
        view.focus_ship(&ship);
        for size in [Vec2::new(760.0, 330.0), Vec2::new(1200.0, 620.0)] {
            let rect = Rect::from_min_size(egui::Pos2::ZERO, size);
            view.fit_overview(rect, std::slice::from_ref(&ship));
            let safe = rect.shrink(65.0);
            assert!(safe.contains(view.camera.project(DVec3::ZERO, rect)));
            assert!(
                safe.contains(
                    view.camera
                        .project(view.ship_anchor(&ship.location).unwrap(), rect)
                )
            );
            for body in &view.system.bodies {
                assert!(
                    safe.contains(
                        view.camera
                            .project(display_position(&view.system, body), rect)
                    )
                );
            }
        }
    }
}
