//! The police capture: factions in play, a criminal record with the holder
//! of system 2, and the fleet flying in until that faction's patrol stops it,
//! so the system map shows the patrol and the contact card shows the choice.
use super::LocalSession;
use game_core::{SystemId, navigation::EncounterKind};
use game_protocol::ClientCommand;

impl LocalSession {
    /// Returns the policed system.
    pub fn capture_police(&mut self) -> Result<SystemId, String> {
        let system = SystemId(2);
        self.capture_factions(false)?;
        self.authority.stage_police_pursuit(system)?;
        self.command(ClientCommand::GetSession);
        self.command(ClientCommand::PlotDestination {
            ship: self.active_ship,
            system,
            body: Some(game_core::BodyId(2)),
            expected_revision: self.state.revision,
        });
        for _ in 0..40 {
            let stopped = self
                .state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.kind == EncounterKind::Police);
            if stopped {
                return Ok(system);
            }
            self.capture_resolve_contacts();
            self.command(ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision,
            });
        }
        Err(format!("No patrol stopped the fleet: {}", self.notice))
    }
}
