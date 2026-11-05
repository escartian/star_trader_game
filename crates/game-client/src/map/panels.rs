use super::{Map, position, visuals};
use egui::Rect;

impl Map {
    pub(super) fn details(&mut self, ui: &mut egui::Ui, rect: Rect) {
        if self.candidates.len() > 1 {
            ui.group(|ui| {
                ui.label("Overlapping targets — choose one:");
                for system in self
                    .systems
                    .iter()
                    .filter(|s| self.candidates.contains(&s.id))
                {
                    let distance = self
                        .route_position()
                        .map_or(0.0, |s| s.distance(position(system)));
                    if ui
                        .selectable_label(
                            Some(system.id) == self.selected,
                            format!(
                                "{} • {} • {:.2} ly",
                                system.name,
                                system.star_kind.label(),
                                distance
                            ),
                        )
                        .clicked()
                    {
                        self.selected = Some(system.id);
                    }
                }
            });
        }
        if let Some(system) = self.systems.iter().find(|s| Some(s.id) == self.selected) {
            let origin = self.route_position();
            let distance = origin.map_or(0.0, |s| s.distance(position(system)));
            ui.label(format!("{} • {}", system.name, system.star_kind.label()));
            ui.label(format!(
                "{distance:.2} light-years from fleet • true 3D distance"
            ));
            if !rect.contains(self.camera.project(position(system), rect)) {
                ui.label("Selection is outside the map. Use Focus selection to reveal it.");
            }
        }
        if ui
            .add_enabled(
                self.selected.is_some(),
                egui::Button::new("Open selected system"),
            )
            .clicked()
        {
            self.open_selected_system();
        }
        egui::CollapsingHeader::new("Systems & star legend").show(ui, |ui| {
            ui.label("Symbolic sizes • background stars are decorative");
            egui::ScrollArea::vertical()
                .id_salt("systems")
                .max_height(160.0)
                .show(ui, |ui| {
                    for system in &self.systems {
                        ui.horizontal(|ui| {
                            ui.colored_label(visuals::appearance(system.star_kind).0, "●");
                            if ui
                                .selectable_label(
                                    Some(system.id) == self.selected,
                                    format!("{} — {}", system.name, system.star_kind.label()),
                                )
                                .clicked()
                            {
                                self.selected = Some(system.id);
                            }
                        });
                    }
                });
        });
    }
}
