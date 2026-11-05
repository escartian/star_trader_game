use game_core::{GalaxyPosition, StarKind, StarSystem, SystemId};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Session,
    Overlap,
    Dense,
}

impl Scene {
    pub fn label(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Overlap => "Overlap test",
            Self::Dense => "Dense test",
        }
    }
}

/// Interface fixtures are never passed back to the authoritative session.
pub fn systems(scene: Scene) -> Vec<StarSystem> {
    let kinds = [
        StarKind::YellowDwarf,
        StarKind::RedDwarf,
        StarKind::BlueGiant,
        StarKind::RedGiant,
        StarKind::WhiteDwarf,
    ];
    (0..if scene == Scene::Overlap { 5 } else { 64 })
        .map(|i| StarSystem {
            id: SystemId(1000 + i),
            name: format!(
                "{} {:02}",
                if scene == Scene::Overlap {
                    "Depth"
                } else {
                    "Cluster"
                },
                i + 1
            ),
            star_kind: kinds[i as usize % kinds.len()],
            position: if scene == Scene::Overlap {
                GalaxyPosition {
                    x: 0,
                    y: 0,
                    z: i as i64 * 4 - 8,
                }
            } else {
                GalaxyPosition {
                    x: ((i * 17) % 29) as i64 - 14,
                    y: ((i * 11) % 23) as i64 - 11,
                    z: ((i * 7) % 31) as i64 - 15,
                }
            },
        })
        .collect()
}
