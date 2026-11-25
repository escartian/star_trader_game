//! Job boards: a port posts a few offers when one of your ships is there
//! and its last board has expired (`refresh_runs` after posting), drawn from
//! the world around it (its `reach_systems` nearest systems): goods another
//! port there needs, a raider band nearby, a trader bound for a port, a body
//! to survey. Only boards where you have been recently are kept, so saves
//! stay small in the largest galaxies. Offers are a pure function of the
//! saved RNG state, the run and the port. Accepted jobs are never touched.
use super::{Job, JobKind, KindRules, Task};
use crate::economy::{Commodity, Credits, Economy, Market, MarketId, TradeSide};
use crate::movement::MoveTarget;
use crate::travel::TravelState;
use crate::{BodyKind, StarSystem};

/// Drops expired boards, then posts one at each port where your ships are
/// and none stands. Returns whether anything changed.
pub fn refresh_offers(state: &mut TravelState, economy: &Economy, systems: &[StarSystem]) -> bool {
    let catalog = &economy.rules.job_catalog;
    let run = state.navigation.run;
    let fresh = |posted: u64| run < posted.saturating_add(catalog.refresh_runs);
    let before = state.jobs.boards.len();
    state.jobs.boards.retain(|(_, posted)| fresh(*posted));
    let standing: Vec<MarketId> = state
        .jobs
        .boards
        .iter()
        .map(|(market, _)| *market)
        .collect();
    state
        .jobs
        .offers
        .retain(|job| standing.contains(&job.issuer));
    let mut changed = state.jobs.boards.len() != before;
    let mut here: Vec<MarketId> = state
        .ships
        .iter()
        .filter_map(|ship| economy.current_market(ship).ok())
        .map(|market| market.id)
        .filter(|market| !standing.contains(market))
        .collect();
    here.sort_by_key(|market| (market.system.0, market.body.0));
    here.dedup();
    for id in here {
        let Ok(market) = economy.market(id) else {
            continue;
        };
        let reach = reach(market, catalog.reach_systems as usize, systems);
        for slot in 0..catalog.offers_per_port {
            let seed = state.navigation.rng_state
                ^ run.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (id.system.0 << 32)
                ^ (id.body.0 << 8)
                ^ u64::from(slot);
            let board = Board {
                rng: Rng(seed),
                state,
                economy,
                systems,
                issuer: market,
                reach: &reach,
            };
            if let Some(mut job) = board.post() {
                job.id = state.jobs.next_id;
                state.jobs.next_id = state.jobs.next_id.saturating_add(1);
                state.jobs.offers.push(job);
            }
        }
        state.jobs.boards.push((id, run));
        changed = true;
    }
    changed
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// A value in `0..bound` (0 when `bound` is 0).
    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() % bound as u64) as usize
        }
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        (!items.is_empty()).then(|| &items[self.below(items.len())])
    }
}

/// One port's board while it posts an offer.
struct Board<'a> {
    rng: Rng,
    state: &'a TravelState,
    economy: &'a Economy,
    systems: &'a [StarSystem],
    issuer: &'a Market,
    /// The issuer's system and its nearest neighbours.
    reach: &'a [&'a StarSystem],
}

/// The issuer's system and its `count - 1` nearest neighbours.
fn reach<'a>(issuer: &Market, count: usize, systems: &'a [StarSystem]) -> Vec<&'a StarSystem> {
    let Some(home) = systems.iter().find(|system| system.id == issuer.id.system) else {
        return Vec::new();
    };
    let mut near: Vec<&StarSystem> = systems.iter().collect();
    near.sort_by_key(|system| distance_squared(home, system));
    near.truncate(count);
    near
}

/// A task with its size (units, raiders or light-years) and text fields.
struct Draft {
    task: Task,
    size: u64,
    goods: Credits,
    fields: Vec<(&'static str, String)>,
}

impl Board<'_> {
    fn post(mut self) -> Option<Job> {
        let catalog = &self.economy.rules.job_catalog;
        let total: u32 = catalog.kinds.iter().map(|rules| rules.weight).sum();
        let mut roll = self.rng.below(total as usize) as u32;
        let mut kind = JobKind::Deliver;
        for rules in &catalog.kinds {
            if roll < rules.weight {
                kind = rules.kind;
                break;
            }
            roll -= rules.weight;
        }
        // A kind with nothing to offer here falls back to a delivery.
        let (kind, draft) = match self.draft(kind) {
            Some(draft) => (kind, draft),
            None => (JobKind::Deliver, self.draft(JobKind::Deliver)?),
        };
        Some(self.finish(catalog.rules(kind), draft))
    }

    fn draft(&mut self, kind: JobKind) -> Option<Draft> {
        match kind {
            JobKind::Deliver => self.deliver(),
            JobKind::Hunt => self.hunt(),
            JobKind::Escort => self.escort(),
            JobKind::Survey => self.survey(),
        }
    }

    fn system(&self, id: crate::SystemId) -> Option<&StarSystem> {
        self.systems.iter().find(|system| system.id == id)
    }

    fn in_reach(&self, id: crate::SystemId) -> bool {
        self.reach.iter().any(|system| system.id == id)
    }

    /// Light-years from the issuer's system to `id` (at least 1).
    fn light_years(&self, id: crate::SystemId) -> u64 {
        match (self.system(self.issuer.id.system), self.system(id)) {
            (Some(home), Some(there)) => {
                (distance_squared(home, there) as f64).sqrt().ceil() as u64
            }
            _ => 1,
        }
        .max(1)
    }

