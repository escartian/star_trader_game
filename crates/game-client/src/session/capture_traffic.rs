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

    /// Pursues the first raider band and selects the first trader on the map.
    pub fn capture_pursue_band(&mut self) -> Result<(), String> {
        let band = self
            .state
            .navigation
            .bands
            .first()
            .ok_or("No raider band")?
            .id;
        self.pursue(game_core::pursuit::Quarry::Band(band));
        if self.state.navigation.pursuit.is_none() {
            return Err(format!("The pursuit did not start: {}", self.notice));
        }
        let trader = self.economy.traders.first().ok_or("No trader")?.id;
        self.select_npc(game_core::pursuit::Quarry::Trader(trader));
        Ok(())
    }

    /// Pursues the first trader between systems.
    pub fn capture_pursue_flying(&mut self) -> Result<(), String> {
        let trader = self
            .economy
            .traders
            .iter()
            .find(|npc| npc.state.ships[0].location.system().is_none())
            .ok_or("No trader between systems")?
            .id;
        self.pursue(game_core::pursuit::Quarry::Trader(trader));
        match self.state.navigation.pursuit {
            Some(_) => Ok(()),
            None => Err(format!("The pursuit did not start: {}", self.notice)),
        }
    }
}
