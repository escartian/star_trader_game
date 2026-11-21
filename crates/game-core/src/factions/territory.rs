//! New-game territory: capitals, borders, relations and opening standing.
//!
//! - Your allegiance faction, if any, has its capital at the home system.
//!   Every other faction's capital is the system farthest from the capitals
//!   placed so far (and from home), so powers start apart. A galaxy too small
//!   for every capital leaves the last factions without territory.
//! - Every system leans to its nearest capital. The [`Coverage`] share of
//!   systems nearest their capital is claimed (capitals always); the rest are
//!   independent. Home is independent unless it is a capital.
//! - Relations are the catalog's classic pairs, or rolled from the seed: each
//!   pair allied one time in four, rivals one time in four.
//! - Declaring allegiance starts you at +30 with that faction, spread to its
//!   allies (+15) and rivals (−15) as any standing change is.
use super::{Faction, FactionCatalog, Factions, Relation};
use crate::{GalaxyPosition, StarSystem, SystemId};

/// Opening standing with your allegiance faction.
pub const ALLEGIANCE_STANDING: i32 = 30;

/// How much of the galaxy the factions claim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Coverage {
    /// 40% of systems.
    Sparse,
    /// 70% of systems.
    #[default]
    Standard,
    /// Every system but an unclaimed home.
    Full,
}

impl Coverage {
    pub const ALL: [Self; 3] = [Self::Sparse, Self::Standard, Self::Full];

    pub fn percent(self) -> usize {
        match self {
            Self::Sparse => 40,
            Self::Standard => 70,
            Self::Full => 100,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sparse => "Sparse: 40% claimed, wide independent frontier",
            Self::Standard => "Standard: 70% claimed",
            Self::Full => "Full: every system claimed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RelationMode {
    /// The catalog's fixed alliances and rivalries.
    #[default]
    Classic,
    /// Rolled from the game's seed.
    Random,
}

impl RelationMode {
    pub const ALL: [Self; 2] = [Self::Classic, Self::Random];

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic alliances and rivalries",
            Self::Random => "Random (from the seed)",
        }
    }
}

/// The faction choices of a new game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactionSetup {
    /// Which factions take part, by [`Faction::index`]; none: factions off.
    pub in_play: [bool; 4],
    pub allegiance: Option<Faction>,
    pub coverage: Coverage,
    pub relations: RelationMode,
}

impl Default for FactionSetup {
    fn default() -> Self {
        Self {
            in_play: [true; 4],
            allegiance: None,
            coverage: Coverage::default(),
            relations: RelationMode::default(),
        }
    }
}

impl FactionSetup {
    pub fn factions(&self) -> Vec<Faction> {
        Faction::ALL
            .into_iter()
            .filter(|faction| self.in_play[faction.index()])
            .collect()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self
            .allegiance
            .is_some_and(|faction| !self.in_play[faction.index()])
        {
            return Err("Your allegiance must be to a faction in play".into());
        }
        Ok(())
    }

    /// Territory, relations and opening standing for `systems` (home first).
    pub fn generate(
        &self,
        seed: u64,
        systems: &[StarSystem],
        catalog: &FactionCatalog,
    ) -> Result<Factions, String> {
        self.validate()?;
        let in_play = self.factions();
        let mut factions = Factions {
            in_play: in_play.clone(),
            allegiance: self.allegiance,
            ..Factions::default()
        };
        if in_play.is_empty() || systems.is_empty() {
            return Ok(factions);
        }
        factions.relations = self.relations(seed, &in_play, catalog);
        let capitals = capitals(&in_play, self.allegiance, systems);
        let nearest = |system: &StarSystem| {
            capitals
                .iter()
                .map(|(faction, capital)| (gap(system.position, capital.position), *faction))
                .min()
        };
        let mut claims: Vec<(i128, SystemId, Faction)> = systems
            .iter()
            .filter_map(|system| nearest(system).map(|(gap, faction)| (gap, system.id, faction)))
            .collect();
        claims.sort_by_key(|(gap, system, faction)| (*gap, system.0, *faction));
        let quota = (systems.len() * self.coverage.percent()).div_ceil(100);
        let home = systems[0].id;
        let home_is_capital = capitals.iter().any(|(_, capital)| capital.id == home);
        factions.territory = claims
            .into_iter()
            .take(quota.max(capitals.len()))
            .filter(|(_, system, _)| *system != home || home_is_capital)
            .map(|(_, system, faction)| (system, faction))
            .collect();
        factions.territory.sort_by_key(|(system, _)| system.0);
        if let Some(faction) = self.allegiance {
            factions.adjust(faction, ALLEGIANCE_STANDING);
        }
        Ok(factions)
    }

    fn relations(
        &self,
        seed: u64,
        in_play: &[Faction],
        catalog: &FactionCatalog,
    ) -> Vec<(Faction, Faction, Relation)> {
        let mut state = seed ^ 0xfac7_10a5;
        let mut pairs = Vec::new();
        for (index, a) in in_play.iter().enumerate() {
            for b in &in_play[index + 1..] {
                let relation = match self.relations {
                    RelationMode::Classic => catalog
                        .classic_relations
                        .iter()
                        .find(|pair| pair.between == [*a, *b] || pair.between == [*b, *a])
                        .map_or(Relation::Neutral, |pair| pair.relation),
                    RelationMode::Random => match splitmix(&mut state) % 4 {
                        0 => Relation::Allied,
                        1 => Relation::Rival,
                        _ => Relation::Neutral,
                    },
                };
                if relation != Relation::Neutral {
                    pairs.push((*a, *b, relation));
                }
            }
        }
        pairs
    }
}

/// Each faction's capital, in faction order; see the module notes.
fn capitals<'a>(
    in_play: &[Faction],
    allegiance: Option<Faction>,
    systems: &'a [StarSystem],
) -> Vec<(Faction, &'a StarSystem)> {
    let home = &systems[0];
    let mut placed: Vec<(Faction, &StarSystem)> = Vec::new();
    if let Some(faction) = allegiance {
        placed.push((faction, home));
    }
    for faction in in_play.iter().filter(|f| Some(**f) != allegiance) {
        let anchors: Vec<GalaxyPosition> = std::iter::once(home.position)
            .chain(placed.iter().map(|(_, capital)| capital.position))
            .collect();
        let farthest = systems
            .iter()
            .filter(|system| placed.iter().all(|(_, capital)| capital.id != system.id))
            .filter(|system| system.id != home.id)
            .max_by_key(|system| {
                let clearance = anchors.iter().map(|at| gap(system.position, *at)).min();
                (clearance, std::cmp::Reverse(system.id.0))
            });
        if let Some(system) = farthest {
            placed.push((*faction, system));
        }
    }
    placed.sort_by_key(|(faction, _)| *faction);
    placed
}

/// Squared distance in ly².
fn gap(a: GalaxyPosition, b: GalaxyPosition) -> i128 {
    [(a.x, b.x), (a.y, b.y), (a.z, b.z)]
        .iter()
        .map(|(a, b)| (i128::from(*a) - i128::from(*b)).pow(2))
        .sum()
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
