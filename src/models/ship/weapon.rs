use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Weapon {
    PhotonSingularityBeam { damage: i32 },
    QuantumEntanglementTorpedo { damage: i32 },
    NeutronBeam { damage: i32 },
    GravitonPulse { damage: i32 },
    MagneticResonanceDisruptor { damage: i32 },
}
impl Weapon {
    pub fn damage(&self) -> i32 {
        match self {
            Weapon::PhotonSingularityBeam { damage } => *damage,
            Weapon::QuantumEntanglementTorpedo { damage } => *damage,
            Weapon::NeutronBeam { damage } => *damage,
            Weapon::GravitonPulse { damage } => *damage,
            Weapon::MagneticResonanceDisruptor { damage } => *damage,
        }
    }
}