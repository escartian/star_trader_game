//! What each non-raider contact offers on its card: the situation, the action
//! (with why it is unavailable) and how to pass it by. Rules and outcomes live
//! in `game_core::navigation::contacts`.
use crate::theme::icons::Icon;
use game_core::{
    economy::Commodity,
    navigation::{EncounterChoice, EncounterKind},
};

pub(super) struct Offer {
    pub text: &'static str,
    pub icon: Icon,
    pub act: &'static str,
    pub choice: EncounterChoice,
    /// Why the action is unavailable, if it is.
    pub blocked: Option<&'static str>,
    pub pass: &'static str,
}

/// What the fleet has aboard, as the offers need it.
pub(crate) struct Aboard {
    pub free_space: bool,
    pub medicine: bool,
    pub cargo: bool,
}

pub(super) fn offer(kind: EncounterKind, aboard: &Aboard) -> Offer {
    use EncounterChoice as Choice;
    use EncounterKind as Kind;
    let space = (!aboard.free_space).then_some("No free cargo space in this fleet.");
    let (text, icon, act, choice, blocked, pass) = match kind {
        Kind::DebrisField => (
            "Your fleet detects recoverable debris: one ore for the taking.",
            Icon::Good(Commodity::Ore),
            "Salvage +1 ore",
            Choice::Investigate,
            space,
            "Bypass",
        ),
        Kind::DistressSignal => (
            "A stranded crew requests one medicine. Assisting records a rescue.",
            Icon::Good(Commodity::Medicine),
            "Assist −1 medicine",
            Choice::Assist,
            (!aboard.medicine).then_some("No medicine aboard this fleet."),
            "Continue past",
        ),
        Kind::DerelictFreighter => (
            "A dead freighter drifts on your course, its hold still sealed. Salvage what fits.",
            Icon::Cargo,
            "Salvage cargo",
            Choice::Investigate,
            space,
            "Leave it",
        ),
        Kind::MerchantConvoy => (
            "An independent convoy hails you, buying up to five units of your best cargo at 120% of list price.",
            Icon::Sell,
            "Sell to convoy",
            Choice::Trade,
            (!aboard.cargo).then_some("Nothing aboard to sell."),
            "Decline",
        ),
        Kind::CargoPod => (
            "A cargo pod tumbles past, unclaimed. It could hold goods, or be rigged by raiders.",
            Icon::Cargo,
            "Open the pod",
            Choice::Investigate,
            space,
            "Leave it",
        ),
        Kind::CustomsInspection => (
            "A port authority cutter orders you to heave to for inspection. The fee is 20 cr; refusing risks a 100 cr fine.",
            Icon::Info,
            "Comply · 20 cr",
            Choice::Comply,
            None,
            "Refuse",
        ),
        Kind::AuthorityCutter => (
            "An authority cutter under raider fire asks for an escort. The authority pays 150 cr, but you may take hits.",
            Icon::Weapon,
            "Escort · +150 cr",
            Choice::Assist,
            None,
            "Decline",
        ),
        Kind::Anomaly => (
            "Sensors catch an anomaly. A full scan may yield survey data worth selling, or nothing at all.",
            Icon::Star,
            "Scan",
            Choice::Investigate,
            None,
            "Ignore",
        ),
        Kind::Comet => (
            "A comet crosses your path. Its ice refines into fuel: up to 10 per ship.",
            Icon::Fuel,
            "Harvest ice",
            Choice::Investigate,
            None,
            "Bypass",
        ),
        Kind::LostProbe => (
            "A survey company's lost probe is pinging for pickup. Its owners pay 120 cr for its return.",
            Icon::Pin,
            "Recover · +120 cr",
            Choice::Investigate,
            None,
            "Bypass",
        ),
        Kind::Raiders | Kind::Trader => {
            unreachable!("raider and trader contacts have their own cards")
        }
    };
    Offer {
        text,
        icon,
        act,
        choice,
        blocked,
        pass,
    }
}

/// An empty fleet's view of a contact, for tests.
#[cfg(test)]
pub(crate) fn contact_aboard_nothing() -> Aboard {
    Aboard {
        free_space: true,
        medicine: false,
        cargo: false,
    }
}

/// The pass-by button's label on `kind`'s card, for tests.
#[cfg(test)]
pub(crate) fn contact_pass_label(kind: EncounterKind, aboard: &Aboard) -> &'static str {
    offer(kind, aboard).pass
}
