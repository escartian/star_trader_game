use super::SaveData;
use game_core::{BodyKind, economy::MarketId, planet::Biome};
use std::collections::HashSet;

impl SaveData {
    pub(super) fn validate_markets(&self) -> Result<(), String> {
        let mut expected = HashSet::new();
        let mut stations = HashSet::new();
        for system in &self.systems {
            let station = system
                .bodies
                .iter()
                .find(|body| body.kind == BodyKind::Station)
                .ok_or("Missing primary station")?;
            let id = MarketId {
                system: system.id,
                body: station.id,
            };
            expected.insert(id);
            stations.insert(id);
            for body in &system.bodies {
                let surfaced = matches!(body.kind, BodyKind::Moon | BodyKind::Asteroid);
                if body.surface.is_some() && !surfaced
                    || body.surface.as_ref().is_some_and(|surface| {
                        surface.biome == Biome::Gas || !(10..=1000).contains(&surface.ore_percent)
                    })
                {
                    return Err("Invalid body surface".into());
                }
                match (&body.planet, body.kind) {
                    (Some(profile), BodyKind::RockyPlanet | BodyKind::GasGiant) => {
                        if (body.kind == BodyKind::GasGiant) != (profile.biome == Biome::Gas) {
                            return Err("Invalid planet biome".into());
                        }
                        expected.insert(MarketId {
                            system: system.id,
                            body: body.id,
                        });
                    }
                    (None, BodyKind::Moon | BodyKind::Asteroid | BodyKind::Station) => {}
                    _ => return Err("Missing or unexpected planet profile".into()),
                }
            }
        }
        if self.economy.markets.len() != expected.len() {
            return Err("Missing body markets".into());
        }
        let mut actual = HashSet::new();
        for market in &self.economy.markets {
            if !expected.contains(&market.id)
                || !actual.insert(market.id)
                || market.is_station != stations.contains(&market.id)
                || market.credits.0 > 1_000_000_000_000_000
                || market.ship_stock.iter().any(|stock| *stock > 1000)
                || market.residents_percent.iter().sum::<u32>() != 100
            {
                return Err("Invalid market identity, treasury or ship inventory".into());
            }
            for good in &market.goods {
                if !(1..=1000).contains(&good.target)
                    || good.stock > good.capacity()
                    || good.base_price.0 == 0
                    || good.base_price.0 > 1_000_000_000
                    || good.production > 1000
                    || good.consumption > 1000
                {
                    return Err("Invalid market balances or rates".into());
                }
            }
        }
        Ok(())
    }
}
