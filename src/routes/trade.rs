pub async fn buy_from_planet(
    Path((system_id, planet_id)): Path<(usize, usize)>,
    State(state): State<Arc<Mutex<GameState>>>,
    Json(payload): Json<MarketTransaction>,
) -> Result<Json<MarketResponse>, StatusCode> {
    let mut game_state = state.lock().await;
    let player = &mut game_state.player;
    
    // Load the market for this planet
    let mut market = Market::load(system_id, planet_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Calculate total cost
    let total_cost = market.buy_resource(payload.resource_type, payload.quantity, system_id, planet_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Check if player has enough credits
    if player.credits < total_cost as f32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Update player's credits and inventory
    player.credits -= total_cost as f32;
    player.add_resource(payload.resource_type, payload.quantity);
    
    // Save both player and market state
    player.save().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    market.save(system_id, planet_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MarketResponse {
        success: true,
        message: format!("Successfully bought {} units of {}", payload.quantity, payload.resource_type),
        new_credits: player.credits,
    }))
}

pub async fn sell_to_planet(
    Path((system_id, planet_id)): Path<(usize, usize)>,
    State(state): State<Arc<Mutex<GameState>>>,
    Json(payload): Json<MarketTransaction>,
) -> Result<Json<MarketResponse>, StatusCode> {
    let mut game_state = state.lock().await;
    let player = &mut game_state.player;
    
    // Load the market for this planet
    let mut market = Market::load(system_id, planet_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Check if player has enough resources
    if !player.has_resource(payload.resource_type, payload.quantity) {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Calculate total value and update market
    let total_value = market.sell_resource(payload.resource_type, payload.quantity, system_id, planet_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Update player's credits and inventory
    player.credits += total_value as f32;
    player.remove_resource(payload.resource_type, payload.quantity);
    
    // Save both player and market state
    player.save().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    market.save(system_id, planet_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MarketResponse {
        success: true,
        message: format!("Successfully sold {} units of {}", payload.quantity, payload.resource_type),
        new_credits: player.credits,
    }))
} 