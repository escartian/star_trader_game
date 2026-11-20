//! Non-raider contacts: what each offers and what each choice does. Every
//! contact can be passed by (`Bypass`, shown as Decline or Refuse where that
//! reads better). Outcomes use cargo, fuel, hull and credits; credits go into
//! `NavigationState::payout` (at a new game's price level), which the host
//! settles into the player's wallet at today's prices straight away.
//!
//! | Contact | Act | Outcome |
//! | --- | --- | --- |
//! | Debris field | Investigate | 1 ore |
//! | Distress signal | Assist | −1 medicine, a rescue recorded |
//! | Derelict freighter | Investigate | 2–6 units of a random good, space allowing |
//! | Merchant convoy | Trade | up to 5 units of the most valuable cargo aboard at 120% of list price |
//! | Cargo pod | Investigate | 3 units of a random good; one time in four, a trap damages a hull |
//! | Customs inspection | Comply | a 20 cr fee; refusing risks a 100 cr fine (one time in two) |
//! | Authority cutter | Assist | 150 cr for the escort; three times in ten, damage to a hull |
//! | Anomaly | Investigate | six times in ten, survey data worth 80 cr |
//! | Comet | Investigate | up to 10 fuel per ship |
//! | Lost probe | Investigate | a 120 cr return reward |
use super::{Encounter, EncounterChoice, EncounterKind, NavigationError};
use crate::{
    economy::{Commodity, EconomyRules},
    fleet::{Fleet, FleetMovementError},
    travel::{ShipId, TravelError, TravelState},
};

/// Credits in cents, at a new game's price level.
const CUSTOMS_FEE: i64 = 2_000;
const CUSTOMS_FINE: i64 = 10_000;
const ESCORT_REWARD: i64 = 15_000;
const SURVEY_DATA: i64 = 8_000;
const PROBE_REWARD: i64 = 12_000;
/// Units a merchant convoy buys at most, and its premium over list price.
const CONVOY_UNITS: u32 = 5;
const CONVOY_PREMIUM_PERCENT: u64 = 120;
/// Share of a hull's points a trap or a skirmish costs.
const DAMAGE_PERCENT: u32 = 15;
const COMET_FUEL: u32 = 10;

impl TravelState {
    /// Applies `choice` to a non-raider contact; returns the log line.
    pub(super) fn resolve_contact(
        &mut self,
        encounter: &Encounter,
        fleet: &Fleet,
        choice: EncounterChoice,
        rules: &EconomyRules,
    ) -> Result<String, FleetMovementError> {
        use EncounterChoice as Choice;
        use EncounterKind as Kind;
        Ok(match (encounter.kind, choice) {
            (Kind::CustomsInspection, Choice::Bypass) => {
                if self.navigation.rolls().is_multiple_of(2) {
                    self.pay(-CUSTOMS_FINE)?;
                    "Refused inspection: the port authority fined you 100 cr.".into()
                } else {
                    "Refused inspection and slipped past the cutter.".into()
                }
            }
            (_, Choice::Bypass) => "Contact passed by; the course can resume.".into(),
            (Kind::DebrisField, Choice::Investigate) => {
                let id = self.free_hold(fleet).ok_or(NavigationError::CargoFull)?;
                self.ship_mut(id)?.cargo[Commodity::Ore.index()] += 1;
                "Recovered one ore from the debris field.".into()
            }
            (Kind::DistressSignal, Choice::Assist) => self.assist_distress(fleet)?,
            (Kind::DerelictFreighter, Choice::Investigate) => {
                let good = self.random_good();
                let wanted = 2 + (self.navigation.rolls() % 5) as u32;
                let loaded = self.load(fleet, good, wanted)?;
                format!(
                    "Salvaged {loaded} {} from the derelict's hold.",
                    good.label()
                )
            }
            (Kind::MerchantConvoy, Choice::Trade) => self.sell_to_convoy(fleet, rules)?,
            (Kind::CargoPod, Choice::Investigate) => {
                if self.navigation.rolls().is_multiple_of(4) {
                    let ship = self.damage_one(fleet, rules)?;
                    format!("The pod was a trap: {ship} took hull damage.")
                } else {
                    let good = self.random_good();
                    let loaded = self.load(fleet, good, 3)?;
                    format!("The pod held {loaded} {}.", good.label())
                }
            }
            (Kind::CustomsInspection, Choice::Comply) => {
                self.pay(-CUSTOMS_FEE)?;
                "Inspection passed; the 20 cr fee is paid.".into()
            }
            (Kind::AuthorityCutter, Choice::Assist) => {
                self.pay(ESCORT_REWARD)?;
                if self.navigation.rolls() % 10 < 3 {
                    let ship = self.damage_one(fleet, rules)?;
                    format!("Escorted the cutter clear for 150 cr; {ship} was hit in the skirmish.")
                } else {
                    "Escorted the cutter clear; the authority paid 150 cr.".into()
                }
            }
            (Kind::Anomaly, Choice::Investigate) => {
                if self.navigation.rolls() % 10 < 6 {
                    self.pay(SURVEY_DATA)?;
                    "Scanned the anomaly; the survey data sold for 80 cr.".into()
                } else {
                    "Scanned the anomaly: a sensor ghost, nothing of value.".into()
                }
            }
            (Kind::Comet, Choice::Investigate) => {
                let mut total = 0;
                for id in &fleet.ships {
                    let ship = self.ship_mut(*id)?;
                    let units = COMET_FUEL.min(ship.fuel_capacity.saturating_sub(ship.fuel));
                    ship.fuel += units;
                    total += units;
                }
                format!("Harvested comet ice: {total} fuel across the fleet.")
            }
            (Kind::LostProbe, Choice::Investigate) => {
                self.pay(PROBE_REWARD)?;
                "Recovered the probe; its owners paid a 120 cr reward.".into()
            }
            _ => return Err(NavigationError::InvalidChoice.into()),
        })
    }

