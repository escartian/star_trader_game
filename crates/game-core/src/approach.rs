//! NPC traders that come to you.
//!
//! After each navigation run (and after the police have moved), an NPC trader
//! within hailing range of your flagship ([`piracy::hail_check`]) may approach
//! it: one chance per run in its personality's `approach_percent`, rolled
//! from the trader's saved random state and the run (nothing changes when it
//! stays away), and not again for the catalog's `cooldown_runs`. At most one
//! trader approaches per run, lowest id first. Traders never demand cargo:
//! only raiders shake fleets down.
//!
//! - A trader with cargo aboard offers its hold: **Trade** buys as much as fits and you can
//!   afford, most valuable goods first, at `offer_price_percent` of the
//!   galaxy list price (scaled by inflation); the money goes to the trader's
//!   wallet. **Bypass** lets it go.
//! - Before any of that, a trader that a raider band stalks calls for help,
//!   one run in `distress_percent` (see [`rescue`]): **Fight** or
//!   **Auto-resolve** the band, or **Bypass** and leave the trader to it.
mod rescue;
use crate::{
    StarSystem,
    economy::{Commodity, Credits, Economy},
    fleet::{FleetError, FleetId, FleetMovementError},
    navigation::{Encounter, EncounterChoice, EncounterKind, NavigationError, NavigationEvent},
    npc::Trader,
    piracy,
    travel::{PlayerId, TravelError, TravelState},
};
pub use rescue::settle_rescue;

/// Opens at most one NPC-initiated contact with the player's flagship.
pub fn approach_run(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let Some(flagship) = state.flagship(player) else {
        return Ok(Vec::new());
    };
    let run = state.navigation.run;
    let rules = economy.rules.personality_catalog.approaches.clone();
    let mut order: Vec<usize> = (0..economy.traders.len()).collect();
    order.sort_by_key(|index| economy.traders[*index].id);
    for index in order {
        let trader = &economy.traders[index];
        let rested = trader.last_approach == 0
            || run.saturating_sub(trader.last_approach) >= rules.cooldown_runs;
        if !rested
            || piracy::hail_check(state, economy, player, flagship, trader.id, systems).is_err()
        {
            continue;
        }
        let roll = approach_roll(trader.state.navigation.rng_state, run);
        // A stalked trader's call for help comes first (the roll's high bits).
        if let Some(band) = rescue::stalker(state, trader.id)
            && (roll >> 32) % 100 < rules.distress_percent
        {
            let band = band.clone();
            let mut npc = trader.clone();
            let encounter = rescue::open_distress(state, flagship, &npc, &band)?;
            npc.last_approach = run;
            npc.last_action = "Stalked by raiders; calling nearby ships for help".into();
            economy.traders[index] = npc;
            return Ok(vec![NavigationEvent::EncounterFound(encounter)]);
        }
        let chance = economy
            .rules
            .personality_catalog
            .get(trader.personality)
            .approach_percent;
        if roll % 100 >= chance {
            continue;
        }
        if trader.state.ships[0].cargo_used() == 0 {
            continue;
        }
        let mut npc = trader.clone();
        let encounter = open_offer(state, flagship, &npc)?;
        npc.last_approach = run;
        npc.last_action = "Came alongside a fleet to trade".into();
        economy.traders[index] = npc;
        return Ok(vec![NavigationEvent::EncounterFound(encounter)]);
    }
    Ok(Vec::new())
}

