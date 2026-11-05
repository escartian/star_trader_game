//! A galaxy-space filter. Camera rotation never changes which stars it includes.
use game_core::GalaxyPosition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    fn coordinate(self, position: GalaxyPosition) -> f64 {
        match self {
            Self::X => position.x as f64,
            Self::Y => position.y as f64,
            Self::Z => position.z as f64,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DepthSlice {
    pub enabled: bool,
    pub axis: Axis,
    pub center: f64,
    pub thickness: f64,
}

impl Default for DepthSlice {
    fn default() -> Self {
        Self {
            enabled: false,
            axis: Axis::Z,
            center: 0.0,
            thickness: 8.0,
        }
    }
}

impl DepthSlice {
    pub fn contains(&self, position: GalaxyPosition) -> bool {
        !self.enabled
            || (self.axis.coordinate(position) - self.center).abs() <= self.thickness / 2.0
    }

    pub fn reveal(&mut self, position: GalaxyPosition) {
        if !self.contains(position) {
            self.center = self.axis.coordinate(position);
        }
    }

    pub fn description(&self) -> String {
        format!(
            "{}: {:.1} to {:.1} ly • outside slice hidden",
            self.axis.label(),
            self.center - self.thickness / 2.0,
            self.center + self.thickness / 2.0
        )
    }

    /// Returns true when the filter changes, so stale overlap candidates can clear.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            changed |= ui.checkbox(&mut self.enabled, "Depth slice").changed();
            if self.enabled {
                for axis in [Axis::X, Axis::Y, Axis::Z] {
                    changed |= ui
                        .selectable_value(&mut self.axis, axis, axis.label())
                        .changed();
                }
                ui.label("Center (ly)");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.center)
                            .speed(0.5)
                            .range(-1_000_000.0..=1_000_000.0),
                    )
                    .changed();
                ui.label("Thickness (ly)");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.thickness)
                            .speed(0.5)
                            .range(0.1..=2_000_000.0),
                    )
                    .changed();
            }
        });
        if self.enabled {
            ui.label(self.description());
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_boundaries_are_inclusive_on_each_world_axis() {
        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let slice = DepthSlice {
                enabled: true,
                axis,
                center: 2.0,
                thickness: 4.0,
            };
            for value in [-1, 0, 2, 4, 5] {
                let position = match axis {
                    Axis::X => GalaxyPosition {
                        x: value,
                        y: 100,
                        z: 100,
                    },
                    Axis::Y => GalaxyPosition {
                        x: 100,
                        y: value,
                        z: 100,
                    },
                    Axis::Z => GalaxyPosition {
                        x: 100,
                        y: 100,
                        z: value,
                    },
                };
                assert_eq!(slice.contains(position), (0..=4).contains(&value));
            }
        }
    }

    #[test]
    fn reveal_recenters_only_when_needed_and_preserves_thickness() {
        let mut slice = DepthSlice {
            enabled: true,
            ..DepthSlice::default()
        };
        slice.reveal(GalaxyPosition { x: 0, y: 0, z: 2 });
        assert_eq!(slice.center, 0.0);
        slice.reveal(GalaxyPosition { x: 0, y: 0, z: 12 });
        assert_eq!(slice.center, 12.0);
        assert_eq!(slice.thickness, 8.0);
        slice.enabled = false;
        assert!(slice.contains(GalaxyPosition {
            x: 100,
            y: 100,
            z: 100
        }));
    }
}
