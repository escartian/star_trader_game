//! Spatial fleet selection and destination cards. Coordinate entry is an advanced tool.
use super::{LocalSession, navigation::location_label};
use game_core::{BodyId, StarSystem, SystemId, fleet::Fleet, movement::MoveTarget, travel::ShipId};
use game_protocol::{ClientCommand, ServerMessage};

#[derive(Clone, Copy)]
pub(super) enum MapTarget {
    Destination(SystemId, Option<BodyId>),
    Departure(SystemId),
}

impl LocalSession {
    pub fn local_course_system(&self) -> Option<SystemId> {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok()?;
        let order = self.state.navigation.order(fleet.id)?;
        if matches!(
            order.plan.mode,
            game_core::movement::MovementMode::Local | game_core::movement::MovementMode::Exit
        ) {
            order.plan.origin.system()
        } else {
            None
        }
    }

    pub fn traffic(&self) -> &[game_core::npc::Trader] {
        &self.economy.traders
    }

    pub fn fleets(&self) -> &[Fleet] {
        &self.state.fleets
    }
    pub fn run_number(&self) -> u64 {
        self.state.navigation.run
    }
    pub fn select_map_fleet(&mut self, ship: ShipId) {
        self.select_ship(ship);
        self.map_target = None;
    }
    pub fn select_destination(&mut self, system: SystemId, body: Option<BodyId>) {
        self.map_target = Some(MapTarget::Destination(system, body));
    }
    pub fn select_departure(&mut self, system: SystemId) {
        self.map_target = Some(MapTarget::Departure(system));
    }

    pub fn map_roster(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        crate::theme::heading(
            ui,
            "FLEET COMMAND",
            "Select fleet / select destination / plot course",
        );
        self.destination_card(ui, systems);
        ui.separator();
        for fleet in self.state.fleets.clone() {
            let Some(ship) = fleet.ships.first().copied() else {
                continue;
            };
            let selected = fleet.ships.contains(&self.active_ship);
            let status = if self
                .state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.fleet == fleet.id)
            {
                "CONTACT / decision needed"
            } else if let Some(order) = self.state.navigation.order(fleet.id) {
                if order.started {
                    "UNDERWAY"
                } else {
                    "COURSE READY"
                }
            } else if self.state.navigation.holds.contains(&fleet.id) {
                "HOLDING"
            } else {
                "AWAITING ORDERS"
            };
            let frame = egui::Frame::group(ui.style()).fill(if selected {
                egui::Color32::from_rgb(18, 53, 68)
            } else {
                crate::theme::PANEL
            });
            frame.show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if ui
                    .selectable_label(selected, egui::RichText::new(&fleet.name).strong())
                    .clicked()
                {
                    self.select_map_fleet(ship);
                }
                ui.small(format!("F{} / {} ships", fleet.id.0, fleet.ships.len()));
                ui.colored_label(
                    if selected {
                        crate::theme::CYAN
                    } else {
                        crate::theme::AMBER
                    },
                    egui::RichText::new(status).monospace().size(10.0),
                );
                if let Some(order) = self.state.navigation.order(fleet.id) {
                    let target = order.remaining_waypoints.last().unwrap_or(&order.target);
                    let destination = match target {
                        MoveTarget::Body { system, body } => systems
                            .iter()
                            .find(|known| known.id == *system)
                            .and_then(|known| known.bodies.iter().find(|known| known.id == *body))
                            .map_or("Unknown body", |body| body.name.as_str()),
                        MoveTarget::EnterSystem(system) => systems
                            .iter()
                            .find(|known| known.id == *system)
                            .map_or("Unknown system", |system| system.name.as_str()),
                        _ => "Navigation waypoint",
                    };
                    ui.small(format!(
                        "→ {destination} / {} legs",
                        order.remaining_waypoints.len() + 1
                    ));
                }
                if let Ok(ship) = self.state.ship(ship) {
                    ui.small(location_label(&ship.location, systems));
                }
            });
        }
        ui.separator();
        ui.colored_label(
            crate::theme::AMBER,
            egui::RichText::new("TRAFFIC / NPC").monospace(),
        );
        for trader in &self.economy.traders {
            ui.strong(&trader.name);
            ui.small(trader.personality.label());
            ui.small(&trader.last_action);
        }
    }

    fn destination_card(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok().cloned();
        ui.strong(
            fleet
                .as_ref()
                .map_or("Selected fleet", |fleet| fleet.name.as_str()),
        );
        let Some(target) = self.map_target else {
            ui.label("Click a planet, station or point of interest on the map.");
            return;
        };
        let command = match target {
            MapTarget::Destination(system, body) => ClientCommand::PreviewDestination {
                ship: self.active_ship,
                system,
                body,
            },
            MapTarget::Departure(system) => ClientCommand::PreviewMove {
                ship: self.active_ship,
                target: MoveTarget::LeaveSystem(system),
            },
        };
        let (fuel, runs, revision) = match self.authority.handle(command) {
            ServerMessage::DestinationPreview(course) => {
                if let Some(last) = course.legs.last() {
                    ui.label(location_label(&last.destination, systems));
                }
                ui.small(format!("{} route legs / safe approach", course.legs.len()));
                (
                    course
                        .legs
                        .iter()
                        .map(|leg| u64::from(leg.fuel_per_ship))
                        .sum::<u64>(),
                    course
                        .legs
                        .iter()
                        .map(|leg| {
                            leg.duration_ticks
                                .div_ceil(self.state.movement_rules.ticks_per_navigation_run)
                        })
                        .sum::<u64>(),
                    self.state.revision,
                )
            }
            ServerMessage::MovementPreview(plan) => {
                ui.label("Depart to outer galaxy");
                (
                    u64::from(plan.fuel_per_ship),
                    plan.duration_ticks
                        .div_ceil(self.state.movement_rules.ticks_per_navigation_run),
                    plan.revision,
                )
            }
            ServerMessage::CommandRejected(error) => {
                ui.colored_label(crate::theme::AMBER, error.to_string());
                return;
            }
            _ => return,
        };
        ui.label(format!("{fuel} fuel / ship · {runs} runs minimum"));
        if ui
            .add_sized(
                [ui.available_width(), 34.0],
                egui::Button::new("PLOT COURSE"),
            )
            .clicked()
        {
            let command = match target {
                MapTarget::Destination(system, body) => ClientCommand::PlotDestination {
                    ship: self.active_ship,
                    system,
                    body,
                    expected_revision: revision,
                },
                MapTarget::Departure(system) => ClientCommand::Move {
                    ship: self.active_ship,
                    target: MoveTarget::LeaveSystem(system),
                    expected_revision: revision,
                },
            };
            self.command(command);
            self.map_target = None;
        }
    }
}

