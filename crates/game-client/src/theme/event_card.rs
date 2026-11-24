//! Event cards: the pop-up for anything that asks the captain to choose. A
//! small category line and a display-font title over an illustrated scene
//! (`event_art`), the story text, then the choices stacked as full-width rows.
//! Contacts, campaign ends, jobs and quests all use it. Presentation only: the
//! caller sends the chosen command.
use super::event_art::{self, Illustration};
use super::icons::{self, Icon};
use egui::{Color32, RichText, Sense, Stroke, Ui, Vec2};

/// Card width on a wide screen, in points; a phone gets the screen width.
const WIDTH: f32 = 460.0;
/// Illustration height on a wide screen and on a phone, in points.
const ART_HEIGHT: (f32, f32) = (176.0, 136.0);
/// Height of one choice row, in points.
const CHOICE_HEIGHT: f32 = 38.0;

pub struct Card<'a> {
    /// Distinguishes simultaneous cards (egui window id).
    pub id: egui::Id,
    /// The category line above the title, e.g. "CONTACT · Fleet 1".
    pub kicker: &'a str,
    pub title: &'a str,
    pub art: &'a Illustration,
    /// Offer a Hide control (a decision that can wait); returns true when used.
    pub can_hide: bool,
}

/// Show `card` centred over the map with `add` as its body. Returns true
/// when the player hid it.
pub fn show(ctx: &egui::Context, card: Card<'_>, add: impl FnOnce(&mut Ui)) -> bool {
    let screen = ctx.screen_rect();
    let phone = screen.width() < super::metal::FULL_SCREEN_BELOW;
    let width = WIDTH.min(screen.width() - 40.0);
    let accent = card.art.mood.accent();
    let mut hide = false;
    egui::Window::new(card.title)
        .id(card.id)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .frame(egui::Frame::NONE)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            super::metal::frame(ui, 255, |ui| {
                ui.set_width(width);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(card.kicker.to_uppercase())
                            .small()
                            .color(Color32::from_gray(170)),
                    );
                    if card.can_hide {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            hide = icons::icon_button(
                                ui,
                                Icon::Cancel,
                                "Hide: decide later from the card's contact line",
                            )
                            .clicked();
                        });
                    }
                });
                ui.label(
                    RichText::new(card.title)
                        .font(super::fonts::display(ui.ctx(), 20.0))
                        .color(accent),
                );
                // A short screen trades picture for room to read and choose.
                let preferred = if phone { ART_HEIGHT.1 } else { ART_HEIGHT.0 };
                let height = preferred.min(screen.height() * 0.2);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
                let painter = ui.painter_at(rect);
                event_art::paint(&painter, rect, card.art);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0_f32, accent.gamma_multiply(0.6)),
                    egui::StrokeKind::Outside,
                );
                ui.add_space(6.0);
                add(ui);
            });
        });
    hide
}

/// Lay out the card's choices, stacked one per row.
pub fn choices(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.add_space(4.0);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        add(ui);
    });
}

/// One full-width choice row: an accent bar, an icon and the label.
pub fn choice(ui: &mut Ui, icon: Icon, label: &str) -> egui::Response {
    let text = ui.painter().layout(
        label.into(),
        super::fonts::text(15.0),
        ui.visuals().text_color(),
        ui.available_width() - 60.0,
    );
    let height = CHOICE_HEIGHT.max(text.size().y + 14.0);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if response.clicked() {
        crate::audio::mark_click(ui.ctx());
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    icons::frame(ui, rect, &response, false);
    let accent = icons::accent(ui);
    let bar = egui::Rect::from_min_size(
        rect.min + Vec2::new(3.0, 5.0),
        Vec2::new(3.0, height - 10.0),
    );
    let bar_color = if response.hovered() {
        accent
    } else {
        accent.gamma_multiply(0.45)
    };
    ui.painter().rect_filled(bar, 1.0, bar_color);
    icons::paint(
        ui.painter(),
        icon,
        rect.left_center() + Vec2::new(24.0, 0.0),
        accent,
    );
    ui.painter().galley(
        rect.left_center() + Vec2::new(42.0, -text.size().y * 0.5),
        text,
        ui.visuals().text_color(),
    );
    response
}

/// A [`choice`] that may be unavailable, with the reason on hover.
pub fn choice_action(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    blocked: Option<String>,
) -> egui::Response {
    let enabled = blocked.is_none();
    let response = ui
        .add_enabled_ui(enabled, |ui| choice(ui, icon, label))
        .inner;
    match blocked {
        Some(reason) => response.on_disabled_hover_text(reason),
        None => response,
    }
}
