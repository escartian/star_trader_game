use super::{Encounter, EncounterChoice, EncounterKind, NavigationError, NavigationEvent};
use crate::{
    StarSystem,
    economy::EconomyRules,
    fleet::{FleetError, FleetId, FleetMovementError},
    travel::{Location, PlayerId, TravelError, TravelState},
};

impl TravelState {
    pub fn execute_navigation_run(
        &mut self,
        player: PlayerId,
        systems: &[StarSystem],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        self.execute_navigation_run_with(player, systems, &[])
    }

    /// [`Self::execute_navigation_run`], with `on_lane` naming fleets whose
    /// trade routes run on a chartered shipping lane: raiders never strike them.
    pub fn execute_navigation_run_with(
        &mut self,
        player: PlayerId,
        systems: &[StarSystem],
        on_lane: &[FleetId],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        self.execute_navigation_run_hunting(player, systems, on_lane, &[])
    }

    /// [`Self::execute_navigation_run_with`], with `prey` naming NPC traders
    /// out in the open that raider bands may stalk (see `raiders`).
    pub fn execute_navigation_run_hunting(
        &mut self,
        player: PlayerId,
        systems: &[StarSystem],
        on_lane: &[FleetId],
        prey: &[super::Prey],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        let mut candidate = self.clone();
        let events = candidate.resolve_run(player, systems, on_lane, prey)?;
        *self = candidate;
        Ok(events)
    }

    fn resolve_run(
        &mut self,
        player: PlayerId,
        systems: &[StarSystem],
        on_lane: &[FleetId],
        prey: &[super::Prey],
    ) -> Result<Vec<NavigationEvent>, FleetMovementError> {
        if let Some(encounter) = self.navigation.encounters.first() {
            return Err(NavigationError::EncounterPending(encounter.fleet).into());
        }
        if let Some(fleet) = self.navigation_attention(player) {
            return Err(NavigationError::NeedsOrders(fleet).into());
        }
        let run = self
            .navigation
            .run
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let ticks = self.movement_rules.ticks_per_navigation_run;
        self.tick
            .checked_add(ticks)
            .ok_or(TravelError::TimeOverflow)?;
        self.revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let mut orders = self.navigation.orders.clone();
        orders.sort_by_key(|order| order.plan.fleet.0);
        let mut ready_orders = Vec::with_capacity(orders.len());
        let mut events = Vec::new();
        for mut order in orders {
            let fleet = self
                .fleets
                .iter()
                .find(|fleet| fleet.id == order.plan.fleet)
                .ok_or(FleetError::UnknownFleet)?
                .clone();
            if fleet.owner != player {
                return Err(FleetError::NotOwner.into());
            }
            if !order.started {
                let ship = *fleet.ships.first().ok_or(FleetError::InvalidSelection)?;
                if order.plan.tick < self.tick {
                    let target = order.target;
                    let remaining = order.remaining_waypoints;
                    let Some(updated) = self.next_leg(player, ship, target, remaining, systems)?
                    else {
                        events.push(NavigationEvent::CourseBlocked(fleet.id));
                        continue;
                    };
                    order = updated;
                }
                let mut checked = self.plan_movement(player, ship, &order.target, systems)?;
                checked.revision = order.plan.revision;
                if checked != order.plan {
                    return Err(TravelError::StaleQuote.into());
                }
                for id in fleet.ships {
                    let member = self.ship_mut(id)?;
                    member.fuel = member
                        .fuel
                        .checked_sub(order.plan.fuel_per_ship)
                        .ok_or(TravelError::InsufficientFuel)?;
                    member.location = Location::InTransit(order.plan.fleet);
                }
                order.started = true;
            }
            ready_orders.push(order);
        }
        // One shared clock advance, regardless of fleet count or course length.
        self.advance(ticks)?;
        // From here on, bodies stand where their orbits put them after the run.
        let placed = crate::orbits::placed(systems, self.tick);
        let systems = placed.as_slice();
        self.navigation.run = run;
        // Wait orders deliberately survive the run; only a new course replaces one.
        self.navigation.orders.clear();
        for mut order in ready_orders {
            let roll = self.navigation.rolls();
            if roll % 100 < u64::from(self.movement_rules.encounter_chance_percent) {
                let next = self
                    .navigation
                    .next_encounter_id
                    .checked_add(1)
                    .ok_or(TravelError::TimeOverflow)?;
                let raiders = (roll >> 40).is_multiple_of(3);
                let peaceful = &EncounterKind::PEACEFUL;
                if raiders && on_lane.contains(&order.plan.fleet) {
                    // Patrolled lane: the raider contact never happens.
                    self.navigation.orders.push(order);
                    continue;
                }
                let kind = if raiders {
                    EncounterKind::Raiders
                } else {
                    peaceful[((roll >> 32) % peaceful.len() as u64) as usize]
                };
                let hostiles = match kind {
                    EncounterKind::Raiders => self.raider_group(order.plan.fleet)?,
                    _ => Vec::new(),
                };
                let encounter = Encounter {
                    id: self.navigation.next_encounter_id,
                    fleet: order.plan.fleet,
                    kind,
                    run,
                    hostiles,
                    band: None,
                    haven: false,
                    trader: None,
                    police: None,
                    rescue: None,
                };
                self.navigation.next_encounter_id = next;
                self.navigation.encounters.push(encounter.clone());
                events.push(NavigationEvent::EncounterFound(encounter));
                self.navigation.orders.push(order);
                continue;
            }
            order.remaining_ticks = order.remaining_ticks.saturating_sub(ticks);
            if order.remaining_ticks == 0 {
                let members = self
                    .fleets
                    .iter()
                    .find(|fleet| fleet.id == order.plan.fleet)
                    .ok_or(FleetError::UnknownFleet)?
                    .ships
                    .clone();
                for id in members {
                    self.ship_mut(id)?.location = order.plan.destination.clone();
                }
                events.push(NavigationEvent::Arrived(order.plan.clone()));
                if !order.remaining_waypoints.is_empty() {
                    let target = order.remaining_waypoints.remove(0);
                    let ship = *self
                        .fleets
                        .iter()
                        .find(|fleet| fleet.id == order.plan.fleet)
                        .and_then(|fleet| fleet.ships.first())
                        .ok_or(FleetError::UnknownFleet)?;
                    match self.next_leg(player, ship, target, order.remaining_waypoints, systems)? {
                        Some(next) => self.navigation.orders.push(next),
                        None => events.push(NavigationEvent::CourseBlocked(order.plan.fleet)),
                    }
                }
            } else {
                self.navigation.orders.push(order);
            }
        }
        events.extend(self.move_bands(player, run, systems, on_lane, prey)?);
        // Escort contracts run down by one run each.
        for escort in &mut self.navigation.escorts {
            escort.runs_left -= 1;
        }
        self.navigation
            .escorts
            .retain(|escort| escort.runs_left > 0);
        events.extend(self.mine(systems)?);
        events.push(NavigationEvent::RunExecuted(run));
        Ok(events)
    }

