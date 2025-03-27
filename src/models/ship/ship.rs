use std::ptr::null;

use crate::constants::MAP_HEIGHT;
use crate::constants::MAP_LENGTH;
use crate::constants::MAP_WIDTH;
use crate::models::position::Position;
use crate::models::resource::Resource;
use crate::models::resource::ResourceType;
use crate::models::ship::shield::Shield;
use crate::models::ship::armor::Armor;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::weapon::Weapon;

//SHIP DETAILS
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ship {
    pub name: String,
    pub owner: String,
    pub position: Position,
    pub status: ShipStatus,
    pub hp: i32,
    pub combat_state: CombatState,
    pub specialization: ShipType,
    pub size: ShipSize,
    pub engine: ShipEngine,
    pub weapons: Vec<Weapon>,
    pub cargo: Vec<Resource>,
    pub shields: Shield,
    pub armor: Armor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShipStatus {
    OnPlanetRough,
    Docked,
    Launching,
    Landing,
    OrbitingPlanet,
    SubLightTravel,
    Warp,
    Stationary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CombatState {
    NotInCombat,
    Aggressive,
    Default,
    Evasive,
    Passive,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShipType {
    Fighter,
    Battleship,
    Freighter,
    Explorer,
    Shuttle,
    Capital,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShipSize {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
    Planetary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShipEngine {
    Basic,
    Advanced,
    Experimental,
}

impl Distribution<ShipEngine> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ShipEngine {
        match rng.gen_range(0..3) {
            0 => ShipEngine::Basic,
            1 => ShipEngine::Advanced,
            _ => ShipEngine::Experimental,
        }
    }
}

impl Distribution<ShipSize> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ShipSize {
        match rng.gen_range(0..6) {
            0 => ShipSize::Tiny,
            1 => ShipSize::Small,
            2 => ShipSize::Medium,
            3 => ShipSize::Large,
            4 => ShipSize::Huge,
            _ => ShipSize::Planetary,
        }
    }
}

impl Distribution<ShipType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ShipType {
        match rng.gen_range(0..5) {
            0 => ShipType::Fighter,
            1 => ShipType::Battleship,
            2 => ShipType::Freighter,
            3 => ShipType::Explorer,
            4 => ShipType::Shuttle,
            _ => ShipType::Capital,
        }
    }
}
fn generate_ship_weapons(specialization: &ShipType) -> Vec<Weapon> {
    let mut weapons = Vec::new();

    let photon_singularity_beam = Weapon::PhotonSingularityBeam { damage: 10 };
    let quantum_entanglement_torpedo = Weapon::QuantumEntanglementTorpedo { damage: 20 };
    let neutron_beam = Weapon::NeutronBeam { damage: 30 };
    let graviton_pulse = Weapon::GravitonPulse { damage: 40 };
    let magnetic_resonance_disruptor = Weapon::MagneticResonanceDisruptor { damage: 50 };

    match specialization {
        ShipType::Fighter => {
            weapons.push(photon_singularity_beam);
        }
        ShipType::Battleship => {
            weapons.push(photon_singularity_beam);
            weapons.push(graviton_pulse);
            weapons.push(magnetic_resonance_disruptor);
        }
        ShipType::Freighter => {
            weapons.push(photon_singularity_beam);
            weapons.push(graviton_pulse);
        }
        ShipType::Explorer => {
            weapons.push(neutron_beam);
            weapons.push(quantum_entanglement_torpedo);
            weapons.push(graviton_pulse);
        }
        ShipType::Shuttle => {
            weapons.push(photon_singularity_beam);
            weapons.push(quantum_entanglement_torpedo);
        }
        ShipType::Capital => {
            weapons.push(photon_singularity_beam);
            weapons.push(quantum_entanglement_torpedo);
            weapons.push(magnetic_resonance_disruptor);
            weapons.push(graviton_pulse);
            weapons.push(neutron_beam);
        }
    }
    weapons
}

fn generate_ship_resources() -> Vec<Resource> {
    let mut resources = Vec::new();
    let fuel_amount = 10;
    let fuel = Resource {
        resource_type: ResourceType::Fuel,
        quantity: Some(fuel_amount),
        buy: None,
        sell: None,
    };
    resources.push(fuel);

    resources
}

impl Distribution<ShipStatus> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ShipStatus {
        match rng.gen_range(0..5) {
            0 => ShipStatus::OnPlanetRough,
            1 => ShipStatus::Docked,
            2 => ShipStatus::Launching,
            3 => ShipStatus::Landing,
            4 => ShipStatus::OrbitingPlanet,
            5 => ShipStatus::SubLightTravel,
            6 => ShipStatus::Warp,
            _ => ShipStatus::Stationary,
        }
    }
}

fn generate_ship_name() -> String {
    let mut rng = rand::thread_rng();
    let prefixes = [
        "Alpha",
        "Beta",
        "Gamma",
        "Delta",
        "Epsilon",
        "Zeta",
        "Eta",
        "Theta",
        "Iota",
        "Kappa",
        "Lambda",
        "Mu",
        "Nu",
        "Xi",
        "Omicron",
        "Pi",
        "Rho",
        "Sigma",
        "Tau",
        "Upsilon",
        "Phi",
        "Chi",
        "Psi",
        "Omega",
        "Hyper",
        "Ultra",
        "Nova",
        "Star",
        "Galaxy",
        "Cosmo",
        "Celestial",
        "Stellar",
        "Interstellar",
        "Intergalactic",
        "Cosmic",
        "Lunar",
        "Solar",
        "Nebula",
        "Orion",
        "Andromeda",
        "Proxima",
        "Voyager",
        "Discovery",
        "Enterprise",
        "Millennium",
        "Falcon",
        "Serenity",
        "Firefly",
        "Battlestar",
        "Goliath",
        "Colossus",
        "Titan",
        "Leviathan",
        "Behemoth",
        "Kraken",
        "Hydra",
        "Dragon",
        "Phoenix",
        "Basilisk",
        "Minotaur",
        "Chimera",
        "Cyclops",
        "Medusa",
        "Gorgon",
        "Siren",
        "Mermaid",
        "Naiad",
        "Nereid",
        "Triton",
        "Poseidon",
        "Neptune",
        "Cthulhu",
    ];
    let suffixes = [
        "-class",
        "-type",
        "-model",
        "-series",
        "-mark",
        "-design",
        "-prototype",
        "-experimental",
        "-production",
        "-edition",
        "-variant",
        "-configuration",
        "-version",
        "-generation",
        "-tier",
        "-category",
        "-division",
        "-unit",
        "-team",
        "-squad",
        "-fleet",
        "-wing",
        "-armada",
    ];
    let num = rng.gen_range(1..=1000);

    let prefix_index = rng.gen_range(0..prefixes.len());
    let suffix_index = rng.gen_range(0..suffixes.len());

    let name = format!(
        "{} {}{}",
        prefixes[prefix_index], num, suffixes[suffix_index]
    );
    name
}

fn generate_owner_name() -> String {
    let mut rng = rand::thread_rng();
    let prefixes = vec![
        "Star",
        "Nova",
        "Galactic",
        "Cosmic",
        "Interstellar",
        "Astro",
        "Space",
        "Stellar",
        "Celestial",
        "Lunar",
    ];
    let suffixes = vec![
        "Explorer",
        "Voyager",
        "Pioneer",
        "Pathfinder",
        "Adventurer",
        "Navigator",
        "Discoverer",
        "Traveller",
        "Scout",
        "Seeker",
    ];
    let first_name = vec![
        "Adam", "Aurora", "Eva", "Max", "Alex", "Olivia", "Emma", "Lucas", "Noah", "Luna", "Aria",
        "Leo", "Nova", "Orion", "Stella",
    ];
    let last_name = vec![
        "Smith", "Garcia", "Johnson", "Miller", "Davis", "Wilson", "Martinez", "Anderson",
        "Thomas", "Jackson", "Lee", "Baker", "Gonzalez", "Wang",
    ];
    let prefix = prefixes[rng.gen_range(0..prefixes.len())];
    let suffix = suffixes[rng.gen_range(0..suffixes.len())];
    let first = first_name[rng.gen_range(0..first_name.len())];
    let last = last_name[rng.gen_range(0..last_name.len())];
    format!("{} {} {} {}", prefix, first, last, suffix)
}

impl Distribution<Ship> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Ship {
        let name = format!("{} {}-{}", 
            ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa", "Lambda", "Mu", "Nu", "Xi", "Omicron", "Pi", "Rho", "Sigma", "Tau", "Upsilon", "Phi", "Chi", "Psi", "Omega"][rng.gen_range(0..24)],
            rng.gen_range(100..1000),
            ["squad", "team", "division", "category", "configuration", "mark", "variant", "class", "type", "model"][rng.gen_range(0..10)]
        );

        let specialization = match rng.gen_range(0..6) {
            0 => ShipType::Fighter,
            1 => ShipType::Battleship,
            2 => ShipType::Freighter,
            3 => ShipType::Explorer,
            4 => ShipType::Shuttle,
            _ => ShipType::Capital,
        };

        let size = match rng.gen_range(0..6) {
            0 => ShipSize::Tiny,
            1 => ShipSize::Small,
            2 => ShipSize::Medium,
            3 => ShipSize::Large,
            4 => ShipSize::Huge,
            _ => ShipSize::Planetary,
        };

        let engine = match rng.gen_range(0..3) {
            0 => ShipEngine::Basic,
            1 => ShipEngine::Advanced,
            _ => ShipEngine::Experimental,
        };

        // Calculate base stats based on ship type and size
        let base_hp = match specialization {
            ShipType::Fighter => 50,
            ShipType::Battleship => 200,
            ShipType::Freighter => 100,
            ShipType::Explorer => 150,
            ShipType::Shuttle => 30,
            ShipType::Capital => 300,
        };

        let size_multiplier = match size {
            ShipSize::Tiny => 0.5,
            ShipSize::Small => 0.75,
            ShipSize::Medium => 1.0,
            ShipSize::Large => 1.5,
            ShipSize::Huge => 2.0,
            ShipSize::Planetary => 3.0,
        };

        let hp = (base_hp as f32 * size_multiplier) as i32;
        let shield_capacity = (hp as f32 * 1.5) as i32;
        let armor_capacity = (hp as f32 * 2.0) as i32;

        Ship {
            name,
            owner: String::new(), // Will be set by fleet
            position: Position { x: 0, y: 0, z: 0 }, // Will be set by fleet
            status: ShipStatus::Stationary,
            hp,
            combat_state: CombatState::NotInCombat,
            specialization,
            size,
            engine,
            weapons: vec![], // Will be populated by weapon generation
            cargo: vec![],
            shields: Shield::new(shield_capacity),
            armor: Armor::new(armor_capacity),
        }
    }
}

impl Ship {
    pub fn get_cargo_capacity(&self) -> u32 {
        match self.size {
            ShipSize::Tiny => 100,
            ShipSize::Small => 250,
            ShipSize::Medium => 500,
            ShipSize::Large => 1000,
            ShipSize::Huge => 2500,
            ShipSize::Planetary => 5000,
        }
    }

    pub fn get_current_cargo(&self) -> u32 {
        self.cargo.iter().map(|r| r.quantity.unwrap_or(0)).sum()
    }
}
