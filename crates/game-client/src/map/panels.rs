use super::{Map, position, visuals};
use crate::theme::icons::{Icon, button, stat};
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
                        if self.scene == super::fixtures::Scene::Session {
                            self.session.select_destination(system.id, None);
                        }
                    }
                }
            });
        }
        let mut open = false;
        if let Some(system) = self.systems.iter().find(|s| Some(s.id) == self.selected) {
            let origin = self.route_position();
            let distance = origin.map_or(0.0, |s| s.distance(position(system)));
            ui.horizontal(|ui| {
                crate::theme::icons::glyph(
                    ui,
                    Icon::Star,
                    visuals::appearance(system.star_kind).0,
                    system.star_kind.label(),
                );
                ui.strong(&system.name);
                if let Some((faction, color, tier)) = self.session.system_holder(system.id) {
                    ui.label(egui::RichText::new(faction.label()).color(color))
                        .on_hover_text(format!(
                            "{} space; your standing: {}",
                            faction.label(),
                            tier.label()
                        ));
                }
                stat(
                    ui,
                    Icon::Distance,
                    crate::theme::CYAN,
                    &format!("{distance:.2} ly"),
                    "Light-years from your fleet (true 3D distance)",
                );
                if !rect.contains(self.camera.project(position(system), rect)) {
                    crate::theme::icons::glyph(
                        ui,
                        Icon::Warning,
                        crate::theme::AMBER,
                        "Selection is outside the map. Use Focus selection to reveal it.",
                    );
                }
                open = button(ui, Icon::Enter, "Open system").clicked();
            });
        }
        if open {
            self.open_selected_system();
        }
        egui::CollapsingHeader::new("Systems").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("systems")
                .max_height(160.0)
                .show(ui, |ui| {
                    for system in &self.systems {
                        ui.horizontal(|ui| {
                            crate::theme::icons::glyph(
                                ui,
                                Icon::Star,
                                visuals::appearance(system.star_kind).0,
                                system.star_kind.label(),
                            );
                            if ui
                                .selectable_label(Some(system.id) == self.selected, &system.name)
                                .clicked()
                            {
                                self.selected = Some(system.id);
                                if self.scene == super::fixtures::Scene::Session {
                                    self.session.select_destination(system.id, None);
                                }
                            }
                        });
                    }
                });
        });
    }
}