impl LocalSession {
    pub fn capture_wait_runs(&mut self, count: usize) {
        for _ in 0..count {
            for fleet in self.state.fleets.clone() {
                self.command(ClientCommand::HoldFleet {
                    fleet: fleet.id,
                    expected_revision: self.state.revision,
                });
            }
            self.request_execute_run();
        }
    }
}

impl LocalSession {
    pub fn map_route_points(
        &mut self,
        system: SystemId,
        systems: &[StarSystem],
    ) -> Vec<game_core::LocalPosition> {
        let plans = match self.map_target {
            Some(MapTarget::Destination(destination, body)) => {
                match self.authority.handle(ClientCommand::PreviewDestination {
                    ship: self.active_ship,
                    system: destination,
                    body,
                }) {
                    ServerMessage::DestinationPreview(course) => course.legs,
                    _ => Vec::new(),
                }
            }
            Some(MapTarget::Departure(origin)) => {
                match self.authority.handle(ClientCommand::PreviewMove {
                    ship: self.active_ship,
                    target: MoveTarget::LeaveSystem(origin),
                }) {
                    ServerMessage::MovementPreview(plan) => vec![plan],
                    _ => Vec::new(),
                }
            }
            None => self
                .state
                .fleet_for_ship(self.active_ship)
                .ok()
                .and_then(|fleet| self.state.navigation.order(fleet.id))
                .map(|order| vec![order.plan.clone()])
                .unwrap_or_default(),
        };
        let mut points = Vec::new();
        for plan in plans {
            if let (Ok(start), Ok(end)) = (
                game_core::movement::local_position(&plan.origin, system, systems),
                game_core::movement::local_position(&plan.destination, system, systems),
            ) {
                if points.last() != Some(&start) {
                    points.push(start);
                }
                points.push(end);
            }
        }
        points
    }
}