    fn pay(&mut self, cents: i64) -> Result<(), FleetMovementError> {
        self.navigation.payout = self
            .navigation
            .payout
            .checked_add(cents)
            .ok_or(TravelError::TimeOverflow)?;
        Ok(())
    }

    fn random_good(&mut self) -> Commodity {
        Commodity::ALL[(self.navigation.rolls() % 6) as usize]
    }

    /// The first ship in the fleet with free cargo space.
    fn free_hold(&self, fleet: &Fleet) -> Option<ShipId> {
        fleet.ships.iter().copied().find(|id| {
            self.ship(*id)
                .is_ok_and(|ship| ship.cargo_used() < u64::from(ship.cargo_capacity))
        })
    }

    /// Loads up to `wanted` units of `good` into free space; fails when none is free.
    fn load(
        &mut self,
        fleet: &Fleet,
        good: Commodity,
        wanted: u32,
    ) -> Result<u32, FleetMovementError> {
        self.free_hold(fleet).ok_or(NavigationError::CargoFull)?;
        let mut left = wanted;
        for id in &fleet.ships {
            let ship = self.ship_mut(*id)?;
            let space = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
            let units = left.min(space.min(u64::from(u32::MAX)) as u32);
            ship.cargo[good.index()] += units;
            left -= units;
        }
        Ok(wanted - left)
    }

    fn assist_distress(&mut self, fleet: &Fleet) -> Result<String, FleetMovementError> {
        let rescues = self
            .navigation
            .rescues
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let id = fleet
            .ships
            .iter()
            .find(|id| {
                self.ship(**id)
                    .is_ok_and(|ship| ship.cargo[Commodity::Medicine.index()] > 0)
            })
            .ok_or(NavigationError::MissingMedicine)?;
        self.ship_mut(*id)?.cargo[Commodity::Medicine.index()] -= 1;
        self.navigation.rescues = rescues;
        Ok("Delivered one medicine; rescue recorded in the ship's log.".into())
    }

    /// Sells the fleet's most valuable cargo (by list price) to the convoy.
    fn sell_to_convoy(
        &mut self,
        fleet: &Fleet,
        rules: &EconomyRules,
    ) -> Result<String, FleetMovementError> {
        let list = |good: Commodity| {
            let total: u64 = rules
                .markets
                .iter()
                .map(|market| market.base_prices[good.index()].0)
                .sum();
            total / rules.markets.len().max(1) as u64
        };
        let held = |good: Commodity, state: &TravelState| -> u32 {
            fleet
                .ships
                .iter()
                .filter_map(|id| state.ship(*id).ok())
                .map(|ship| ship.cargo[good.index()])
                .sum()
        };
        let good = Commodity::ALL
            .into_iter()
            .filter(|good| held(*good, self) > 0)
            .max_by_key(|good| list(*good))
            .ok_or(NavigationError::NothingToSell)?;
        let mut left = CONVOY_UNITS.min(held(good, self));
        let sold = left;
        for id in &fleet.ships {
            let ship = self.ship_mut(*id)?;
            let units = left.min(ship.cargo[good.index()]);
            ship.cargo[good.index()] -= units;
            left -= units;
        }
        let price = list(good) * CONVOY_PREMIUM_PERCENT / 100;
        let total =
            i64::try_from(price * u64::from(sold)).map_err(|_| TravelError::TimeOverflow)?;
        self.pay(total)?;
        Ok(format!(
            "Sold {sold} {} to the convoy at {}.{:02} cr each.",
            good.label(),
            price / 100,
            price % 100
        ))
    }

    /// Damages the fleet's first ship by a share of its hull, never destroying it.
    fn damage_one(
        &mut self,
        fleet: &Fleet,
        rules: &EconomyRules,
    ) -> Result<String, FleetMovementError> {
        let id = *fleet.ships.first().ok_or(NavigationError::NoCourse)?;
        let ship = self.ship_mut(id)?;
        let hull = rules
            .ship_catalog
            .spec(ship.kind)
            .map_or(1, |spec| spec.combat.hull);
        let hit = (hull * DAMAGE_PERCENT / 100).max(1);
        ship.hull_damage = (ship.hull_damage + hit).min(hull.saturating_sub(1));
        Ok(ship.name.clone())
    }
}