    fn deliver(&mut self) -> Option<Draft> {
        let targets: Vec<&Market> = self
            .economy
            .markets
            .iter()
            .filter(|market| market.id != self.issuer.id && self.in_reach(market.id.system))
            .collect();
        let target = *self.rng.pick(&targets).unwrap_or(&self.issuer);
        let commodity = *self.rng.pick(&Commodity::ALL)?;
        let hold = self.economy.rules.cargo_capacity.max(4);
        let quantity = 4 + self.rng.below(hold.min(16) as usize - 3) as u32;
        let good = &target.goods[commodity.index()];
        let goods = Credits(
            good.price(TradeSide::Buy, good.stock)
                .0
                .saturating_mul(u64::from(quantity)),
        );
        let mut fields = self.place_fields(target.id);
        fields.push(("cargo", commodity.label().to_owned()));
        fields.push(("units", quantity.to_string()));
        Some(Draft {
            task: Task::Deliver {
                commodity,
                quantity,
                market: target.id,
            },
            size: u64::from(quantity),
            goods,
            fields,
        })
    }

    fn hunt(&mut self) -> Option<Draft> {
        let bands: Vec<_> = self
            .state
            .navigation
            .bands
            .iter()
            .filter(|band| band.transit.is_none() && self.in_reach(band.system))
            .collect();
        let band = *self.rng.pick(&bands)?;
        let mut fields = vec![
            ("system", self.system(band.system)?.name.clone()),
            ("ships", band.ships.len().to_string()),
        ];
        fields.extend(
            self.place_fields(self.issuer.id)
                .into_iter()
                .filter(|(key, _)| *key == "port"),
        );
        Some(Draft {
            task: Task::Hunt {
                band: band.id,
                system: band.system,
            },
            size: band.ships.len() as u64,
            goods: Credits(0),
            fields,
        })
    }

    fn escort(&mut self) -> Option<Draft> {
        let mut candidates = Vec::new();
        for trader in &self.economy.traders {
            let Some(here) = trader
                .state
                .ships
                .first()
                .and_then(|ship| ship.location.system())
            else {
                continue;
            };
            let Some(order) = trader.state.navigation.orders.first() else {
                continue;
            };
            let last = order.remaining_waypoints.last().unwrap_or(&order.target);
            let MoveTarget::Body { system, body } = *last else {
                continue;
            };
            let market = MarketId { system, body };
            if system != here && self.in_reach(here) && self.economy.market(market).is_ok() {
                candidates.push((trader, market));
            }
        }
        let (trader, market) = *self.rng.pick(&candidates)?;
        let mut fields = self.place_fields(market);
        fields.push(("trader", trader.name.clone()));
        Some(Draft {
            task: Task::Escort {
                trader: trader.id,
                market,
            },
            size: self.light_years(market.system),
            goods: Credits(0),
            fields,
        })
    }

    fn survey(&mut self) -> Option<Draft> {
        let mut bodies = Vec::new();
        for system in self.reach {
            for body in &system.bodies {
                let market = MarketId {
                    system: system.id,
                    body: body.id,
                };
                if body.kind != BodyKind::Station && self.economy.market(market).is_err() {
                    bodies.push((system, body));
                }
            }
        }
        let (system, body) = *self.rng.pick(&bodies)?;
        Some(Draft {
            task: Task::Survey {
                system: system.id,
                body: body.id,
            },
            size: self.light_years(system.id),
            goods: Credits(0),
            fields: vec![
                ("system", system.name.clone()),
                ("body", place(system, &body.name)),
            ],
        })
    }

    /// {port} and {system} for a market.
    fn place_fields(&self, market: MarketId) -> Vec<(&'static str, String)> {
        let Some(system) = self.system(market.system) else {
            return Vec::new();
        };
        let port = system
            .bodies
            .iter()
            .find(|body| body.id == market.body)
            .map_or_else(|| system.name.clone(), |body| place(system, &body.name));
        vec![("port", port), ("system", system.name.clone())]
    }

    fn finish(&mut self, rules: &KindRules, draft: Draft) -> Job {
        let deadline = self
            .state
            .navigation
            .run
            .saturating_add(rules.deadline_runs);
        let mut fields = draft.fields;
        fields.push(("deadline", deadline.to_string()));
        let title = fill(
            self.rng.pick(&rules.titles).map_or("", String::as_str),
            &fields,
        );
        let brief = fill(
            self.rng.pick(&rules.briefs).map_or("", String::as_str),
            &fields,
        );
        let reward = rules
            .reward
            .0
            .saturating_add(rules.reward_step.0.saturating_mul(draft.size))
            .saturating_add(draft.goods.0);
        Job {
            id: 0,
            kind: rules.kind,
            issuer: self.issuer.id,
            faction: self.economy.factions.owner(self.issuer.id.system),
            task: draft.task,
            reward: Credits(reward),
            standing: rules.standing,
            penalty: rules.penalty,
            deadline,
            title,
            brief,
        }
    }
}

fn distance_squared(a: &StarSystem, b: &StarSystem) -> i64 {
    let (dx, dy, dz) = (
        a.position.x - b.position.x,
        a.position.y - b.position.y,
        a.position.z - b.position.z,
    );
    dx.saturating_mul(dx)
        .saturating_add(dy.saturating_mul(dy))
        .saturating_add(dz.saturating_mul(dz))
}

/// A body's name that says where it is: generated names already start with
/// their system's ("Sol Inner world"); others get it added ("Terra, Sol").
pub(crate) fn place(system: &StarSystem, body: &str) -> String {
    if body.starts_with(&system.name) {
        body.to_owned()
    } else {
        format!("{body}, {}", system.name)
    }
}

/// Replaces each `{key}` in `template` with its value.
fn fill(template: &str, fields: &[(&'static str, String)]) -> String {
    let mut text = template.to_owned();
    for (key, value) in fields {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}
