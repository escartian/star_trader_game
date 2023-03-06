use super::{resource::{Resource, generate_resources}};

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub resources: Vec<Resource>,
    pub credits: f32,
}

impl Player {
    pub fn new(player_name: &str) -> Self {
        Player {
            name: player_name.to_string(),
            resources: generate_resources(),
            credits: 1000.0, // Set initial credits to 100
        }
    }
}