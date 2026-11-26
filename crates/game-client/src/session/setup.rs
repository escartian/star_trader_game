//! New-game setup controls, shared by the game chooser and the Session panel:
//! galaxy size, starting credits, inflation, danger, the campaign's length
//! and Trader goal, and the factions in play. The choices become session rules in `game_server::setup`.
use egui::{Response, Ui};
use game_core::{
    economy::Credits,
    factions::{Coverage, Faction, RelationMode},
    species::Species,
};
use game_server::{DangerLevel, GameSetup, STARTING_CREDITS, WEALTH_SHARES};

/// Campaign lengths offered (0: open-ended).
const RUN_LIMITS: [u64; 4] = [0, 200, 400, 800];

/// Every setting on its own line: its name, then its value right beside it.
pub(crate) fn setup_controls(ui: &mut Ui, setup: &mut GameSetup) {
    let phone = ui.ctx().screen_rect().width() <= 600.0;
    let picker_id = ui.id().with("captain-species-picker");
    let mut picker_open = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(picker_id).unwrap_or(false));
    ui.horizontal(|ui| {
        ui.label("Captain name");
        ui.add(
            egui::TextEdit::singleline(&mut setup.captain_name)
                .char_limit(game_core::travel::MAX_CAPTAIN_NAME_CHARS)
                .hint_text("Captain")
                .desired_width(180.0),
        )
        .on_hover_text("How the game addresses you; leave empty for plain \"Captain\".");
    });
    ui.horizontal(|ui| {
        super::species_portraits::show(ui, setup.species, 112.0);
        ui.vertical(|ui| {
            let title = format!("Captain species: {} ▾", setup.species.label());
            if phone {
                if ui.button(title).clicked() {
                    picker_open = true;
                }
            } else {
                ui.menu_button(title, |ui| {
                    ui.set_width((ui.ctx().screen_rect().width().min(440.0) - 24.0).max(280.0));
                    for species in Species::ALL {
                        if species_option(ui, species, setup.species).clicked() {
                            setup.species = species;
                            ui.close_menu();
                        }
                    }
                });
            }
            ui.small(setup.species.description());
            ui.small("This captain's traits affect the ships they command.");
        });
    });
    if phone && picker_open {
        let screen = ui.ctx().screen_rect();
        let response = egui::Modal::new(picker_id).show(ui.ctx(), |ui| {
            ui.set_width((screen.width() - 40.0).clamp(160.0, 400.0));
            ui.heading("Choose captain species");
            ui.small("Choose who commands your starting fleet.");
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((screen.height() - 160.0).max(180.0))
                .show(ui, |ui| {
                    for species in Species::ALL {
                        if species_option(ui, species, setup.species).clicked() {
                            setup.species = species;
                            picker_open = false;
                        }
                    }
                });
            if ui.button("Cancel").clicked() {
                picker_open = false;
            }
        });
        if response.should_close() {
            picker_open = false;
        }
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(picker_id, picker_open));
    egui::Grid::new("new-game-setup")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            let galaxy = &mut setup.galaxy;
            number_row(ui, "Star systems", &mut galaxy.systems, 4..=150);
            number_row(ui, "Map radius (ly)", &mut galaxy.radius_ly, 20..=95);
            number_row(
                ui,
                "Max objects per system",
                &mut galaxy.max_objects,
                6..=15,
            );
            choice_row(
                ui,
                "Starting credits",
                &mut setup.starting_credits,
                STARTING_CREDITS.map(Credits),
                |credits| format!("{credits} cr"),
            );
            ui.label("Inflation / 100 runs");
            ui.add(
                egui::DragValue::new(&mut setup.inflation_basis_points)
                    .range(0..=10_000)
                    .speed(1)
                    .custom_formatter(|value, _| format!("{:.2}%", value / 100.0))
                    .custom_parser(|text| {
                        text.trim()
                            .trim_end_matches('%')
                            .trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|value| value.is_finite())
                            .map(|value| (value * 100.0).round())
                    }),
            )
            .on_hover_text("Compounds across navigation ticks; 0.00% keeps prices steady.");
            ui.end_row();
            choice_row(ui, "Danger", &mut setup.danger, DangerLevel::ALL, |level| {
                level.label().to_owned()
            });
            choice_row(
                ui,
                "Campaign length",
                &mut setup.run_limit,
                RUN_LIMITS.map(|runs| (runs > 0).then_some(runs)),
                |limit| match limit {
                    None => "Open-ended".into(),
                    Some(runs) => format!("{runs} runs, then score"),
                },
            );
            choice_row(
                ui,
                "Trader goal",
                &mut setup.wealth_share_percent,
                WEALTH_SHARES,
                |share| format!("{share}% of galaxy wealth"),
            );
            faction_rows(ui, &mut setup.factions);
        });
    let galaxy = &setup.galaxy;
    let area = std::f64::consts::PI * f64::from(galaxy.radius_ly).powi(2);
    ui.small(format!(
        "Up to {} ly across · {:.1} systems per 1,000 ly² · objects include the star and \
         trade port. Every system is reachable by hops of 20 ly or less.",
        u32::from(galaxy.radius_ly) * 2,
        f64::from(galaxy.systems) * 1000.0 / area
    ));
    if !setup.factions.in_play.contains(&true) {
        ui.small("No factions: every system is independent and trades at list prices.");
    }
}

