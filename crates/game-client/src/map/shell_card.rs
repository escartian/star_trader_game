//! The context card's visibility and the fading message pop-up. The card can
//! be closed for the whole map; it comes back by itself when something new
//! wants it (a tap on the map, another tab, a contact). Each new session
//! message shows briefly at the bottom of the map, then fades; the Menu
//! page keeps the history. Presentation only.
use super::Map;
use crate::session::SheetTab;
use egui::{Align2, Color32, RichText};
use game_core::{BodyId, SystemId, fleet::FleetId, travel::ShipId};

/// Seconds a message stays fully visible, then seconds it takes to fade.
const TOAST_HOLD: f64 = 3.0;
const TOAST_FADE: f64 = 1.0;

/// What the card shows; when this changes, a closed card opens again.
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    tab: SheetTab,
    system: Option<SystemId>,
    local: Option<(SystemId, Option<BodyId>)>,
    ship: ShipId,
    npc: Option<game_core::pursuit::Quarry>,
    contacts: Vec<FleetId>,
}

pub(super) struct CardState {
    pub open: bool,
    shown: Option<Shown>,
    /// The message serial on screen and when it first appeared (seconds).
    toast: Option<(u64, f64)>,
    /// The map's area this frame, below the strip and beside the card.
    pub map_area: egui::Rect,
}

impl Default for CardState {
    fn default() -> Self {
        Self {
            open: true,
            shown: None,
            toast: None,
            map_area: egui::Rect::NOTHING,
        }
    }
}

impl CardState {
    /// Shows the current message as new (captures, which start mid-session).
    #[cfg(any(test, feature = "dev-tools"))]
    pub fn show_next_toast(&mut self) {
        self.toast = Some((u64::MAX, 0.0));
    }
}

impl Map {
    /// Reopens a closed card when what it would show changed since last frame.
    pub(super) fn reopen_card_on_change(&mut self) {
        let shown = Shown {
            tab: self.sheet,
            system: self.selected,
            local: self
                .local
                .as_ref()
                .map(|local| (local.system().id, local.selected_body())),
            ship: self.session.selected_ship().id,
            npc: self.session.npc_focus().0,
            contacts: self.session.contact_fleets(),
        };
        if let Some(last) = &self.card.shown {
            let moved = (last.tab, last.system, last.local, last.ship, last.npc)
                != (shown.tab, shown.system, shown.local, shown.ship, shown.npc);
            // A new contact needs a decision; a resolved one does not.
            let new_contact = shown
                .contacts
                .iter()
                .any(|fleet| !last.contacts.contains(fleet));
            if moved || new_contact {
                self.card.open = true;
            }
        }
        self.card.shown = Some(shown);
    }

    /// Closes the whole card (back to Selection for when it reopens).
    pub(super) fn close_card(&mut self) {
        self.sheet = SheetTab::Selection;
        self.card.open = false;
        // Closing must not count as a change that reopens it.
        if let Some(shown) = &mut self.card.shown {
            shown.tab = SheetTab::Selection;
        }
    }

    /// The latest message at the bottom of the map, fading out.
    pub(super) fn show_toast(&mut self, ctx: &egui::Context) {
        let serial = self.session.notice_serial();
        let now = ctx.input(|input| input.time);
        let started = match self.card.toast {
            Some((shown, started)) if shown == serial => started,
            // The first frame adopts what is there without showing it.
            None => {
                self.card.toast = Some((serial, f64::NEG_INFINITY));
                return;
            }
            Some(_) => {
                self.card.toast = Some((serial, now));
                now
            }
        };
        let age = now - started;
        if age > TOAST_HOLD + TOAST_FADE || self.session.notice().is_empty() {
            return;
        }
        let alpha = if age <= TOAST_HOLD {
            1.0
        } else {
            (1.0 - (age - TOAST_HOLD) / TOAST_FADE) as f32
        };
        ctx.request_repaint();
        let area = self.card.map_area;
        egui::Area::new(egui::Id::new("message-toast"))
            .order(egui::Order::Foreground)
            .interactable(false)
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(egui::pos2(area.center().x, area.bottom() - 12.0))
            .show(ctx, |ui| {
                ui.set_max_width((area.width() - 32.0).max(120.0));
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(
                        14,
                        20,
                        24,
                        (225.0 * alpha) as u8,
                    ))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        crate::theme::CYAN.gamma_multiply(0.6 * alpha),
                    ))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(self.session.notice())
                                .color(Color32::WHITE.gamma_multiply(alpha)),
                        );
                    });
            });
    }
}
