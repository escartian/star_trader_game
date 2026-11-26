//! Fence work, posted at ports in systems no faction holds: a **raid**
//! (seize cargo from NPC traders, by demand or plunder) and a **smuggling
//! run** (goods into a faction port in reach, unseen by its police).
use super::{Board, Draft};
use crate::economy::{Commodity, Credits, Market};
use crate::jobs::Task;

impl Board<'_> {
    pub(super) fn raid(&mut self) -> Option<Draft> {
        let units = 6 + self.rng.below(7) as u32;
        self.raid_of(units)
    }

    /// A raid for `units` units; none in a galaxy without traders.
    pub(in crate::jobs) fn raid_of(&self, units: u32) -> Option<Draft> {
        if self.economy.traders.is_empty() {
            return None;
        }
        let mut fields = self.place_fields(self.issuer.id);
        fields.retain(|(key, _)| *key == "port");
        fields.push(("units", units.to_string()));
        Some(Draft {
            task: Task::Raid { units, taken: 0 },
            size: u64::from(units),
            goods: Credits(0),
            fields,
        })
    }

    pub(super) fn smuggle(&mut self) -> Option<Draft> {
        let targets: Vec<&Market> = self
            .economy
            .markets
            .iter()
            .filter(|market| {
                market.id != self.issuer.id
                    && self.in_reach(market.id.system)
                    && self.economy.factions.owner(market.id.system).is_some()
            })
            .collect();
        let target = *self.rng.pick(&targets)?;
        let commodity = *self.rng.pick(&Commodity::ALL)?;
        let quantity = 4 + self.rng.below(7) as u32;
        Some(self.smuggle_to(target, commodity, quantity))
    }

    /// A smuggling run of `quantity` `commodity` into `target`.
    pub(in crate::jobs) fn smuggle_to(
        &self,
        target: &Market,
        commodity: Commodity,
        quantity: u32,
    ) -> Draft {
        let mut draft = self.deliver_to(target, commodity, quantity);
        draft.task = Task::Smuggle {
            commodity,
            quantity,
            market: target.id,
        };
        let faction = self.economy.factions.owner(target.id.system).map_or_else(
            || "the local".to_owned(),
            |faction| self.economy.rules.faction_catalog.get(faction).name.clone(),
        );
        draft.fields.push(("faction", faction));
        draft
    }
}
