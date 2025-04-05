use rand::distributions::{Distribution, Standard};

use rand::{Rng};
use serde::{Deserialize, Serialize};

use super::position::{Position, random_position};
//STAR DETAILS

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Star {
    pub name: String,
    pub star_type: StarType,
    position: Position,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StarType {
    Nebula,
    Protostar,
    YellowDwarf,
    RedDwarf,
    WhiteDwarf,
    BrownDwarf,
    BlueGiant,
    BlueSuperiant,
    RedGiant,
    RedSuperGiant,
    NeutronStar,
    BlackHole,
}

//Using a range of numbers instead of 1 to 1 allows for more fine tuning of random generation
impl Distribution<StarType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StarType {
        let value = rng.gen_range(0..107);
        match value {
            0..=4 => StarType::Nebula,
            5..=10 => StarType::Protostar,
            11..=30 => StarType::YellowDwarf,
            31..=50 => StarType::RedDwarf,
            51..=70 => StarType::WhiteDwarf,
            71..=80 => StarType::BrownDwarf,
            81..=85 => StarType::BlueGiant,
            86..=90 => StarType::BlueSuperiant,
            91..=95 => StarType::RedGiant,
            96..=99 => StarType::RedSuperGiant,
            100..=105 => StarType::NeutronStar,
            _ => StarType::BlackHole
        }
    }
}

pub fn generate_star(map_width: i32, map_height: i32, map_length: i32) -> Star {
    let name = generate_star_name();
    let star_type: StarType = rand::random();
    // For now, always place the star at the center (0,0,0)
    // In the future, we can add special cases for binary/trinary systems
    let position = Position { x: 0, y: 0, z: 0 };
    Star {
        name,
        star_type,
        position
    }
}

fn generate_star_name() -> String {
    let mut rng = rand::thread_rng();

    // Star type prefixes
    let star_type_prefixes = vec![
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
    ];

    // Star type suffixes
    let star_type_suffixes = vec![
        "Cerberus",
        "Phoenix",
        "Manticore",
        "Chimera",
        "Griffin",
        "Basilisk",
        "Hydra",
        "Minotaur",
        "Kraken",
        "Sphinx",
    ];

    // Star names
    let star_names = vec![
        "Achernar",
        "Aldebaran",
        "Algol",
        "Altair",
        "Antares",
        "Arcturus",
        "Betelgeuse",
        "Canopus",
        "Capella",
        "Castor",
        "Deneb",
        "Fomalhaut",
        "Pollux",
        "Procyon",
        "Regulus",
        "Rigel",
        "Sirius",
        "Spica",
        "Vega",
        "Zubenelgenubi",
    ];

    let star_type_prefix = star_type_prefixes[rng.gen_range(0..star_type_prefixes.len())];
    let star_type_suffix = star_type_suffixes[rng.gen_range(0..star_type_suffixes.len())];
    let star_name = star_names[rng.gen_range(0..star_names.len())];

    format!("{} {} {}", star_type_prefix, star_name, star_type_suffix)
}
