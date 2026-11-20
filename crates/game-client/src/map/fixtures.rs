#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Session,
    /// Developer layout fixtures, absent from release builds.
    #[cfg(any(test, feature = "dev-tools"))]
    Overlap,
    #[cfg(any(test, feature = "dev-tools"))]
    Dense,
}

#[cfg(any(test, feature = "dev-tools"))]
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
#[cfg(any(test, feature = "dev-tools"))]
pub fn systems(scene: Scene) -> Vec<game_core::StarSystem> {
    use game_core::{GalaxyPosition, StarKind, StarSystem, SystemId};
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
            bodies: Vec::new(),
            name: format!(
                "{} {:02}",
                if scene == Scene::Overlap {
                    "Overlap"
                } else {
                    "Cluster"
                },
                i + 1
            ),
            star_kind: kinds[i as usize % kinds.len()],
            position: if scene == Scene::Overlap {
                GalaxyPosition { x: 0, y: 0, z: 0 }
            } else {
                GalaxyPosition {
                    x: ((i * 17) % 29) as i64 - 14,
                    y: 0,
                    z: ((i * 7) % 31) as i64 - 15,
                }
            },
        })
        .collect()
}