fn species_option(ui: &mut Ui, species: Species, selected: Species) -> Response {
    let text_width = (ui.available_width() - 56.0 - ui.spacing().item_spacing.x).max(120.0);
    let response = ui
        .horizontal(|ui| {
            super::species_portraits::show(ui, species, 56.0);
            ui.vertical(|ui| {
                ui.set_width(text_width);
                ui.selectable_label(
                    selected == species,
                    format!("{}\n{}", species.label(), species.description()),
                )
            })
            .inner
        })
        .inner;
    ui.separator();
    response
}

/// Which factions take part, then (with any in play) your allegiance,
/// territory and relations: grid rows.
fn faction_rows(ui: &mut Ui, factions: &mut game_core::factions::FactionSetup) {
    ui.label("Factions");
    ui.horizontal_wrapped(|ui| {
        for faction in Faction::PLAYABLE {
            ui.checkbox(&mut factions.in_play[faction.index()], faction.label());
        }
    });
    ui.end_row();
    // An allegiance to a faction left out of the game lapses.
    if factions
        .allegiance
        .is_some_and(|faction| !factions.in_play[faction.index()])
    {
        factions.allegiance = None;
    }
    if !factions.in_play.contains(&true) {
        return;
    }
    let choices = std::iter::once(None).chain(
        Faction::PLAYABLE
            .into_iter()
            .filter(|faction| factions.in_play[faction.index()])
            .map(Some),
    );
    choice_row(
        ui,
        "Allegiance",
        &mut factions.allegiance,
        choices,
        |choice| match choice {
            None => "None: neutral with everyone".into(),
            Some(faction) => format!("{}: home is its capital", faction.label()),
        },
    );
    choice_row(
        ui,
        "Territory",
        &mut factions.coverage,
        Coverage::ALL,
        |coverage| coverage.label().to_owned(),
    );
    choice_row(
        ui,
        "Relations",
        &mut factions.relations,
        RelationMode::ALL,
        |mode| mode.label().to_owned(),
    );
}

/// A grid row: a number setting and its value.
fn number_row<T: egui::emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
) {
    ui.label(label);
    ui.add(egui::DragValue::new(value).range(range));
    ui.end_row();
}

/// A grid row: a drop-down over a fixed set of choices.
fn choice_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    choices: impl IntoIterator<Item = T>,
    text: impl Fn(T) -> String,
) {
    ui.label(label);
    egui::ComboBox::from_id_salt(label)
        .selected_text(text(*value))
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(value, choice, text(choice));
            }
        });
    ui.end_row();
}

/// One line describing a new game, for the log.
pub(crate) fn setup_summary(seed: u64, setup: &GameSetup) -> String {
    let inflation_whole = setup.inflation_basis_points / 100;
    let inflation_fraction = setup.inflation_basis_points % 100;
    format!(
        "New map for {}, a {} captain (seed {seed}, {} systems, {} ly radius, up to {} objects per system; \
         {} cr to start, {inflation_whole}.{inflation_fraction:02}% per 100 runs inflation, {} danger; {}).",
        game_core::travel::captain_title(&game_core::travel::clean_captain_name(
            &setup.captain_name
        )),
        setup.species.label(),
        setup.galaxy.systems,
        setup.galaxy.radius_ly,
        setup.galaxy.max_objects,
        setup.starting_credits,
        format!("{:?}", setup.danger).to_lowercase(),
        faction_summary(&setup.factions),
    )
}

fn faction_summary(factions: &game_core::factions::FactionSetup) -> String {
    let names: Vec<_> = factions.factions().iter().map(|f| f.label()).collect();
    match (names.is_empty(), factions.allegiance) {
        (true, _) => "no factions".into(),
        (false, Some(faction)) => format!("{}; sworn to the {}", names.join(", "), faction.label()),
        (false, None) => names.join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, Pos2, RawInput, Rect, Vec2, epaint::Shape};

    fn frame(ctx: &egui::Context, setup: &mut GameSetup, events: Vec<Event>) -> egui::FullOutput {
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(390.0, 844.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| setup_controls(ui, setup));
            },
        )
    }

    fn text_position(output: &egui::FullOutput, label: &str) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let Shape::Text(text) = &shape.shape
                    && text.galley.job.text.contains(label)
                {
                    Some(text.pos + text.galley.size() / 2.0)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("missing {label} text"))
    }

    fn tap(ctx: &egui::Context, setup: &mut GameSetup, pos: Pos2) -> egui::FullOutput {
        let mut output = egui::FullOutput::default();
        for pressed in [true, false] {
            output = frame(
                ctx,
                setup,
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        output
    }

    #[test]
    fn phone_captain_picker_opens_and_selects_from_full_option_text() {
        let ctx = egui::Context::default();
        let mut setup = GameSetup::default();
        let first = frame(&ctx, &mut setup, vec![]);
        let button = text_position(&first, "Captain species: Human");
        tap(&ctx, &mut setup, button);
        let open = frame(&ctx, &mut setup, vec![]);
        let option = text_position(&open, "Kethari\n");
        assert!((0.0..390.0).contains(&option.x));
        assert!((0.0..844.0).contains(&option.y));
        tap(&ctx, &mut setup, option);
        assert_eq!(setup.species, Species::Kethari);
    }

    #[test]
    fn the_new_game_summary_names_the_captain() {
        let mut setup = GameSetup::default();
        assert!(setup_summary(7, &setup).starts_with("New map for Captain, a Human captain"));
        setup.captain_name = "  Ilsa   Vey ".into();
        assert!(setup_summary(7, &setup).starts_with("New map for Captain Ilsa Vey, a Human"));
    }
}
