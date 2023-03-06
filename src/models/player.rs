use std::collections::HashMap;

use super::{resource::{Resource, ResourceType}, trader::Trader};

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub resources: HashMap<ResourceType, u32>,
    pub credits: f32,
}

impl Player {
    pub fn new(player_name: &str) -> Self {
        Player {
            name: player_name.to_string(),
            resources: HashMap::new(),
            credits: 100.0, // Set initial credits to 100
        }
    }
}