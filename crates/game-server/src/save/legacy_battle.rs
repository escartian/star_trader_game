//! Battles saved on the 3D grid (schema 7) or the banded hex board (schema 8).
//! Neither position has a fair equivalent on today's board, so an open one is
//! rebuilt on a fresh board, keeping each ship's damage, shields, withdrawal
//! and shot record, and then auto-resolved as soon as the save loads. The
//! loader reports this to the player.
use super::SaveData;
use game_core::{
    combat::{
        CombatProfile, CombatantSetup, Side,
        engagement::ShotRecord,
        tactical::{Battle, MAX_ROUNDS},
    },
    fleet::{FleetId, ShipKind},
    navigation::{ActiveBattle, BattleAction},
    travel::ShipId,
};
use serde::Deserialize;
use serde_json::Value;

/// The parts of a schema-7 `ActiveBattle` that survive the move; grid cells,
/// sizes and turn state are ignored.
#[derive(Deserialize)]
struct GridActiveBattle {
    encounter: u64,
    fleet: FleetId,
    evasion_failed: bool,
    battle: GridBattle,
}

#[derive(Deserialize)]
struct GridBattle {
    seed: u64,
    units: Vec<GridUnit>,
    rounds: Vec<Vec<ShotRecord>>,
}

#[derive(Deserialize)]
struct GridUnit {
    name: String,
    side: Side,
    ship: Option<ShipId>,
    kind: ShipKind,
    profile: CombatProfile,
    starting_damage: u32,
    damage: u32,
    shields: u32,
    withdrawn: bool,
    hits: u32,
    shots: u32,
    hull_dealt: u32,
}

pub(super) const NOTICE: &str =
    "A battle saved on an older battle board was auto-resolved on load. See the combat report.";

/// Rebuilds `saved` on the hex board and fights it out with the AI.
pub(super) fn resolve(data: &mut SaveData, saved: Value) -> Result<(), String> {
    let invalid = |error: String| format!("Invalid saved battle: {error}");
    let saved: GridActiveBattle =
        serde_json::from_value(saved).map_err(|error| invalid(error.to_string()))?;
    let rules = &data.economy.rules;
    let setups: Vec<_> = saved
        .battle
        .units
        .iter()
        .map(|unit| CombatantSetup {
            name: unit.name.clone(),
            side: unit.side,
            ship: unit.ship,
            kind: unit.kind,
            profile: unit.profile.clone(),
            hull_damage: unit.starting_damage,
        })
        .collect();
    let mut battle =
        Battle::new(&setups, &rules.combat_catalog, saved.battle.seed).map_err(invalid)?;
    for (unit, old) in battle.units.iter_mut().zip(&saved.battle.units) {
        if old.damage < old.starting_damage
            || old.damage > unit.profile.hull
            || old.shields > unit.profile.shields
            || old.hits > old.shots
        {
            return Err(invalid(format!("{} has impossible damage", old.name)));
        }
        unit.damage = old.damage;
        unit.shields = old.shields;
        unit.withdrawn = old.withdrawn;
        unit.hits = old.hits;
        unit.shots = old.shots;
        unit.hull_dealt = old.hull_dealt;
    }
    let rounds = saved.battle.rounds.len();
    if !(1..=MAX_ROUNDS as usize).contains(&rounds) {
        return Err(invalid("bad round count".into()));
    }
    battle.rounds = saved.battle.rounds;
    battle.round = rounds as u32;
    // Resume with the first ship in the initiative order still fighting.
    if let Some(turn) = battle
        .order
        .iter()
        .position(|&index| battle.units[usize::from(index)].active())
    {
        battle.turn = turn;
    }
    let owner = data
        .player
        .fleets
        .iter()
        .find(|fleet| fleet.id == saved.fleet)
        .ok_or_else(|| invalid("unknown fleet".into()))?
        .owner;
    data.player.navigation.battle = Some(Box::new(ActiveBattle {
        encounter: saved.encounter,
        fleet: saved.fleet,
        evasion_failed: saved.evasion_failed,
        battle,
    }));
    data.player
        .battle_action(owner, BattleAction::AutoResolve, &data.economy.rules)
        .map(|_| ())
        .map_err(|error| invalid(error.to_string()))
}
