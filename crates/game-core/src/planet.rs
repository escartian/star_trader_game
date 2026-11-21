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
    /// Iron-nickel rock: metallic asteroids.
    Metallic,
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
            Self::Metallic => "Metallic",
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

/// A moon's or asteroid's surface. It has no market; it shapes the art,
/// where raiders patrol and how much ore a landed fleet mines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceProfile {
    pub biome: Biome,
    pub danger: Danger,
    /// Ore mined, as a percentage of the landing rules' per-ship yield for
    /// the body's kind (rounded down, at least 1 when the base yield is).
    pub ore_percent: u32,
}

impl SurfaceProfile {
    /// Ore per ship per run on this surface, from the kind's `base` yield.
    pub fn ore_per_ship(&self, base: u32) -> u32 {
        if base == 0 {
            return 0;
        }
        (u64::from(base) * u64::from(self.ore_percent) / 100).clamp(1, u64::from(u32::MAX)) as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceTemplate {
    pub kind: BodyKind,
    pub profile: SurfaceProfile,
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
    /// Moon and asteroid surfaces (none in older definitions).
    #[serde(default)]
    pub surfaces: Vec<SurfaceTemplate>,
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
        for surface in &self.surfaces {
            if !matches!(surface.kind, BodyKind::Moon | BodyKind::Asteroid)
                || surface.profile.biome == Biome::Gas
                || !(10..=1000).contains(&surface.profile.ore_percent)
            {
                return Err("Invalid surface template".into());
            }
        }
        if self.surfaces.len() > 64 {
            return Err("Too many surface templates".into());
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
    /// Keep authored starter systems stable; vary planets within generated systems.
    pub fn template_for_body(
        &self,
        system_index: usize,
        body: &crate::LocalBody,
    ) -> Option<&PlanetTemplate> {
        let variant = if system_index < 3 {
            system_index
        } else {
            system_index.wrapping_add(body.id.0 as usize)
        };
        self.template(variant, body.kind)
    }
    pub fn populate(&self, systems: &mut [StarSystem]) {
        for (index, system) in systems.iter_mut().enumerate() {
            for body in &mut system.bodies {
                body.planet = self
                    .template_for_body(index, body)
                    .map(|template| template.profile.clone());
                body.surface = self.surface_for_body(index, body);
            }
        }
        // Saves from before surfaces get them in `populate_surfaces`.
    }

    /// Gives moons and asteroids that lack one the surface a new game would
    /// (saves from before surfaces). Deterministic: the same pick as `populate`.
    pub fn populate_surfaces(&self, systems: &mut [StarSystem]) {
        for (index, system) in systems.iter_mut().enumerate() {
            for body in &mut system.bodies {
                if body.surface.is_none() {
                    body.surface = self.surface_for_body(index, body);
                }
            }
        }
    }

    /// The surface a moon or asteroid gets: varied by system and body ID.
    pub fn surface_for_body(
        &self,
        system_index: usize,
        body: &crate::LocalBody,
    ) -> Option<SurfaceProfile> {
        let matches: Vec<_> = self
            .surfaces
            .iter()
            .filter(|surface| surface.kind == body.kind)
            .collect();
        if matches.is_empty() {
            return None;
        }
        let variant = system_index.wrapping_add(body.id.0 as usize);
        Some(matches[variant % matches.len()].profile.clone())
    }
}
