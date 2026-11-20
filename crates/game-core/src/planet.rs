//! Planet identity and economy templates. Gas-giant commerce takes place in orbit.
use crate::{
    BodyKind, StarSystem,
    economy::{Commodity, Credits},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EconomyType {
    Agricultural,
    Extraction,
    Industrial,
}
impl EconomyType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Agricultural => "Agricultural",
            Self::Extraction => "Extraction",
            Self::Industrial => "Industrial",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    Temperate,
    Arid,
    Frozen,
    Gas,
    Ocean,
    Jungle,
    Volcanic,
    Barren,
}
impl Biome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Temperate => "Temperate",
            Self::Arid => "Arid",
            Self::Frozen => "Frozen",
            Self::Gas => "Gas atmosphere",
            Self::Ocean => "Ocean",
            Self::Jungle => "Jungle",
            Self::Volcanic => "Volcanic",
            Self::Barren => "Barren",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Danger {
    VerySafe,
    Safe,
    Moderate,
    Dangerous,
    VeryDangerous,
    Insidious,
}
impl Danger {
    pub fn label(self) -> &'static str {
        match self {
            Self::VerySafe => "Very safe",
            Self::Safe => "Safe",
            Self::Moderate => "Moderate",
            Self::Dangerous => "Dangerous",
            Self::VeryDangerous => "Very dangerous",
            Self::Insidious => "Insidious",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetProfile {
    pub economy: EconomyType,
    pub specialization: Commodity,
    pub biome: Biome,
    pub danger: Danger,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetTemplate {
    pub kind: BodyKind,
    pub profile: PlanetProfile,
    pub market_template: usize,
    pub starting_credits: Credits,
    #[serde(deserialize_with = "crate::fleet::per_class_stock")]
    pub ship_stock: [u32; crate::fleet::SHIP_KIND_COUNT],
    /// Per-commodity multiplier (percent, commodity order) on the market
    /// template's base prices. Below 100 = local surplus (cheap), above 100 =
    /// local demand (expensive). Gives each planet its own trade identity.
    #[serde(default = "neutral_prices")]
    pub price_percent: [u32; 6],
}

fn neutral_prices() -> [u32; 6] {
    [100; 6]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetCatalog {
    pub schema_version: u32,
    pub templates: Vec<PlanetTemplate>,
}
impl Default for PlanetCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../config/planets.json"))
            .expect("embedded planet definitions must validate")
    }
}
impl PlanetCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.templates.is_empty() || self.templates.len() > 64 {
            return Err("Unsupported planet catalog".into());
        }
        for kind in [BodyKind::RockyPlanet, BodyKind::GasGiant] {
            if !self.templates.iter().any(|template| template.kind == kind) {
                return Err("Missing planet class templates".into());
            }
        }
        for template in &self.templates {
            if !matches!(template.kind, BodyKind::RockyPlanet | BodyKind::GasGiant)
                || (template.kind == BodyKind::GasGiant) != (template.profile.biome == Biome::Gas)
                || template.market_template > 63
                || template.starting_credits.0 > 1_000_000_000_000
                || template.ship_stock.iter().any(|stock| *stock > 1000)
                || template
                    .price_percent
                    .iter()
                    .any(|percent| !(20..=300).contains(percent))
            {
                return Err("Invalid planet template".into());
            }
        }
        Ok(())
    }
    pub fn template(&self, system_index: usize, kind: BodyKind) -> Option<&PlanetTemplate> {
        let matches: Vec<_> = self
            .templates
            .iter()
            .filter(|template| template.kind == kind)
            .collect();
        if matches.is_empty() {
            None
        } else {
            Some(matches[system_index % matches.len()])
        }
    }
    pub fn populate(&self, systems: &mut [StarSystem]) {
        for (index, system) in systems.iter_mut().enumerate() {
            for body in &mut system.bodies {
                body.planet = self
                    .template(index, body.kind)
                    .map(|template| template.profile.clone());
            }
        }
    }
}
