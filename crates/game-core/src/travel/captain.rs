//! Identity of the player captain, separate from the directly commanded fleet.
use super::{PlayerId, TravelState};
use crate::{fleet::FleetId, species::FleetCaptain};

impl TravelState {
    /// The player captain's fleet. Old saves without an explicit ID use the
    /// first owned fleet, which is Fleet 1 in a new campaign.
    pub fn player_captain_fleet_id(&self, player: PlayerId) -> Option<FleetId> {
        self.player_captain_fleet
            .filter(|id| {
                self.fleets
                    .iter()
                    .any(|fleet| fleet.id == *id && fleet.owner == player)
            })
            .or_else(|| {
                self.fleets
                    .iter()
                    .filter(|fleet| fleet.owner == player)
                    .map(|fleet| fleet.id)
                    .min_by_key(|id| id.0)
            })
    }

    /// Snapshot the captain before a fleet may be removed by sale, merge or
    /// battle. The saved experience follows the player into a surviving fleet.
    pub fn player_captain_record(&self, player: PlayerId) -> Option<(FleetId, FleetCaptain)> {
        let id = self.player_captain_fleet_id(player)?;
        let captain = self.fleets.iter().find(|fleet| fleet.id == id)?.captain;
        Some((id, captain))
    }

    /// Keep the player with their current fleet, or move them immediately to
    /// the surviving flagship (then the first owned fleet) if it was lost.
    /// Returns the new fleet ID only when a transfer occurred.
    pub fn reseat_player_captain(
        &mut self,
        player: PlayerId,
        previous: (FleetId, FleetCaptain),
    ) -> Option<FleetId> {
        if self
            .fleets
            .iter()
            .any(|fleet| fleet.id == previous.0 && fleet.owner == player)
        {
            self.player_captain_fleet = Some(previous.0);
            return None;
        }
        let next = self.flagship(player);
        self.player_captain_fleet = next;
        if let Some(id) = next
            && let Some(fleet) = self.fleets.iter_mut().find(|fleet| fleet.id == id)
        {
            fleet.captain = previous.1;
        }
        next
    }
}
