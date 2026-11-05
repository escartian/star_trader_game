//! Repeatable developer captures use the real application views.
use super::{Map, local::LocalView};

impl Map {
    pub fn prepare_capture(&mut self, view: &str) -> Result<(), String> {
        match view {
            "galaxy" => {}
            "system" => {
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.local = Some(LocalView::new(system));
            }
            "market" => self.session.open_market(),
            "fleets" => self.session.open_fleets(),
            "saves" => self.session.open_saves(),
            _ => return Err("Capture view must be galaxy, system, market, fleets or saves".into()),
        }
        Ok(())
    }
}
