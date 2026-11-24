//! Friend-or-foe colours for map markers and the lines drawn from them:
//! red for raiders and anyone hostile to you, green for your own fleets and
//! friends, blue-grey for everyone else. Faction identity is shown separately
//! (a small pip or fill in the faction's colour).
use egui::Color32;

/// How a ship on the map stands toward you.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    /// Your own fleets, and factions that count you Friendly or better.
    Friendly,
    /// Independents and factions with no strong feeling either way.
    Neutral,
    /// Raiders, corsairs, factions that count you Hostile or want you.
    Hostile,
}

pub const FRIENDLY: Color32 = super::GREEN;
pub const NEUTRAL: Color32 = Color32::from_rgb(150, 175, 200);
pub const HOSTILE: Color32 = Color32::from_rgb(255, 104, 84);

impl Stance {
    pub fn color(self) -> Color32 {
        match self {
            Self::Friendly => FRIENDLY,
            Self::Neutral => NEUTRAL,
            Self::Hostile => HOSTILE,
        }
    }
}
