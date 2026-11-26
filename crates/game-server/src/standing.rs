//! Faction standing earned and lost by what you do (see `game_core::factions`).
//!
//! Around every accepted command the host compares the world before and after:
//! - each new act of piracy (`NavigationState::piracies`) costs the holder of
//!   the system where your hailing fleet stood its `piracy_penalty`, and makes
//!   you wanted there (`factions::wanted`); robbing a trader that flies for
//!   another faction (`npc::Trader::affiliation`) costs that faction the same
//!   way, by its own `piracy_penalty`;
//! - a newly finished battle against raiders (not a trader you attacked, nor
//!   the police) earns the holder of the battle's system `raider_kill_standing`
//!   per raider destroyed;
//! - a newly finished battle against a faction's police costs that faction
//!   its `piracy_penalty` per patrol ship destroyed; surrendering to them
//!   (their cargo impounded) clears your record instead;
//! - a won rescue (a trader's distress call answered) earns the rescued
//!   trader's faction its catalog `rescue_standing`;
//! - sales already earned their points in `Economy::trade`.
//!
//! Then every faction whose standing moved, for any of these reasons, gets a
//! [`SessionEvent::StandingChanged`], and every faction that starts wanting
//! you, wants you longer or stops wanting you gets a
//! [`SessionEvent::WantedChanged`]. Independent space wins and loses nothing.
use super::Session;
use game_core::{
    SystemId,
    combat::{CombatOutcome, Side},
    factions::Faction,
    fleet::FleetId,
    navigation::EncounterKind,
};
use game_protocol::{ServerMessage, SessionEvent};

/// What standing compares across one command.
pub(super) struct Watch {
    standing: [i32; 6],
    wanted: [u32; 6],
    piracies: u64,
    /// The encounter of the last combat report, if any.
    combat: Option<u64>,
    /// Where the fleet hailing a trader stands, if a hail is pending.
    hail: Option<SystemId>,
    /// The faction the hailed trader flies for, if any.
    hailed_for: Option<Faction>,
}

impl Session {
    pub(super) fn watch_standing(&self) -> Watch {
        let navigation = &self.state.navigation;
        let contact = navigation
            .encounters
            .iter()
            .find(|contact| contact.kind == EncounterKind::Trader);
        let hail = contact.and_then(|contact| self.fleet_system(contact.fleet));
        let hailed_for = contact
            .and_then(|contact| contact.trader.as_ref())
            .and_then(|hailed| {
                let trader = self.economy.traders.iter().find(|npc| npc.id == hailed.id);
                trader?.affiliation
            });
        Watch {
            standing: self.economy.factions.standing,
            wanted: self.economy.factions.wanted,
            piracies: navigation.piracies,
            combat: navigation
                .last_combat
                .as_ref()
                .map(|report| report.encounter),
            hail,
            hailed_for,
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
        let holder = watch
            .hail
            .and_then(|system| self.economy.factions.owner(system));
        let victim = watch.hailed_for.filter(|faction| Some(*faction) != holder);
        for faction in holder.into_iter().chain(victim) {
            let penalty = i64::from(catalog.get(faction).piracy_penalty) * acts as i64;
            let delta = i32::try_from(-penalty).unwrap_or(i32::MIN);
            self.economy.factions.adjust(faction, delta);
            let count = u32::try_from(acts).unwrap_or(u32::MAX);
            self.economy.factions.charge(faction, count, &catalog);
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
                let surrendered = report.outcome == CombatOutcome::Surrendered;
                (report.fleet, destroyed, report.police, surrendered)
            });
        // A won rescue: the rescued trader's faction is grateful.
        let rescued = navigation
            .last_combat
            .as_ref()
            .filter(|report| Some(report.encounter) != watch.combat)
            .filter(|report| report.outcome == CombatOutcome::Victory)
            .and_then(|report| report.rescue)
            .and_then(|id| self.economy.traders.iter().find(|npc| npc.id == id))
            .and_then(|npc| npc.affiliation);
        if let Some((_, destroyed, Some(faction), surrendered)) = finished {
            let penalty = i64::from(catalog.get(faction).piracy_penalty) * destroyed as i64;
            let delta = i32::try_from(-penalty).unwrap_or(i32::MIN);
            self.economy.factions.adjust(faction, delta);
            if surrendered {
                self.economy.factions.clear_wanted(faction);
            }
        } else if let Some((fleet, destroyed, None, _)) = finished
            && let Some(faction) = self
                .fleet_system(fleet)
                .and_then(|system| self.economy.factions.owner(system))
        {
            let per_raider = catalog.get(faction).raider_kill_standing as usize;
            let gain = i32::try_from(per_raider * destroyed).unwrap_or(i32::MAX);
            self.economy.factions.adjust(faction, gain);
        }
        if let Some(faction) = rescued {
            let approaches = &self.economy.rules.personality_catalog.approaches;
            let thanks = i32::try_from(approaches.rescue_standing).unwrap_or(0);
            self.economy.factions.adjust(faction, thanks);
        }
        let factions = &self.economy.factions;
        let mut changes: Vec<SessionEvent> = factions
            .in_play
            .iter()
            .filter_map(|faction| {
                let standing = factions.standing(*faction);
                let change = standing - watch.standing[faction.index()];
                (change != 0).then_some(SessionEvent::StandingChanged {
                    faction: *faction,
                    change,
                    standing,
                })
            })
            .collect();
        // Wanted status running down a run at a time is not news; starting,
        // growing and ending are.
        changes.extend(factions.in_play.iter().filter_map(|faction| {
            let (before, runs) = (
                watch.wanted[faction.index()],
                factions.wanted_runs(*faction),
            );
            (runs > before || (runs == 0 && before > 0)).then_some(SessionEvent::WantedChanged {
                faction: *faction,
                runs,
            })
        }));
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
