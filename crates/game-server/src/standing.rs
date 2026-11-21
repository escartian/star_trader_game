//! Faction standing earned and lost by what you do (see `game_core::factions`).
//!
//! Around every accepted command the host compares the world before and after:
//! - each new act of piracy (`NavigationState::piracies`) costs the holder of
//!   the system where your hailing fleet stood its `piracy_penalty`;
//! - a newly finished battle against raiders (not a trader you attacked) earns
//!   the holder of the battle's system `raider_kill_standing` per raider destroyed;
//! - sales already earned their points in `Economy::trade`.
//!
//! Then every faction whose standing moved, for any of these reasons, gets a
//! [`SessionEvent::StandingChanged`]. Independent space wins and loses nothing.
use super::Session;
use game_core::{SystemId, combat::Side, fleet::FleetId, navigation::EncounterKind};
use game_protocol::{ServerMessage, SessionEvent};

/// What standing compares across one command.
pub(super) struct Watch {
    standing: [i32; 4],
    piracies: u64,
    /// The encounter of the last combat report, if any.
    combat: Option<u64>,
    /// Where the fleet hailing a trader stands, if a hail is pending.
    hail: Option<SystemId>,
}

impl Session {
    pub(super) fn watch_standing(&self) -> Watch {
        let navigation = &self.state.navigation;
        let hail = navigation
            .encounters
            .iter()
            .find(|contact| contact.kind == EncounterKind::Trader)
            .and_then(|contact| self.fleet_system(contact.fleet));
        Watch {
            standing: self.economy.factions.standing,
            piracies: navigation.piracies,
            combat: navigation
                .last_combat
                .as_ref()
                .map(|report| report.encounter),
            hail,
        }
    }

    pub(super) fn record_standing(
        &mut self,
        watch: Watch,
        message: ServerMessage,
    ) -> ServerMessage {
        if !self.economy.factions.is_active() {
            return message;
        }
        let catalog = self.economy.rules.faction_catalog.clone();
        let navigation = &self.state.navigation;
        let acts = navigation.piracies.saturating_sub(watch.piracies);
        if let Some(faction) = watch
            .hail
            .and_then(|system| self.economy.factions.owner(system))
        {
            let penalty = i64::from(catalog.get(faction).piracy_penalty) * acts as i64;
            let delta = i32::try_from(-penalty).unwrap_or(i32::MIN);
            self.economy.factions.adjust(faction, delta);
        }
        let finished = navigation
            .last_combat
            .as_ref()
            .filter(|report| Some(report.encounter) != watch.combat && report.trader.is_none())
            .map(|report| {
                let destroyed = report
                    .side(Side::Hostile)
                    .filter(|ship| ship.destroyed)
                    .count();
                (report.fleet, destroyed)
            });
        if let Some((fleet, destroyed)) = finished
            && let Some(faction) = self
                .fleet_system(fleet)
                .and_then(|system| self.economy.factions.owner(system))
        {
            let per_raider = catalog.get(faction).raider_kill_standing as usize;
            let gain = i32::try_from(per_raider * destroyed).unwrap_or(i32::MAX);
            self.economy.factions.adjust(faction, gain);
        }
        let changes: Vec<SessionEvent> = self
            .economy
            .factions
            .in_play
            .iter()
            .filter_map(|faction| {
                let standing = self.economy.factions.standing(*faction);
                let change = standing - watch.standing[faction.index()];
                (change != 0).then_some(SessionEvent::StandingChanged {
                    faction: *faction,
                    change,
                    standing,
                })
            })
            .collect();
        if changes.is_empty() {
            return message;
        }
        let ServerMessage::SessionSnapshot { mut events, .. } = message else {
            return message;
        };
        events.extend(changes);
        self.snapshot(events)
    }

    /// The system the fleet's lead ship is in, if it is in one.
    fn fleet_system(&self, fleet: FleetId) -> Option<SystemId> {
        let lead = *self
            .state
            .fleets
            .iter()
            .find(|known| known.id == fleet)?
            .ships
            .first()?;
        self.state.ship(lead).ok()?.location.system()
    }
}
