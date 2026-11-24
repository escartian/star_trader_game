//! The approach captures: the flagship brought within hailing range of a
//! trader, which then comes alongside to trade or, as a corsair, to shake the
//! battered flagship down, so the contact card shows.
use super::LocalSession;
use game_core::navigation::EncounterChoice;
use game_protocol::ClientCommand;

impl LocalSession {
    pub fn capture_approach(&mut self, shakedown: bool) -> Result<(), String> {
        // The hail capture brings a trader within reach; let that one go.
        self.capture_hail()?;
        let hail = self.state.navigation.encounters.first().cloned();
        let (Some(hail), Some(trader)) = (
            hail.clone(),
            hail.and_then(|contact| contact.trader).map(|t| t.id),
        ) else {
            return Err("The hail capture opened no contact".into());
        };
        self.command(ClientCommand::ResolveEncounter {
            encounter: hail.id,
            choice: EncounterChoice::Bypass,
            expected_revision: self.state.revision,
        });
        if !self.authority.stage_approach(trader, shakedown)? {
            return Err("The trader did not approach".into());
        }
        self.command(ClientCommand::GetSession);
        Ok(())
    }
}
