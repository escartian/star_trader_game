//! The picture at the top of an event card: a procedural scene of deep space
//! (nebula light, fixed stars, a planet's limb) with the ship models involved,
//! yours on the left facing right, the others mirrored on the right. Painted
//! from fixed hashes, so a card looks the same every frame. Presentation only.
use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use game_core::fleet::ShipKind;

/// The scene's lighting, which tells the player the kind of event at a glance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    /// Raiders, corsairs, a fight.
    Hostile,
    /// Traders, offers, ordinary space.
    Neutral,
    /// A call for help, a hazard.
    Distress,
    /// Police and customs.
    Lawful,
    /// A road to victory completed, or a campaign completed.
    Triumph,
    /// The last ship lost.
    Loss,
}

impl Mood {
    /// (nebula, second nebula, accent) colours.
    fn palette(self) -> (Color32, Color32, Color32) {
        let rgba = Color32::from_rgba_unmultiplied;
        match self {
            Self::Hostile => (
                rgba(150, 30, 30, 70),
                rgba(90, 20, 60, 50),
                super::iff::HOSTILE,
            ),
            Self::Neutral => (rgba(30, 70, 120, 60), rgba(40, 100, 110, 45), super::CYAN),
            Self::Distress => (rgba(150, 90, 20, 60), rgba(110, 40, 20, 45), super::AMBER),
            Self::Lawful => (
                rgba(30, 60, 150, 70),
                rgba(70, 70, 140, 45),
                super::iff::NEUTRAL,
            ),
            Self::Triumph => (rgba(60, 130, 40, 60), rgba(150, 130, 40, 50), super::GREEN),
            Self::Loss => (
                rgba(110, 20, 20, 60),
                rgba(40, 40, 50, 50),
                super::iff::HOSTILE,
            ),
        }
    }

    /// The card's accent: title colour and frame line.
    pub fn accent(self) -> Color32 {
        self.palette().2
    }
}

/// What the scene shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Illustration {
    pub mood: Mood,
    /// Your ships, lead first (left, facing right).
    pub ours: Vec<ShipKind>,
    /// The other side's ships, lead first (right, mirrored).
    pub theirs: Vec<ShipKind>,
    /// Draw the trade station behind the scene (ports, victories).
    pub station: bool,
    /// Varies the stars and nebulae between events.
    pub seed: u64,
}

impl Illustration {
    pub fn new(mood: Mood, seed: u64) -> Self {
        Self {
            mood,
            ours: Vec::new(),
            theirs: Vec::new(),
            station: false,
            seed,
        }
    }
}

/// Paint the scene into `rect` (the caller clips and frames it).
pub fn paint(painter: &Painter, rect: Rect, art: &Illustration) {
    let (nebula, second, accent) = art.mood.palette();
    painter.rect_filled(rect, 0.0, Color32::from_rgb(3, 4, 10));
    let at = |x: f32, y: f32| rect.min + rect.size() * Vec2::new(x, y);
    let jitter = |salt: u32| unit(art.seed, salt) * 0.3 - 0.15;
    glow(painter, at(0.3 + jitter(1), 0.4), rect.size() * 0.7, nebula);
    glow(
        painter,
        at(0.75 + jitter(2), 0.6),
        rect.size() * 0.55,
        second,
    );
    for index in 0..90_u32 {
        let point = at(
            unit(art.seed, index * 3 + 10),
            unit(art.seed, index * 3 + 11),
        );
        let glint = unit(art.seed, index * 3 + 12);
        let level = 70 + (150.0 * glint * glint) as u8;
        painter.circle_filled(point, 0.6 + glint, Color32::from_gray(level));
    }
    // A planet's limb along the bottom, lit along its rim in the accent colour.
    let planet = Pos2::new(
        at(0.5 + jitter(3), 0.0).x,
        rect.bottom() + rect.width() * 0.9,
    );
    let radius = rect.width() * 1.02;
    painter.circle_filled(planet, radius, Color32::from_rgb(8, 11, 16));
    painter.circle_stroke(
        planet,
        radius,
        Stroke::new(2.0_f32, accent.gamma_multiply(0.55)),
    );
    painter.circle_stroke(
        planet,
        radius - 3.0,
        Stroke::new(4.0_f32, accent.gamma_multiply(0.12)),
    );
    let height = rect.height();
    if art.station
        && let Some(station) = crate::battle::model::station()
    {
        let tint = Color32::from_gray(150);
        station.paint_portrait(painter, at(0.5, 0.38), height * 0.75, false, tint);
    }
    let ours_tint = if art.mood == Mood::Loss {
        Color32::from_rgb(120, 70, 60)
    } else {
        Color32::WHITE
    };
    fleet(painter, rect, &art.ours, false, ours_tint);
    fleet(painter, rect, &art.theirs, true, Color32::WHITE);
    // A soft vignette keeps the frame edge dark.
    painter.rect_stroke(
        rect.shrink(6.0),
        0.0,
        Stroke::new(12.0_f32, Color32::from_black_alpha(90)),
        egui::StrokeKind::Outside,
    );
}

/// One side's ships: the lead large and forward, the others smaller behind it.
fn fleet(painter: &Painter, rect: Rect, ships: &[ShipKind], right: bool, tint: Color32) {
    let height = rect.height();
    // (x, y) as fractions of the rect for the left side; mirrored for the right.
    const SLOTS: [(f32, f32, f32); 4] = [
        (0.27, 0.52, 0.62),
        (0.12, 0.32, 0.38),
        (0.13, 0.74, 0.36),
        (0.04, 0.52, 0.30),
    ];
    // Back ships first so the lead draws over them.
    for (index, kind) in ships.iter().enumerate().take(SLOTS.len()).rev() {
        let Some(model) = crate::battle::model::for_kind(*kind) else {
            continue;
        };
        let (x, y, size) = SLOTS[index];
        let x = if right { 1.0 - x } else { x };
        let at = rect.min + rect.size() * Vec2::new(x, y);
        let dim = if index == 0 { 1.0 } else { 0.8 };
        model.paint_portrait(painter, at, height * size, right, tint.gamma_multiply(dim));
    }
    if ships.len() > SLOTS.len() {
        let x = if right {
            rect.right() - 30.0
        } else {
            rect.left() + 30.0
        };
        painter.text(
            Pos2::new(x, rect.bottom() - 16.0),
            egui::Align2::CENTER_CENTER,
            format!("+{}", ships.len() - SLOTS.len()),
            egui::FontId::proportional(13.0),
            Color32::from_gray(200),
        );
    }
}

/// A transparent radial fan: soft light without textures.
fn glow(painter: &Painter, center: Pos2, radius: Vec2, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for index in 0..=40_u32 {
        let angle = index as f32 * std::f32::consts::TAU / 40.0;
        mesh.colored_vertex(
            center + Vec2::new(angle.cos(), angle.sin()) * radius,
            Color32::TRANSPARENT,
        );
        if index > 0 {
            mesh.add_triangle(0, index, index + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// A well-mixed hash of (seed, salt) in [0, 1).
fn unit(seed: u64, salt: u32) -> f32 {
    let mut x = seed ^ (u64::from(salt).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}
