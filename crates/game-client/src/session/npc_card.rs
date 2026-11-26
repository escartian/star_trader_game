//! A trader or raider band tapped on the map: who it is, where it is or is
//! bound, and Hail, Pursue or Stop pursuit. Display only; rules live in
//! `game_core::piracy` and `game_core::pursuit`.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::iff;
use game_core::{StarSystem, SystemId, piracy, pursuit::Quarry};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

impl LocalSession {
    /// Selects a trader or band tapped on the map (dropping a destination).
    pub fn select_npc(&mut self, quarry: Quarry) {
        self.npc_target = Some(quarry);
        self.map_target = None;
    }

    pub fn flagship_fleet(&self) -> Option<game_core::fleet::FleetId> {
        self.state.flagship(LOCAL_PLAYER)
    }

    /// The NPC selected on the map, and the one the flagship pursues.
    pub fn npc_focus(&self) -> (Option<Quarry>, Option<Quarry>) {
        (
            self.npc_target,
            self.state.navigation.pursuit.map(|pursuit| pursuit.quarry),
        )
    }

    /// The selected NPC's card at the top of the Selection page.
    pub fn npc_card(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let Some(quarry) = self.npc_target else {
            return;
        };
        let name = |id: SystemId| {
            systems
                .iter()
                .find(|known| known.id == id)
                .map_or("an unknown system", |known| known.name.as_str())
        };
        let (title, ring, whereabouts) = match quarry {
            Quarry::Trader(id) => {
                let Some(npc) = self.economy.traders.iter().find(|npc| npc.id == id) else {
                    self.npc_target = None;
                    return;
                };
                let temper = self.economy.rules.personality_catalog.get(npc.personality);
                let flies = npc.affiliation.map_or("independent".to_owned(), |faction| {
                    self.economy.rules.faction_catalog.get(faction).name.clone()
                });
                let place = match (
                    npc.state.ships[0].location.system(),
                    self.trader_heading(npc),
                ) {
                    (Some(here), Some(bound)) => {
                        format!("In {}, bound for {}", name(here), name(bound))
                    }
                    (Some(here), None) => format!("In {}", name(here)),
                    (None, Some(bound)) => format!("Between systems, bound for {}", name(bound)),
                    (None, None) => "Between systems".to_owned(),
                };
                (
                    format!(
                        "{} ({} {}, {flies})",
                        npc.name,
                        npc.species.label(),
                        temper.label.to_lowercase()
                    ),
                    self.trader_stance(npc).color(),
                    format!("{place}. {}", npc.last_action),
                )
            }
            Quarry::Band(id) => {
                let Some(band) = self
                    .state
                    .navigation
                    .bands
                    .iter()
                    .find(|band| band.id == id)
                else {
                    self.npc_target = None;
                    return;
                };
                let place = match band.transit {
                    Some(_) => format!("Crossing to {}", name(band.system)),
                    None => format!("In {}", name(band.system)),
                };
                (
                    format!("Raiders · {}", super::crew(&band.ships)),
                    iff::HOSTILE,
                    place,
                )
            }
        };
        let pursued = self
            .state
            .navigation
            .pursuit
            .is_some_and(|p| p.quarry == quarry);
        let placed = game_core::orbits::placed(systems, self.state.tick);
        let hailable = match quarry {
            Quarry::Trader(id) => {
                piracy::in_reach(&self.state, &self.economy, LOCAL_PLAYER, &placed).contains(&id)
            }
            Quarry::Band(_) => false,
        };
        let mut action = None;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Contact, ring, "Selected on the map");
                ui.colored_label(ring, egui::RichText::new(title).strong());
                if icons::icon_button(ui, Icon::Cancel, "Deselect").clicked() {
                    action = Some(Act::Deselect);
                }
            });
            ui.small(whereabouts);
            ui.horizontal_wrapped(|ui| {
                if hailable
                    && icons::button(ui, Icon::Contact, "Hail")
                        .on_hover_text("Stop it: trade, demand cargo, attack or let it go.")
                        .clicked()
                {
                    action = Some(Act::Hail);
                }
                if pursued {
                    ui.colored_label(crate::theme::AMBER, "Pursuing");
                    if icons::button(ui, Icon::Hold, "Stop")
                        .on_hover_text("Give up the chase and wait here")
                        .clicked()
                    {
                        action = Some(Act::Stop);
                    }
                } else if !hailable
                    && self.can_pursue()
                    && Self::pursue_button(ui, "it, even into another system")
                {
                    action = Some(Act::Pursue);
                }
            });
        });
        let flagship = self.state.flagship(LOCAL_PLAYER);
        match (action, flagship) {
            (Some(Act::Deselect), _) => self.npc_target = None,
            (Some(Act::Pursue), _) => self.pursue(quarry),
            (Some(Act::Hail), Some(fleet)) => {
                if let Quarry::Trader(trader) = quarry {
                    self.command(ClientCommand::HailTrader {
                        fleet,
                        trader,
                        expected_revision: self.state.revision,
                    });
                }
            }
            (Some(Act::Stop), Some(fleet)) => self.command(ClientCommand::HoldFleet {
                fleet,
                expected_revision: self.state.revision,
            }),
            _ => {}
        }
    }
}

enum Act {
    Deselect,
    Hail,
    Pursue,
    Stop,
}
