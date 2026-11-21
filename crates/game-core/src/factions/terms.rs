//! What your standing buys you at a faction's ports.
//!
//! | Tier | Standing | Buying | Selling | Fuel and repairs | Shipyard, gear, escorts |
//! | --- | --- | --- | --- | --- | --- |
//! | Hostile | −100 to −50 | refused | refused | +50% | refused |
//! | Unfriendly | −49 to −15 | +10% | −10% | +10% | open |
//! | Neutral | −14 to 14 | list | list | list | open |
//! | Friendly | 15 to 49 | −5% | +5% | −5% | open |
//! | Honored | 50 to 100 | −10% | +10% | −10% | open |
//!
//! Terms apply to your ships only (NPC traders trade at list prices) and only
//! in claimed space: independent ports always trade at list prices. The market
//! receives or pays the adjusted amount, so credits are conserved.
use super::Faction;
use crate::{economy::Economy, npc::NPC_ID_BASE, travel::Ship};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Hostile,
    Unfriendly,
    Neutral,
    Friendly,
    Honored,
}

impl Tier {
    pub fn of(standing: i32) -> Self {
        match standing {
            ..=-50 => Self::Hostile,
            -49..=-15 => Self::Unfriendly,
            -14..=14 => Self::Neutral,
            15..=49 => Self::Friendly,
            50.. => Self::Honored,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hostile => "Hostile",
            Self::Unfriendly => "Unfriendly",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Honored => "Honored",
        }
    }

    /// Percent in your favour on prices: positive is cheaper buying, dearer
    /// selling and cheaper services.
    fn favour_percent(self) -> i64 {
        match self {
            Self::Hostile => -50,
            Self::Unfriendly => -10,
            Self::Neutral => 0,
            Self::Friendly => 5,
            Self::Honored => 10,
        }
    }
}

/// Your terms at one port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortTerms {
    /// The port's faction and your tier with it; `None` in independent space.
    pub holder: Option<(Faction, Tier)>,
    /// The holder wants you for your crimes: the port treats you as Hostile
    /// until you pay your fine (`factions::wanted`).
    pub wanted: bool,
}

impl PortTerms {
    pub const LIST: Self = Self {
        holder: None,
        wanted: false,
    };

    /// Hostile ports, and ports whose holder wants you, refuse trade, hulls,
    /// gear, escorts and charters.
    pub fn closed(self) -> bool {
        self.wanted || matches!(self.holder, Some((_, Tier::Hostile)))
    }

    fn favour(self) -> i64 {
        match self.holder {
            Some(_) if self.wanted => Tier::Hostile.favour_percent(),
            holder => holder.map_or(0, |(_, tier)| tier.favour_percent()),
        }
    }

    /// What you pay for goods listed at `cents` (rounded up).
    pub fn buying(self, cents: u64) -> u64 {
        scale(cents, 100 - self.favour(), true)
    }

    /// What you are paid for goods listed at `cents` (rounded down).
    pub fn selling(self, cents: u64) -> u64 {
        scale(cents, 100 + self.favour(), false)
    }

    /// What you pay for fuel or repairs listed at `cents` (rounded up).
    pub fn service(self, cents: u64) -> u64 {
        scale(cents, 100 - self.favour(), true)
    }

    /// A short note for the market window, e.g. "Federation port · Friendly: 5% better prices".
    pub fn summary(self) -> String {
        match self.holder {
            None => "Independent port · list prices".into(),
            Some((faction, _)) if self.wanted => format!(
                "{} port · Wanted: trade refused until you pay your fine; fuel and repairs cost 50% more",
                faction.label()
            ),
            Some((faction, Tier::Hostile)) => format!(
                "{} port · Hostile: trade refused; fuel and repairs cost 50% more",
                faction.label()
            ),
            Some((faction, Tier::Neutral)) => {
                format!("{} port · Neutral: list prices", faction.label())
            }
            Some((faction, tier)) => {
                let favour = tier.favour_percent();
                let (amount, word) = if favour > 0 {
                    (favour, "better")
                } else {
                    (-favour, "worse")
                };
                format!(
                    "{} port · {}: {amount}% {word} prices",
                    faction.label(),
                    tier.label()
                )
            }
        }
    }
}

fn scale(cents: u64, percent: i64, round_up: bool) -> u64 {
    let percent = u128::try_from(percent.max(0)).unwrap_or(0);
    let product = u128::from(cents) * percent;
    let value = if round_up {
        product.div_ceil(100)
    } else {
        product / 100
    };
    u64::try_from(value).unwrap_or(u64::MAX)
}

impl Economy {
    /// Your terms at the market `ship` is at; list terms for NPC ships, in
    /// independent space or away from a market.
    pub fn port_terms(&self, ship: &Ship) -> PortTerms {
        if ship.owner.0 >= NPC_ID_BASE {
            return PortTerms::LIST;
        }
        let Some(system) = self
            .current_market(ship)
            .ok()
            .map(|market| market.id.system)
        else {
            return PortTerms::LIST;
        };
        let holder = self.factions.owner(system);
        PortTerms {
            holder: holder.map(|faction| (faction, self.factions.tier(faction))),
            wanted: holder.is_some_and(|faction| self.factions.is_wanted(faction)),
        }
    }

    /// Refuses business a hostile port turns away.
    pub fn require_open_port(&self, ship: &Ship) -> Result<(), crate::economy::EconomyError> {
        if self.port_terms(ship).closed() {
            return Err(crate::economy::EconomyError::PortClosed);
        }
        Ok(())
    }
}
