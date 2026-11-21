//! Spatial fleet selection and destination cards. Coordinate entry is an advanced tool.
use super::{LocalSession, navigation::location_stat};
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
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

    /// Where each own fleet under way is heading (its current leg's end), in
    /// galaxy-space kilometres; the map draws a short direction tick from it.
    pub fn fleet_headings(
        &self,
        systems: &[StarSystem],
    ) -> Vec<(
        game_core::fleet::FleetId,
        game_core::movement::SpacePosition,
    )> {
        self.state
            .navigation
            .orders
            .iter()
            .filter_map(|order| {
                game_core::navigation::space_position(&order.plan.destination, systems)
                    .map(|position| (order.plan.fleet, position))
            })
            .collect()
    }

    pub fn raider_bands(&self) -> &[game_core::navigation::RaiderBand] {
        &self.state.navigation.bands
    }

    pub fn landing_rules(&self) -> &game_core::movement::LandingRules {
        &self.state.movement_rules.landing
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
    /// Picking a destination plots the flagship's course at once (it replaces
    /// any earlier course; nothing moves until Advance). With a secondary fleet
    /// selected it only selects the target, whose card can then offer that
    /// fleet a trade route; secondary fleets take no direct courses.
    pub fn select_destination(&mut self, system: SystemId, body: Option<BodyId>) {
        self.map_target = Some(MapTarget::Destination(system, body));
        let commanding = self
            .state
            .fleet_for_ship(self.active_ship)
            .is_ok_and(|fleet| self.is_flagship(fleet.id));
        if !commanding {
            return;
        }
        self.command(ClientCommand::PlotDestination {
            ship: self.active_ship,
            system,
            body,
            expected_revision: self.state.revision,
        });
    }
    pub fn select_departure(&mut self, system: SystemId) {
        self.map_target = Some(MapTarget::Departure(system));
        let commanding = self
            .state
            .fleet_for_ship(self.active_ship)
            .is_ok_and(|fleet| self.is_flagship(fleet.id));
        if !commanding {
            return;
        }
        self.command(ClientCommand::Move {
            ship: self.active_ship,
            target: MoveTarget::LeaveSystem(system),
            expected_revision: self.state.revision,
        });
    }

    pub fn map_roster(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        ui.horizontal(|ui| {
            ui.colored_label(
                GREEN,
                egui::RichText::new("FLEETS").font(crate::theme::fonts::display(ui.ctx(), 14.0)),
            );
            icons::glyph(
                ui,
                Icon::Help,
                CYAN,
                "Your flagship takes direct orders: tap a destination on the map to plot its course, then Advance. Other fleets follow standing orders (Wait, trade route or patrol); use Take command to switch.",
            );
        });
        ui.separator();
        let artwork = crate::map::art::Artwork::preload(ui.ctx());
        for fleet in self.state.fleets.clone() {
            let Some(ship) = fleet.ships.first().copied() else {
                continue;
            };
            let selected = fleet.ships.contains(&self.active_ship);
            let (status_icon, status) = if self
                .state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.fleet == fleet.id)
            {
                (Icon::Contact, "CONTACT")
            } else if self.state.patrol(fleet.id).is_some()
                && self.state.navigation.order(fleet.id).is_none()
            {
                (Icon::Overview, "PATROL")
            } else if let Some(route) = self.state.trade_route(fleet.id) {
                if route.paused {
                    (Icon::Warning, "ROUTE PAUSED")
                } else {
                    (Icon::Transfer, "ON ROUTE")
                }
            } else if let Some(order) = self.state.navigation.order(fleet.id) {
                if order.started {
                    (Icon::Underway, "UNDERWAY")
                } else {
                    (Icon::Plot, "READY")
                }
            } else if self.state.navigation.holds.contains(&fleet.id) {
                if self.mining(fleet.id, systems) {
                    (Icon::Good(game_core::economy::Commodity::Ore), "MINING")
                } else {
                    (Icon::Hold, "WAITING")
                }
            } else if self.state.patrol(fleet.id).is_some() {
                (Icon::Overview, "PATROL")
            } else if !self.is_flagship(fleet.id) {
                // Secondary fleets never need orders: idle means waiting.
                (Icon::Hold, "WAITING")
            } else {
                (Icon::Warning, "NEEDS ORDERS")
            };
            let frame = egui::Frame::group(ui.style()).fill(if selected {
                egui::Color32::from_rgb(35, 55, 37)
            } else {
                crate::theme::PANEL
            });
            frame.show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    if let Ok(hull) = self.state.ship(ship) {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::Vec2::splat(48.0), egui::Sense::click());
                        artwork.paint(
                            crate::map::art::ArtKind::from(hull.kind),
                            ui.painter(),
                            rect.center(),
                            48.0,
                        );
                        if response
                            .on_hover_text(format!("Select {}", fleet.name))
                            .clicked()
                        {
                            self.select_map_fleet(ship);
                        }
                    }
                    ui.vertical(|ui| {
                        if ui
                            .selectable_label(selected, egui::RichText::new(&fleet.name).strong())
                            .clicked()
                        {
                            self.select_map_fleet(ship);
                        }
                        ui.horizontal(|ui| {
                            stat(
                                ui,
                                Icon::Ship,
                                GREEN,
                                &fleet.ships.len().to_string(),
                                "Ships in fleet",
                            );
                            let color =
                                if status_icon == Icon::Warning || status_icon == Icon::Contact {
                                    AMBER
                                } else {
                                    GREEN
                                };
                            icons::glyph(ui, status_icon, color, status);
                        });
                    });
                });
                if matches!(status_icon, Icon::Warning | Icon::Contact) {
                    ui.colored_label(AMBER, egui::RichText::new(status).small());
                }
                if self
                    .state
                    .navigation
                    .bands
                    .iter()
                    .any(|band| band.hunting == Some(fleet.id))
                {
                    ui.colored_label(
                        crate::battle::scene::HOSTILE,
                        egui::RichText::new("Raiders closing in. Dock or land to hide.").small(),
                    );
                }
                if status_icon == Icon::Warning
                    && self.state.fleet_position(fleet.id, fleet.owner).is_ok()
                    && icons::button(ui, Icon::Hold, "Wait")
                        .on_hover_text("Keep this fleet here until you plot a new course")
                        .clicked()
                {
                    self.command(ClientCommand::HoldFleet {
                        fleet: fleet.id,
                        expected_revision: self.state.revision,
                    });
                }
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
                    ui.horizontal(|ui| {
                        stat(ui, Icon::Plot, GREEN, destination, "Destination");
                        stat(
                            ui,
                            Icon::Legs,
                            CYAN,
                            &(order.remaining_waypoints.len() + 1).to_string(),
                            "Route legs remaining",
                        );
                    });
                }
                if let Ok(ship) = self.state.ship(ship) {
                    location_stat(ui, &ship.location, systems);
                }
                self.standing_orders(ui, &fleet, systems);
                if self.is_flagship(fleet.id)
                    && self.unlocked(game_core::progress::Feature::Landing)
                {
                    self.landing_controls(ui, fleet.id, systems);
                }
                self.route_line(ui, fleet.id, systems);
            });
        }
        ui.separator();
        self.traffic_list(ui, systems);
    }

    pub fn destination_card(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok().cloned();
        ui.strong(
            fleet
                .as_ref()
                .map_or("Selected fleet", |fleet| fleet.name.as_str()),
        );
        let Some(target) = self.map_target else {
            ui.horizontal(|ui| {
                icons::glyph(ui, Icon::Pin, CYAN, "No destination selected");
                ui.small("Pick a destination on the map");
            });
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
        let (fuel, runs) = match self.authority.handle(command) {
            ServerMessage::DestinationPreview(course) => {
                if let Some(last) = course.legs.last() {
                    location_stat(ui, &last.destination, systems);
                }
                stat(
                    ui,
                    Icon::Legs,
                    CYAN,
                    &course.legs.len().to_string(),
                    "Route legs, including safe approach",
                );
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
                )
            }
            ServerMessage::MovementPreview(plan) => {
                stat(
                    ui,
                    Icon::DeepSpace,
                    CYAN,
                    "Outer galaxy",
                    "Depart to outer galaxy",
                );
                (
                    u64::from(plan.fuel_per_ship),
                    plan.duration_ticks
                        .div_ceil(self.state.movement_rules.ticks_per_navigation_run),
                )
            }
            ServerMessage::CommandRejected(error) => {
                ui.colored_label(crate::theme::AMBER, error.to_string());
                return;
            }
            _ => return,
        };
        ui.horizontal(|ui| {
            stat(ui, Icon::Fuel, CYAN, &fuel.to_string(), "Fuel per ship");
            stat(
                ui,
                Icon::Execute,
                GREEN,
                &runs.to_string(),
                "Navigation runs, at least",
            );
        });
        if let MapTarget::Destination(system, body) = target {
            self.route_button(ui, system, body, systems);
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