    pub fn resolve_encounter(
        &mut self,
        player: PlayerId,
        encounter_id: u64,
        choice: EncounterChoice,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let encounter = self
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.id == encounter_id)
            .ok_or(NavigationError::UnknownEncounter)?
            .clone();
        let fleet = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == encounter.fleet)
            .ok_or(FleetError::UnknownFleet)?
            .clone();
        if fleet.owner != player {
            return Err(FleetError::NotOwner.into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        // A police patrol or a corsair's shakedown is fought or evaded like
        // raiders, and a stalked trader's band is fought; complying (or
        // leaving the trader) needs the wallet or a hold, so `police::resolve`
        // and `approach::resolve` handle it.
        if matches!(
            encounter.kind,
            EncounterKind::Raiders
                | EncounterKind::Police
                | EncounterKind::Shakedown
                | EncounterKind::TraderDistress
        ) {
            if !matches!(
                choice,
                EncounterChoice::Fight | EncounterChoice::Evade | EncounterChoice::AutoResolve
            ) {
                return Err(NavigationError::InvalidChoice.into());
            }
            let event = self.resolve_raiders(&encounter, &fleet, choice, rules)?;
            self.revision = revision;
            return Ok(event);
        }
        if encounter.kind == EncounterKind::Trader {
            // Demand needs the trader's hold: `piracy::resolve` handles it.
            let outcome = match choice {
                EncounterChoice::Fight | EncounterChoice::AutoResolve => {
                    let auto = choice == EncounterChoice::AutoResolve;
                    let event = self.start_battle(&encounter, &fleet, false, auto, rules)?;
                    self.navigation.piracies = self
                        .navigation
                        .piracies
                        .checked_add(1)
                        .ok_or(TravelError::TimeOverflow)?;
                    self.revision = revision;
                    return Ok(event);
                }
                EncounterChoice::Bypass => {
                    let name = encounter.trader.as_ref().map_or("The trader", |t| &t.name);
                    format!("{name} goes on its way.")
                }
                _ => return Err(NavigationError::InvalidChoice.into()),
            };
            self.navigation
                .encounters
                .retain(|record| record.id != encounter_id);
            self.revision = revision;
            return Ok(NavigationEvent::EncounterResolved {
                fleet: encounter.fleet,
                outcome,
            });
        }
        let outcome = self.resolve_contact(&encounter, &fleet, choice, rules)?;
        self.navigation
            .encounters
            .retain(|record| record.id != encounter_id);
        self.revision = revision;
        Ok(NavigationEvent::EncounterResolved {
            fleet: encounter.fleet,
            outcome,
        })
    }
}
