pub struct GameWorld {
    planets: Vec<Planet>,
    ships: Vec<Ship>,
}

pub struct PlayerState {
    player_name : String,
    ship: Vec<Ship>,
    inventory: Vec<Resource>,
    credits: u32,
}

impl GameWorld {
    pub fn new() -> GameWorld {
        /* ... */
    }

    pub fn update(&mut self, player_state: &PlayerState) {
        /* ... */
    }
}

impl PlayerState {
    pub fn new() -> PlayerState {
        /* ... */
    }
}

struct Planet {
    name: String,
    position: (f64, f64),
    resources: Vec<Resource>,
}

enum ResourceType {
    Food,
    Fuel,
    Metals,
}

struct Resource {
    resource_type: ResourceType,
    quantity: u32,
}

