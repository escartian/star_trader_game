//! Raider bounties: port authorities pay for every raider hull destroyed. The
//! credits come from outside the market economy (a reward, not a trade), and
//! a raider that flees pays nothing.
use super::Credits;
use crate::fleet::ShipKind;
use serde::{Deserialize, Serialize};

/// Bounty per destroyed raider, by hull (credits in cents).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaiderBounties {
    pub freighter: Credits,
    pub scout: Credits,
    pub patrol: Credits,
    pub prospector: Credits,
}

impl Default for RaiderBounties {
    fn default() -> Self {
        Self {
            freighter: Credits(10_000),
            scout: Credits(15_000),
            patrol: Credits(40_000),
            prospector: Credits(25_000),
        }
    }
}

impl RaiderBounties {
    pub fn of(&self, kind: ShipKind) -> Credits {
        match kind {
            ShipKind::Freighter => self.freighter,
            ShipKind::Scout => self.scout,
            ShipKind::Patrol => self.patrol,
            ShipKind::Prospector => self.prospector,
        }
    }

    pub(super) fn valid(&self) -> bool {
        [self.freighter, self.scout, self.patrol, self.prospector]
            .iter()
            .all(|bounty| bounty.0 <= 1_000_000_000)
    }
}
