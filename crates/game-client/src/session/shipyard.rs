//! Local shipyard presentation; authoritative rules enforce funds and inventory.
use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_protocol::ClientCommand;

impl LocalSession {
    pub(super) fn shipyard_ui(&self, ui: &mut egui::Ui) -> Option<ClientCommand> {
        let ship = self.selected_ship();
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Shipyard, GREEN, "Shipyard");
            ui.heading("Shipyard");
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                &format!(
                    "New hulls join {}'s fleet.\nSales require an empty hold. Fuel and hold upgrades are not refunded; hulls sell at catalog specifications.",
                    ship.name
                ),
            );
        });
        let market = match self.economy.current_market(ship) {
            Ok(market) => market,
            Err(error) => {
                ui.label(error.to_string());
                return None;
            }
        };
        stat(
            ui,
            Icon::Funds,
            AMBER,
            &market.credits.to_string(),
            "Shipyard funds available to buy hulls",
        );
        let artwork = crate::map::art::Artwork::preload(ui.ctx());
        for spec in &self.economy.rules.ship_catalog.ships {
            ui.horizontal_wrapped(|ui| {
                let stock = market.ship_stock[spec.kind.index()];
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(36.0), egui::Sense::hover());
                artwork.paint(
                    crate::map::art::ArtKind::from(spec.kind),
                    ui.painter(),
                    rect.center(),
                    36.0,
                );
                ui.strong(spec.kind.label());
                let price = self.economy.inflate(spec.price);
                stat(ui, Icon::Credits, AMBER, &price.to_string(), "Price");
                stat(ui, Icon::Funds, CYAN, &stock.to_string(), "In stock");
                stat(
                    ui,
                    Icon::Cargo,
                    GREEN,
                    &spec.cargo_capacity.to_string(),
                    "Cargo hold",
                );
                stat(
                    ui,
                    Icon::Fuel,
                    CYAN,
                    &spec.fuel_capacity.to_string(),
                    "Fuel capacity",
                );
                stat(
                    ui,
                    Icon::Distance,
                    CYAN,
                    &format!("{} ly", spec.range_ly),
                    "Jump range",
                );
                let coursed = self
                    .state
                    .fleet_for_ship(ship.id)
                    .is_ok_and(|fleet| self.state.navigation.order(fleet.id).is_some());
                let blocked = if coursed {
                    Some("Cancel the fleet's plotted course first".to_string())
                } else if stock == 0 {
                    Some("Out of stock".to_string())
                } else if self.economy.credits < price {
                    Some("Not enough credits".to_string())
                } else if self.state.ships.len() >= 64 {
                    Some("Fleet limit of 64 ships reached".to_string())
                } else {
                    None
                };
                let tip = format!("Buy {} for {price} cr", spec.kind.label());
                if icons::action(ui, Icon::Buy, &tip, blocked).clicked() {
                    action = Some(ClientCommand::BuyShip {
                        ship: ship.id,
                        kind: spec.kind,
                        expected_revision: self.state.revision,
                    });
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.add_space(44.0);
                combat_chips(ui, &spec.combat, &self.economy.rules.combat_catalog, None);
            });
        }
        let quote = self.economy.quote_ship_sale(ship);
        let label = quote.as_ref().map_or_else(
            |_| format!("Sell {}", ship.name),
            |price| format!("Sell {} · {} cr", ship.name, price),
        );
        let blocked = if self.state.ships.len() <= 1 {
            Some("Keep at least one owned ship.".to_string())
        } else {
            quote.as_ref().err().map(ToString::to_string)
        };
        let enabled = blocked.is_none();
        let button = ui
            .add_enabled_ui(enabled, |ui| icons::button(ui, Icon::Sell, &label))
            .inner;
        if button.clicked() {
            action = Some(ClientCommand::SellShip {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
        if let Some(reason) = blocked {
            button.on_disabled_hover_text(reason);
        }
        action
    }
}

/// Hull integrity, shields, armor, engine and weapons for one ship class.
/// `damage` shows current integrity for an owned ship; `None` shows a new hull.
pub(super) fn combat_chips(
    ui: &mut egui::Ui,
    combat: &game_core::combat::CombatProfile,
    weapons: &game_core::combat::CombatCatalog,
    damage: Option<u32>,
) {
    let current = combat.hull.saturating_sub(damage.unwrap_or(0));
    // Integrity color: green when sound, amber when damaged, red when critical.
    let integrity = if current * 4 <= combat.hull {
        egui::Color32::from_rgb(240, 80, 70)
    } else if current < combat.hull {
        AMBER
    } else {
        GREEN
    };
    stat(
        ui,
        Icon::Hull,
        integrity,
        &format!("{current}/{}", combat.hull),
        &format!(
            "{} hull: structure points remaining / maximum",
            combat.size.label()
        ),
    );
    stat(
        ui,
        Icon::Shield,
        CYAN,
        &combat.shields.to_string(),
        "Shields (recharge before each engagement)",
    );
    stat(
        ui,
        Icon::Armor,
        CYAN,
        &combat.armor.to_string(),
        "Armor: reduces each hit that reaches the hull",
    );
    stat(
        ui,
        Icon::Engine,
        CYAN,
        &combat.engine.to_string(),
        "Engine rating 1–10: evasion and initiative",
    );
    let fitted: Vec<String> = combat
        .loadout
        .iter()
        .map(|id| {
            weapons.weapon(id).map_or_else(
                || id.clone(),
                |weapon| {
                    format!(
                        "{}: {} dmg × {}, {}% accuracy, {}% vs shields, {} armor piercing",
                        weapon.name,
                        weapon.damage,
                        weapon.shots,
                        weapon.accuracy_percent,
                        weapon.shield_efficiency_percent,
                        weapon.armor_penetration
                    )
                },
            )
        })
        .collect();
    stat(
        ui,
        Icon::Weapon,
        AMBER,
        &format!("{}/{}", combat.loadout.len(), combat.weapon_slots),
        &format!("Weapons fitted / slots\n{}", fitted.join("\n")),
    );
}
