//! Why a navigation command was refused, in words a player can act on.
use crate::fleet::FleetId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationError {
    NeedsOrders(FleetId),
    EncounterPending(FleetId),
    Underway,
    NoCourse,
    FleetHasCourse,
    UnknownEncounter,
    InvalidChoice,
    MissingMedicine,
    CargoFull,
    /// Trade routes start from the market where the fleet is.
    NotAtMarket,
    /// The other end of a route must be a different market.
    InvalidRouteStop,
    NoRoute,
    /// A route would leave with an empty hold: nothing aboard, nothing profitable.
    NothingProfitable,
    NoBattle,
    /// Landing starts from orbit around the body.
    NotInOrbit,
    /// Gas giants and stations have no surface to land on.
    CannotLand,
    NotLanded,
    /// Another raider battle must finish first.
    BattleUnderway,
    /// There is no Raider Haven standing to assault.
    NoHaven,
    /// Assaults start from orbit around (or the surface of) the Haven's body.
    NotAtHaven,
    /// The campaign is lost or out of time; only saves and loads remain.
    CampaignOver,
    /// A convoy has nothing to buy from an empty hold.
    NothingToSell,
    /// Direct orders go to the flagship; other fleets run standing orders.
    NotFlagship,
    /// The flagship takes direct orders and cannot run a patrol.
    FlagshipCommanded,
    /// A run limit must lie beyond the current run.
    InvalidRunLimit,
    /// No NPC trader has that ID.
    UnknownTrader,
    /// The flagship must be close to the trader, in open space or orbit.
    TraderOutOfReach,
    /// The trader is docked at a station or being refitted.
    TraderProtected,
    /// The hailed trader will not hand over cargo to a fleet this weak.
    WillNotYield,
    /// The wallet cannot pay for a single unit of a trader's offer.
    CannotAfford,
    /// The pursuit could not begin (the reason, e.g. out of range).
    PursuitImpossible(String),
    /// No raider band has that ID.
    UnknownBand,
    /// No police patrol has that ID.
    UnknownPatrol,
    Battle(crate::combat::tactical::BattleError),
}
impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsOrders(fleet) => write!(
                f,
                "Fleet {} needs a course or a Wait order before executing this navigation run.",
                fleet.0
            ),
            Self::EncounterPending(fleet) => write!(
                f,
                "Resolve Fleet {}'s encounter before executing another navigation run.",
                fleet.0
            ),
            Self::Underway => f.write_str(
                "This course is already underway; it continues with each navigation run.",
            ),
            Self::NoCourse => f.write_str("This fleet has no course to cancel."),
            Self::FleetHasCourse => {
                f.write_str("Cancel the plotted course first: a fleet with a course cannot change ships or land.")
            }
            Self::UnknownEncounter => {
                f.write_str("This encounter has already been resolved or is unknown.")
            }
            Self::InvalidChoice => f.write_str("That choice is unavailable for this encounter."),
            Self::MissingMedicine => {
                f.write_str("Assisting requires one medicine in this fleet's cargo.")
            }
            Self::CargoFull => {
                f.write_str("The fleet needs one free cargo space to recover salvage.")
            }
            Self::NotAtMarket => f.write_str("Start a trade route while the fleet is at a market."),
            Self::InvalidRouteStop => {
                f.write_str("Pick a different market as the other end of the route.")
            }
            Self::NoRoute => f.write_str("This fleet has no trade route."),
            Self::NothingProfitable => f.write_str(
                "Nothing profitable to carry to that market right now; the hold would be empty.",
            ),
            Self::NoBattle => f.write_str("There is no battle in progress."),
            Self::NotInOrbit => {
                f.write_str("Enter orbit around a planet, moon or asteroid to land.")
            }
            Self::CannotLand => f.write_str("Gas giants and stations have no surface to land on."),
            Self::NotLanded => f.write_str("This fleet is not on a surface."),
            Self::BattleUnderway => f.write_str("Finish the battle in progress first."),
            Self::NoHaven => f.write_str("There is no Raider Haven left to assault."),
            Self::NotFlagship => f.write_str(
                "Only your flagship takes direct orders. Give this fleet a patrol, a trade \
                 route or Wait, or make it the flagship.",
            ),
            Self::FlagshipCommanded => f.write_str(
                "The flagship takes your direct orders. Make another fleet the flagship first.",
            ),
            Self::NothingToSell => f.write_str("There is no cargo aboard to sell."),
            Self::CampaignOver => {
                f.write_str("The campaign is over. Load a save or start a new game.")
            }
            Self::InvalidRunLimit => {
                f.write_str("A campaign length must end after the current run.")
            }
            Self::NotAtHaven => {
                f.write_str("Bring the fleet into orbit at the Raider Haven's body to assault it.")
            }
            Self::UnknownTrader => f.write_str("There is no such trader."),
            Self::TraderOutOfReach => f.write_str(
                "Bring your flagship within 20 million km of the trader, in open space or \
                 orbit, to hail it.",
            ),
            Self::TraderProtected => f.write_str(
                "That trader is laid up for a refit, or docked under a station's protection: \
                 dock at the same port to hail it, and then only to trade.",
            ),
            Self::WillNotYield => f.write_str(
                "The trader will not hand over cargo to a fleet this weak; attack or let it go.",
            ),
            Self::CannotAfford => f.write_str("You cannot afford any of the trader's cargo."),
            Self::PursuitImpossible(reason) => f.write_str(reason),
            Self::UnknownBand => f.write_str("There is no such raider band."),
            Self::UnknownPatrol => f.write_str("There is no such police patrol."),
            Self::Battle(error) => error.fmt(f),
        }
    }
}
