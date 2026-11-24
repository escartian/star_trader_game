//! Repeatable developer captures use the real application views and movement commands.
use super::{Map, local::LocalView};
use game_core::{BodyId, LocalPosition, SystemId, movement::MoveTarget};

impl Map {
    pub fn prepare_capture(&mut self, view: &str) -> Result<(), String> {
        // "system:N": the system view of the galaxy's Nth system (1-based).
        if let Some(index) = view.strip_prefix("system:") {
            let index: usize = index.parse().map_err(|_| "system:N needs a number")?;
            let system = self
                .systems
                .get(index.saturating_sub(1))
                .ok_or("No such system")?
                .clone();
            self.local = Some(LocalView::new(system));
            return Ok(());
        }
        match view {
            "galaxy" => {}
            "traffic" => self.session.capture_wait_runs(4),
            "traffic-flight" => self.session.capture_traders_in_flight(2, 60)?,
            "traffic-system" => {
                // The system view where the most NPC traders rest after a few runs.
                self.session.capture_wait_runs(4);
                let busiest = self
                    .systems
                    .iter()
                    .max_by_key(|system| {
                        self.session
                            .traffic()
                            .iter()
                            .filter(|npc| npc.state.ships[0].location.system() == Some(system.id))
                            .count()
                    })
                    .ok_or("No system to capture")?
                    .clone();
                self.local = Some(LocalView::new(busiest));
            }
            "destination" => {
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.session.select_destination(system.id, Some(BodyId(1)));
                self.local = Some(LocalView::new(system));
            }
            "system" => {
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.local = Some(LocalView::new(system));
            }
            "advance" => {
                // The first capture frame executes the advance's first run.
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.session.capture_advance()?;
                self.local = Some(LocalView::new(system));
            }
            "run-review" => self.session.capture_run_review(),
            "trade-route" => self.session.capture_trade_route()?,
            "encounter" => self.session.capture_encounter()?,
            "hail" => self.session.capture_hail()?,
            "trader-offer" => self.session.capture_approach("offer")?,
            "shakedown" => self.session.capture_approach("shakedown")?,
            "distress" => self.session.capture_approach("distress")?,
            "combat" => self.session.capture_combat()?,
            "battle" => self.session.capture_battle()?,
            "battle-close" => self.session.capture_battle_close()?,
            "victory" => self.session.capture_victory(),
            "factions" => self.session.capture_factions(true)?,
            "factions-map" => self.session.capture_factions(false)?,
            "roster" => self.sheet = crate::session::SheetTab::Fleets,
            "pursue" => {
                // A trader and a raider band to chase in the flagship's system.
                self.session.capture_pursuit_quarry()?;
                self.sheet = crate::session::SheetTab::Fleets;
            }
            "game-menu" => self.sheet = crate::session::SheetTab::Menu,
            "card-closed" => {
                // The card closed and the latest message popping up.
                self.session.capture_wait_runs(1);
                self.close_card();
                self.card.show_next_toast();
            }
            "game-songbook" => {
                self.sheet = crate::session::SheetTab::Menu;
                self.songbook.0 = true;
            }
            "unlock" => self.session.capture_unlock(),
            // "chooser@SECONDS": the main menu at that moment of its animation.
            view if view.starts_with("chooser@") => {
                let seconds = view["chooser@".len()..]
                    .parse::<f64>()
                    .map_err(|_| "chooser@SECONDS needs a number of seconds")?;
                self.menu_seed = 0;
                self.menu_still_time = seconds.max(0.0);
                self.open_save_chooser("data/slot-1.json".into(), None);
            }
            "chooser" => {
                // A fixed sky (the gas giant) keeps menu previews reproducible.
                self.menu_seed = 0;
                self.open_save_chooser("data/slot-1.json".into(), None);
            }
            "menu-songbook" => {
                self.menu_seed = 0;
                self.open_save_chooser("data/slot-1.json".into(), None);
                if let Some(menu) = &mut self.save_recovery {
                    menu.songbook_open = true;
                }
            }
            view if view.starts_with("menu-songbook@") => {
                // The map view as if a part were playing: menu-songbook@cruise_a.
                let id = super::songbook::id(&view["menu-songbook@".len()..])
                    .ok_or("menu-songbook@PART needs a music part ID")?;
                self.menu_seed = 0;
                self.open_save_chooser("data/slot-1.json".into(), None);
                if let Some(menu) = &mut self.save_recovery {
                    menu.songbook_open = true;
                    menu.songbook_view = super::songbook::View::Map;
                    menu.songbook_preview = Some(id);
                }
            }
            "menu-new" | "menu-load" | "menu-settings" => {
                self.menu_seed = 0;
                self.open_save_chooser("data/slot-1.json".into(), None);
                if let Some(menu) = &mut self.save_recovery {
                    menu.page = match view {
                        "menu-new" => super::main_menu::MenuPage::NewGame,
                        "menu-load" => super::main_menu::MenuPage::Load,
                        _ => super::main_menu::MenuPage::Settings,
                    };
                }
            }
            "settings" => self.session.open_settings(),
            "haven" => {
                let id = self.session.capture_haven()?;
                let system = self
                    .systems
                    .iter()
                    .find(|known| known.id == id)
                    .ok_or("No Haven system")?
                    .clone();
                self.local = Some(LocalView::new(system));
            }
            "navigation" => self.session.request_movement(MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            }),
            "deep-space" | "entry" | "arrival" | "approach" => {
                let system = self
                    .session
                    .current_system()
                    .ok_or("Capture requires a ship in a system")?;
                self.session.capture_movement(MoveTarget::Local {
                    system,
                    position: LocalPosition {
                        x_km: self.session.system_half_extent_km() + 1,
                        y_km: 0,
                        z_km: 35_000_000,
                    },
                })?;
                if view == "entry" {
                    self.session
                        .request_movement(MoveTarget::EnterSystem(SystemId(2)));
                } else if matches!(view, "arrival" | "approach") {
                    self.follow_navigation_transitions();
                    self.session
                        .capture_movement(MoveTarget::EnterSystem(system))?;
                    self.follow_navigation_transitions();
                    if view == "approach" {
                        self.session.capture_navigation_step(MoveTarget::Body {
                            system,
                            body: BodyId(2),
                        })?;
                    }
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
                    self.session.open_shipyard();
                }
            }
            "landed" => {
                // Mining on the asteroid: landed, waiting, a few runs of ore aboard.
                let system = self.systems.first().ok_or("No system to capture")?.clone();
                self.session.capture_movement(MoveTarget::Body {
                    system: system.id,
                    body: BodyId(5),
                })?;
                self.session.capture_landing(3)?;
                let mut view = LocalView::new(system);
                view.focus_ship(self.session.selected_ship());
                self.local = Some(view);
            }
            "raiders" => {
                self.session.capture_hunted()?;
                let system = self
                    .session_systems
                    .iter()
                    .find(|system| system.id == game_core::SystemId(2))
                    .ok_or("Meridian is missing")?
                    .clone();
                self.local = Some(LocalView::new(system));
            }
            "police" => {
                let id = self.session.capture_police()?;
                let system = self
                    .session_systems
                    .iter()
                    .find(|system| system.id == id)
                    .ok_or("The policed system is missing")?
                    .clone();
                self.local = Some(LocalView::new(system));
            }
            "market" => self.session.open_market(),
            "shipyard" => self.session.open_shipyard(),
            "fleets" => self.session.open_fleets(),
            "saves" => self.session.open_saves(),
            _ => return Err("Unknown capture view; see README.md for available views".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::map::Map;

    fn map(seed: u64) -> Map {
        let mut authority = game_server::Session::with_seed(seed);
        let game_protocol::ServerMessage::GalaxySnapshot { systems, .. } =
            authority.handle(game_protocol::ClientCommand::GetGalaxy)
        else {
            unreachable!()
        };
        Map::new(systems, authority)
    }

    #[test]
    fn every_capture_view_prepares_from_a_fresh_session() {
        for view in [
            "galaxy",
            "traffic",
            "destination",
            "system",
            "advance",
            "run-review",
            "trade-route",
            "navigation",
            "deep-space",
            "entry",
            "arrival",
            "approach",
            "orbit",
            "landed",
            "raiders",
            "planet-market",
            "planet-shipyard",
            "market",
            "fleets",
            "saves",
            "game-menu",
            "card-closed",
            "traffic-system",
            "traffic-flight",
            "pursue",
            "game-songbook",
            "menu-songbook",
            "menu-songbook@cruise_a",
        ] {
            map(1)
                .prepare_capture(view)
                .unwrap_or_else(|error| panic!("{view}: {error}"));
        }
        map(2).prepare_capture("encounter").unwrap();
        let mut combat = map(1);
        combat.prepare_capture("combat").unwrap();
        assert!(combat.session.combat_report_open());
        let mut battle = map(1);
        battle.prepare_capture("battle").unwrap();
        assert!(battle.session.battle_open());
        // While a battle is open, Escape asks about surrendering; the map
        // behind it keeps its view.
        let ctx = egui::Context::default();
        let local = battle.local.is_some();
        let escape = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(escape, |ctx| battle.ui(ctx));
        assert!(battle.session.surrender_prompt_open());
        assert_eq!(battle.local.is_some(), local);
        assert!(battle.session.battle_open(), "asking does not surrender");
        let mut close = map(1);
        close.prepare_capture("battle-close").unwrap();
        assert!(close.session.battle_open());
        assert!(map(1).prepare_capture("nonsense").is_err());
    }
}
