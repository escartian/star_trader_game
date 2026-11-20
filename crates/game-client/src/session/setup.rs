//! New-game setup controls, shared by the game chooser and the Session panel:
//! galaxy size, starting credits, inflation, danger and the campaign's length
//! and Trader goal. The choices become session rules in `game_server::setup`.
use egui::{RichText, Ui};
use game_core::economy::Credits;
use game_server::{DangerLevel, GameSetup, InflationLevel, STARTING_CREDITS, WEALTH_SHARES};

/// Campaign lengths offered (0: open-ended).
const RUN_LIMITS: [u64; 4] = [0, 200, 400, 800];

pub(crate) fn setup_controls(ui: &mut Ui, setup: &mut GameSetup) {
    let galaxy = &mut setup.galaxy;
    ui.horizontal_wrapped(|ui| {
        ui.label("STAR SYSTEMS");
        ui.add(egui::DragValue::new(&mut galaxy.systems).range(4..=150));
        ui.label("MAP RADIUS (ly)");
        ui.add(egui::DragValue::new(&mut galaxy.radius_ly).range(20..=95));
        ui.label("MAX OBJECTS / SYSTEM");
        ui.add(egui::DragValue::new(&mut galaxy.max_objects).range(6..=15));
    });
    let area = std::f64::consts::PI * f64::from(galaxy.radius_ly).powi(2);
    ui.small(format!(
        "Up to {} ly across · {:.1} systems per 1,000 ly² · objects include the star and \
         trade port. Every system is reachable by hops of 20 ly or less.",
        u32::from(galaxy.radius_ly) * 2,
        f64::from(galaxy.systems) * 1000.0 / area
    ));
    choice_row(
        ui,
        "STARTING CREDITS",
        &mut setup.starting_credits,
        STARTING_CREDITS.map(Credits),
        |credits| format!("{credits} cr"),
    );
    choice_row(
        ui,
        "INFLATION",
        &mut setup.inflation,
        InflationLevel::ALL,
        |level| level.label().to_owned(),
    );
    choice_row(ui, "DANGER", &mut setup.danger, DangerLevel::ALL, |level| {
        level.label().to_owned()
    });
    choice_row(
        ui,
        "CAMPAIGN LENGTH",
        &mut setup.run_limit,
        RUN_LIMITS.map(|runs| (runs > 0).then_some(runs)),
        |limit| match limit {
            None => "Open-ended".into(),
            Some(runs) => format!("{runs} runs, then score"),
        },
    );
    choice_row(
        ui,
        "TRADER GOAL",
        &mut setup.wealth_share_percent,
        WEALTH_SHARES,
        |share| format!("{share}% of galaxy wealth"),
    );
}

/// A labelled drop-down over a fixed set of choices.
fn choice_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    choices: impl IntoIterator<Item = T>,
    text: impl Fn(T) -> String,
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [130.0, 18.0],
            egui::Label::new(RichText::new(label).small()),
        );
        egui::ComboBox::from_id_salt(label)
            .selected_text(text(*value))
            .show_ui(ui, |ui| {
                for choice in choices {
                    ui.selectable_value(value, choice, text(choice));
                }
            });
    });
}

/// One line describing a new game, for the log.
pub(crate) fn setup_summary(seed: u64, setup: &GameSetup) -> String {
    format!(
        "New map (seed {seed}, {} systems, {} ly radius, up to {} objects per system; \
         {} cr to start, {} inflation, {} danger).",
        setup.galaxy.systems,
        setup.galaxy.radius_ly,
        setup.galaxy.max_objects,
        setup.starting_credits,
        setup
            .inflation
            .label()
            .split(' ')
            .next()
            .unwrap_or("standard"),
        format!("{:?}", setup.danger).to_lowercase(),
    )
}
