//! Traffic captures: traders between systems (heading lines) and something
//! to pursue in the flagship's system. Dev tools only.
use super::LocalSession;
use game_protocol::ClientCommand;

impl LocalSession {
    /// Something to chase in the flagship's system (the pursue capture).
    pub fn capture_pursuit_quarry(&mut self) -> Result<(), String> {
        self.authority.stage_pursuit_quarry()?;
        self.command(ClientCommand::GetSession);
        Ok(())
    }

    /// Waits run by run (at most `limit`) until `count` NPC traders are
    /// between star systems with a destination, so the galaxy map shows
    /// their heading lines. Contacts are passed by.
    pub fn capture_traders_in_flight(&mut self, count: usize, limit: usize) -> Result<(), String> {
        for _ in 0..limit {
            let flying = self
                .economy
                .traders
                .iter()
                .filter(|trader| {
                    trader.state.ships[0].location.system().is_none()
                        && self.trader_heading(trader).is_some()
                })
                .count();
            if flying >= count {
                return Ok(());
            }
            self.capture_resolve_contacts();
            self.capture_wait_runs(1);
        }
        Err(format!(
            "Fewer than {count} traders in flight after {limit} runs"
        ))
    }
}
