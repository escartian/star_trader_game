//! The story roads' progress: Diplomat (a great power's treaty) and Pirate
//! (the Free Captains' chain); see `jobs::quests`.
use super::{Road, RoadProgress};
use crate::economy::Economy;
use crate::jobs::quests::{self, chain_progress};
use crate::travel::TravelState;

/// The Diplomat road: carry a great power's treaty, the last step of its
/// story quests. Progress is the furthest chain.
pub(super) fn diplomat(state: &TravelState, economy: &Economy) -> RoadProgress {
    let factions = &economy.factions;
    let best = factions
        .in_play
        .iter()
        .map(|faction| {
            let (done, total) = chain_progress(state, economy, Some(*faction));
            (done, total, *faction)
        })
        .filter(|(_, total, _)| *total > 0)
        .max_by_key(|(done, total, _)| u64::from(*done) * 1_000 / u64::from(*total));
    let complete = state.jobs.treaty.is_some();
    let (percent, detail) = match (state.jobs.treaty, best) {
        (Some(faction), _) => (100, format!("You carried the {} treaty.", faction.label())),
        (None, Some((done, total, faction))) => (
            (done * 100 / total).min(100),
            format!(
                "Win a great power's trust: its story quests open at Friendly standing and its \
                 treaty at Honored. Furthest: the {}, {done} of {total} steps.",
                faction.label()
            ),
        ),
        (None, None) => (0, "No faction in this game offers story quests.".into()),
    };
    RoadProgress {
        road: Road::Diplomat,
        open: factions.is_active() && best.is_some(),
        percent,
        complete,
        detail,
    }
}

/// The Pirate road: complete the Free Captains' chain. Its steps open as
/// your seizures (and, with factions, police escapes) mount.
pub(super) fn pirate(state: &TravelState, economy: &Economy) -> RoadProgress {
    let (done, total) = chain_progress(state, economy, None);
    let name = economy.rules.quest_catalog.pirate_name();
    let record = &state.navigation;
    let complete = total > 0 && done >= total;
    let detail = if complete {
        format!("The {name} count you one of their own.")
    } else {
        let next = match quests::pirate_needs(state, economy) {
            Some(needs) => format!("Their next step needs {needs}."),
            None => "Their next request is in Menu → Jobs, or comes with your next order.".into(),
        };
        format!(
            "Seize cargo from traders (hail them in open space, then demand or attack) to win \
             the {name}' trust. Story steps: {done} of {total}. {next} Seizures so far: {}, \
             {} units taken.",
            record.pirate_seizures, record.pirate_cargo
        )
    };
    RoadProgress {
        road: Road::Pirate,
        open: quests::pirates_active(economy),
        percent: (done * 100).checked_div(total).unwrap_or(0),
        complete,
        detail,
    }
}
