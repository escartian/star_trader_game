//! Consistency checks for saved campaign settings and the first recorded end.
use super::super::SaveData;
use game_core::campaign::{CampaignEnd, Road};

impl SaveData {
    pub(super) fn validate_campaign(&self) -> Result<(), String> {
        let state = &self.player;
        let run = state.navigation.run;
        let limit = state.campaign.run_limit;
        let piracy = &state.navigation;
        if piracy.pirate_seizures > piracy.piracies
            || piracy.pirate_escapes > piracy.next_encounter_id
            || piracy.pirate_cargo > piracy.pirate_seizures.saturating_mul(1_000_000)
        {
            return Err("Invalid pirate progress".into());
        }
        if limit.is_some_and(|value| value == 0 || value > crate::campaign::MAX_RUN_LIMIT) {
            return Err("Invalid campaign length".into());
        }
        let valid_end = match state.campaign.ended {
            None => limit.is_none_or(|value| run < value),
            Some(CampaignEnd::TimeUp { run: ended }) => {
                ended == run && limit.is_some_and(|value| ended >= value)
            }
            Some(CampaignEnd::Lost { run: ended }) => {
                ended == run
                    && state.ships.len() == 1
                    && self
                        .economy
                        .rules
                        .ship_catalog
                        .spec(state.ships[0].kind)
                        .is_some_and(|spec| state.ships[0].hull_damage + 1 == spec.combat.hull)
            }
            Some(CampaignEnd::Won { road, run: ended }) => {
                ended <= run
                    && match road {
                        Road::Trader => true, // Net worth can fall again after a win.
                        Road::Soldier => state
                            .navigation
                            .haven
                            .as_ref()
                            .is_some_and(|haven| haven.destroyed_run == Some(ended)),
                        Road::Diplomat => state.jobs.treaty.is_some(),
                        Road::Pirate => {
                            game_core::campaign::progress(state, &self.economy, crate::LOCAL_PLAYER)
                                .iter()
                                .any(|progress| progress.road == Road::Pirate && progress.complete)
                        }
                    }
            }
        };
        if !valid_end {
            return Err("Invalid campaign end".into());
        }
        Ok(())
    }
}
