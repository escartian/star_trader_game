//! Solo navigation-run commands. Multiplayer readiness will wrap this same resolver.
use super::Session;
use game_core::{
    navigation::NavigationEvent,
    routes,
    travel::{PlayerId, TravelError},
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

impl Session {
    pub(super) fn navigation_command(
        &mut self,
        player: PlayerId,
        command: ClientCommand,
    ) -> Result<ServerMessage, CommandError> {
        let revision = match &command {
            ClientCommand::Move {
                expected_revision, ..
            }
            | ClientCommand::PlotDestination {
                expected_revision, ..
            }
            | ClientCommand::ExecuteNavigationRun { expected_revision }
            | ClientCommand::HoldFleet {
                expected_revision, ..
            }
            | ClientCommand::CancelCourse {
                expected_revision, ..
            }
            | ClientCommand::ResolveEncounter {
                expected_revision, ..
            }
            | ClientCommand::SetTradeRoute {
                expected_revision, ..
            }
            | ClientCommand::CancelTradeRoute {
                expected_revision, ..
            }
            | ClientCommand::BattleAction {
                expected_revision, ..
            }
            | ClientCommand::Land {
                expected_revision, ..
            }
            | ClientCommand::LiftOff {
                expected_revision, ..
            }
            | ClientCommand::SetFlagship {
                expected_revision, ..
            }
            | ClientCommand::SetPatrol {
                expected_revision, ..
            } => *expected_revision,
            _ => unreachable!("only navigation commands reach this handler"),
        };
        if revision != self.state.revision {
            return Err(TravelError::StaleQuote.into());
        }
        let mut candidate = self.clone();
        let mut paid = candidate
            .state
            .navigation
            .last_combat
            .as_ref()
            .map(|report| report.encounter);
        // A fleet given explicit orders leaves its trade route.
        let mut ordered = None;
        let mut events = match command {
            ClientCommand::PlotDestination {
                ship, system, body, ..
            } => {
                candidate.require_flagship(player, candidate.fleet_holding(ship)?)?;
                let fleet = candidate.state.plot_destination(
                    player,
                    ship,
                    system,
                    body,
                    &candidate.systems,
                )?;
                ordered = Some(fleet);
                vec![NavigationEvent::CoursePlotted(fleet)]
            }
            ClientCommand::Move { ship, target, .. } => {
                candidate.require_flagship(player, candidate.fleet_holding(ship)?)?;
                let fleet =
                    candidate
                        .state
                        .plot_course(player, ship, target, &candidate.systems)?;
                ordered = Some(fleet);
                vec![NavigationEvent::CoursePlotted(fleet)]
            }
            ClientCommand::HoldFleet { fleet, .. } => {
                candidate.state.hold_fleet(player, fleet)?;
                ordered = Some(fleet);
                vec![NavigationEvent::Holding(fleet)]
            }
            ClientCommand::CancelCourse { fleet, .. } => {
                candidate.require_flagship(player, fleet)?;
                candidate.state.cancel_course(player, fleet)?;
                ordered = Some(fleet);
                vec![NavigationEvent::CourseCancelled(fleet)]
            }
            ClientCommand::ResolveEncounter {
                encounter, choice, ..
            } => vec![candidate.state.resolve_encounter(
                player,
                encounter,
                choice,
                &self.economy.rules,
            )?],
            ClientCommand::SetTradeRoute {
                fleet, destination, ..
            } => vec![routes::start_route(
                &mut candidate.state,
                &mut candidate.economy,
                player,
                fleet,
                destination,
                &candidate.systems,
            )?],
            ClientCommand::BattleAction { action, .. } => {
                vec![
                    candidate
                        .state
                        .battle_action(player, action, &self.economy.rules)?,
                ]
            }
            ClientCommand::Land { fleet, .. } => {
                candidate.require_flagship(player, fleet)?;
                ordered = Some(fleet);
                vec![candidate.state.land(player, fleet, &candidate.systems)?]
            }
            ClientCommand::LiftOff { fleet, .. } => {
                candidate.require_flagship(player, fleet)?;
                ordered = Some(fleet);
                vec![
                    candidate
                        .state
                        .lift_off(player, fleet, &candidate.systems)?,
                ]
            }
            ClientCommand::SetFlagship { fleet, .. } => {
                vec![candidate.state.set_flagship(player, fleet)?]
            }
            ClientCommand::SetPatrol { fleet, system, .. } => {
                vec![
                    candidate
                        .state
                        .start_patrol(player, fleet, system, &candidate.systems)?,
                ]
            }
            ClientCommand::CancelTradeRoute { fleet, .. } => {
                vec![routes::cancel_route(&mut candidate.state, player, fleet)?]
            }
            ClientCommand::ExecuteNavigationRun { .. } => {
                let on_lane = candidate.lane_fleets(player);
                let mut events = candidate.state.execute_navigation_run_with(
                    player,
                    &candidate.systems,
                    &on_lane,
                )?;
                // The run moved the clock: bodies move on along their orbits.
                game_core::orbits::place(&mut candidate.systems, candidate.state.tick);
                candidate
                    .economy
                    .advance(candidate.state.movement_rules.ticks_per_navigation_run);
                candidate.economy.advance_traders(&candidate.systems)?;
                // Route fleets that reached a stop trade there and set off again.
                events.extend(routes::service_routes(
                    &mut candidate.state,
                    &mut candidate.economy,
                    player,
                    &candidate.systems,
                )?);
                // Patrol fleets set off for their next body.
                events.extend(game_core::patrols::service_patrols(
                    &mut candidate.state,
                    &mut candidate.economy,
                    player,
                    &candidate.systems,
                )?);
                // Secondary fleets settle their own contacts.
                candidate.pay_new_bounty(&mut paid)?;
                events.extend(candidate.settle_secondary_contacts(player, &mut paid)?);
                events
            }
            _ => unreachable!(),
        };
        if let Some(fleet) = ordered {
            if candidate.state.end_trade_route(fleet) {
                events.push(NavigationEvent::RouteEnded {
                    fleet,
                    reason: "Route ended by new orders.".into(),
                });
            }
            if candidate.state.end_patrol(fleet) {
                events.push(NavigationEvent::PatrolNote {
                    fleet,
                    note: "Patrol ended by new orders.".into(),
                });
            }
        }
        // A battle that ended with this command pays its raider bounty, once,
        // and a contact's reward or fee reaches the wallet.
        candidate.pay_new_bounty(&mut paid)?;
        candidate.settle_payout();
        *self = candidate;
        Ok(self.snapshot(events.into_iter().map(SessionEvent::Navigation).collect()))
    }
}
