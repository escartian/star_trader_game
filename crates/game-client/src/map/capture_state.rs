//! Repeatable developer captures use the real application views and movement commands.
use super::{Map, local::LocalView};
use game_core::{BodyId, LocalPosition, SystemId, movement::MoveTarget};

impl Map {
    pub fn prepare_capture(&mut self, view: &str) -> Result<(), String> {
        match view {
            "galaxy" => {}
            "system" => {
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.local = Some(LocalView::new(system));
            }
            "run-review" => self.session.capture_run_review(),
            "encounter" => self.session.capture_encounter()?,
            "navigation" => self.session.request_movement(MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            }),
            "deep-space" | "entry" => {
                let system = self
                    .session
                    .current_system()
                    .ok_or("Capture requires a ship in a system")?;
                self.session.capture_movement(MoveTarget::Local {
                    system,
                    position: LocalPosition {
                        x_km: self.session.system_half_extent_km() + 1,
                        y_km: 10_000_000,
                        z_km: 35_000_000,
                    },
                })?;
                if view == "entry" {
                    self.session
                        .request_movement(MoveTarget::EnterSystem(SystemId(2)));
                }
            }
            "orbit" | "planet-market" | "planet-shipyard" => {
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.session.capture_movement(MoveTarget::Body {
                    system: system.id,
                    body: BodyId(1),
                })?;
                self.local = Some(LocalView::new(system));
                if view == "planet-market" {
                    self.session.open_market();
                }
                if view == "planet-shipyard" {
                    self.session.open_fleets();
                }
            }
            "market" => self.session.open_market(),
            "fleets" => self.session.open_fleets(),
            "saves" => self.session.open_saves(),
            _ => return Err("Unknown capture view; see README.md for available views".into()),
        }
        Ok(())
    }
}