/// A trader's approach roll for `run`: a pure function of its saved random
/// state, so a run without an approach changes nothing, and reloading the
/// same save cannot roll again (splitmix64 of the state mixed with the run).
fn approach_roll(rng_state: u64, run: u64) -> u64 {
    let mut value = rng_state ^ run.wrapping_mul(0xa24b_aed4_963e_e407);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Opens `npc`'s trade offer to the flagship `fleet`.
fn open_offer(
    state: &mut TravelState,
    fleet: FleetId,
    npc: &Trader,
) -> Result<Encounter, FleetMovementError> {
    let navigation = &mut state.navigation;
    let next = navigation
        .next_encounter_id
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let (contact, hostiles) = piracy::contact_of(npc);
    let encounter = Encounter {
        id: navigation.next_encounter_id,
        fleet,
        kind: EncounterKind::TraderOffer,
        run: navigation.run,
        hostiles,
        band: None,
        haven: false,
        trader: Some(contact),
        police: None,
        rescue: None,
    };
    navigation.next_encounter_id = next;
    navigation.encounters.push(encounter.clone());
    Ok(encounter)
}

/// Resolves a trade offer or a distress call (see the module docs).
pub fn resolve(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    encounter_id: u64,
    choice: EncounterChoice,
) -> Result<NavigationEvent, FleetMovementError> {
    let encounter = state
        .navigation
        .encounters
        .iter()
        .find(|encounter| encounter.id == encounter_id)
        .ok_or(NavigationError::UnknownEncounter)?
        .clone();
    let fleet = state
        .fleets
        .iter()
        .find(|record| record.id == encounter.fleet)
        .ok_or(FleetError::UnknownFleet)?
        .clone();
    if fleet.owner != player {
        return Err(FleetError::NotOwner.into());
    }
    if encounter.kind == EncounterKind::TraderDistress {
        let name = encounter
            .rescue
            .and_then(|id| piracy::find(economy, id).ok())
            .map_or_else(|| "the trader".to_owned(), |npc| npc.name.clone());
        let outcome = match choice {
            EncounterChoice::Fight | EncounterChoice::AutoResolve => {
                return state.resolve_encounter(player, encounter_id, choice, &economy.rules);
            }
            EncounterChoice::Bypass => format!("You leave {name} to the raiders."),
            _ => return Err(NavigationError::InvalidChoice.into()),
        };
        return close(state, &encounter, outcome);
    }
    let trader = encounter
        .trader
        .clone()
        .ok_or(NavigationError::InvalidChoice)?;
    let outcome = match (encounter.kind, choice) {
        (EncounterKind::TraderOffer, EncounterChoice::Trade) => {
            let (units, paid) = buy_offer(state, economy, fleet.id, trader.id)?;
            format!("Bought {units} cargo from {} for {paid} cr.", trader.name)
        }
        (EncounterKind::TraderOffer, EncounterChoice::Bypass) => {
            format!("{} goes on its way.", trader.name)
        }
        _ => return Err(NavigationError::InvalidChoice.into()),
    };
    close(state, &encounter, outcome)
}

/// Ends a settled contact with its outcome.
fn close(
    state: &mut TravelState,
    encounter: &Encounter,
    outcome: String,
) -> Result<NavigationEvent, FleetMovementError> {
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    state
        .navigation
        .encounters
        .retain(|record| record.id != encounter.id);
    state.revision = revision;
    Ok(NavigationEvent::EncounterResolved {
        fleet: encounter.fleet,
        outcome,
    })
}

/// What a trader alongside asks per unit of `good`, at today's prices.
pub fn offer_price(economy: &Economy, good: Commodity) -> Credits {
    let percent = economy
        .rules
        .personality_catalog
        .approaches
        .offer_price_percent;
    let list = by_value(economy)
        .into_iter()
        .find(|(listed, _)| *listed == good)
        .map_or(0, |(_, list)| list);
    Credits(economy.inflate(Credits(list * percent / 100)).0.max(1))
}

/// Goods from most to least valuable at list price (the average of the
/// market templates' base prices, in hundredths at a new game's prices).
fn by_value(economy: &Economy) -> Vec<(Commodity, u64)> {
    let templates = &economy.rules.markets;
    let mut goods: Vec<_> = Commodity::ALL
        .into_iter()
        .map(|good| {
            let total: u64 = templates
                .iter()
                .map(|m| m.base_prices[good.index()].0)
                .sum();
            (good, total / templates.len().max(1) as u64)
        })
        .collect();
    goods.sort_by_key(|(good, list)| (std::cmp::Reverse(*list), good.index()));
    goods
}

/// Buys what fits and what the wallet covers from the trader's hold. Returns
/// the units bought and the price paid, which goes to the trader.
fn buy_offer(
    state: &mut TravelState,
    economy: &mut Economy,
    fleet: FleetId,
    trader: u64,
) -> Result<(u32, Credits), FleetMovementError> {
    let members = fleet_ships(state, fleet)?;
    let index = piracy::position(economy, trader)?;
    let mut npc = economy.traders[index].clone();
    let mut wallet = economy.credits.0;
    let (mut units, mut paid) = (0_u32, 0_u64);
    for (good, _) in by_value(economy) {
        let price = offer_price(economy, good).0;
        for id in &members {
            let ship = state.ship_mut(*id)?;
            let room = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
            let held = u64::from(npc.state.ships[0].cargo[good.index()]);
            let bought = held.min(room).min(wallet / price);
            let bought32 = u32::try_from(bought).map_err(|_| TravelError::TimeOverflow)?;
            ship.cargo[good.index()] += bought32;
            npc.state.ships[0].cargo[good.index()] -= bought32;
            wallet -= bought * price;
            paid += bought * price;
            units += bought32;
        }
    }
    if units == 0 {
        let full = members
            .iter()
            .filter_map(|id| state.ship(*id).ok())
            .all(|ship| ship.cargo_used() >= u64::from(ship.cargo_capacity));
        return Err(if full {
            NavigationError::CargoFull
        } else {
            NavigationError::CannotAfford
        }
        .into());
    }
    npc.credits = Credits(
        npc.credits
            .0
            .checked_add(paid)
            .ok_or(TravelError::TimeOverflow)?,
    );
    npc.last_action = format!("Sold {units} cargo to a fleet at sea");
    economy.traders[index] = npc;
    economy.credits = Credits(wallet);
    Ok((units, Credits(paid)))
}

fn fleet_ships(
    state: &TravelState,
    fleet: FleetId,
) -> Result<Vec<crate::travel::ShipId>, FleetMovementError> {
    Ok(state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .ok_or(FleetError::UnknownFleet)?
        .ships
        .clone())
}
