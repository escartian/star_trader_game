# Star Trader Game

A space trading and combat simulation game implemented in Rust, featuring a dynamic economy, fleet management, and interstellar travel.

## Table of Contents
- [Project Overview](#project-overview)
- [Core Game Mechanics](#core-game-mechanics)
- [Key Data Structures](#key-data-structures)
- [Game State Management](#game-state-management)
- [File Structure](#file-structure)
- [Key Requirements](#key-requirements)
- [API Endpoints](#api-endpoints)
- [Technical Notes](#technical-notes)
- [Future Development](#future-development-considerations)
- [Development Environment](#development-environment)

## Project Overview

Star Trader is a space-based game where players can:
- Manage fleets of spaceships
- Trade resources between star systems
- Engage in combat with other fleets
- Navigate through a procedurally generated galaxy
- Interact with various factions and NPCs

## Core Game Mechanics

### Galaxy Structure
- The game world consists of multiple star systems arranged in a 3D space
- Each star system has:
  - A unique position in galactic coordinates
  - Multiple planets with their own markets
  - Local coordinate system for intra-system movement
  - Boundaries for system entry/exit

### Movement System
- The game uses a cubic coordinate system (no spherical calculations needed)
- Coordinate System:
  - Galaxy coordinates: Based on map dimensions (width, height, length)
  - System coordinates: Local coordinate system centered on the star system
  - All positions use integer coordinates (x, y, z)

#### Movement Types
1. **Local Movement**
   - Within star system bounds (configurable in settings.json)
   - Default system size: ±100 units from system center
   - Direct movement to target position
   - No special calculations needed
   - Fleet remains in system space

2. **System Exit**
   - Triggered when target position is outside system bounds
   - Calculates exit point based on movement direction
   - Fleet is placed just outside system boundary
   - Fleet transitions to deep space state
   - Exit point calculation:
     ```rust
     exit_x = system.position.x + dir_x * (system_max + 1)
     exit_y = system.position.y + dir_y * (system_max + 1)
     exit_z = system.position.z + dir_z * (system_max + 1)
     ```

3. **Deep Space Movement**
   - Between star systems
   - Linear movement along calculated path
   - Checks for system entry during movement
   - Game boundary calculations needed

4. **System Entry**
   - Detected when fleet path intersects system boundary
   - Entry point calculated based on approach vector
   - Fleet transitions to system space state
   - Entry point calculation:
     ```rust
     entry_x = system.position.x - norm_dx
     entry_y = system.position.y - norm_dy
     entry_z = system.position.z - norm_dz
     ```

### Economy
- Dynamic market system per planet
- Resource trading with price fluctuations
- Ship buying, selling, and trading
- Multiple resource types and ship configurations

### Planet System
- Each planet has:
  - Unique economy type
  - Specialization (affects resource production)
  - Danger level (from VerySafe to Insidious)
  - Biome type
  - Local market
  - Ship market
  - Credits for trading

### Combat System
Advanced weapons including:
- Photon Singularity Beam
- Quantum Entanglement Torpedo
- Neutron Beam
- Graviton Pulse
- Magnetic Resonance Disruptor

### Faction System
- Multiple factions with influence levels
- Default factions:
  - Federation
  - Empire
  - Republic
  - Alliance
- Faction influence affects:
  - Fleet generation
  - Territory control
  - Trade opportunities

## Key Data Structures

### GameSettings
```rust
struct GameSettings {
    game_id: String,
    player_name: String,
    display_name: String,
    map_width: u32,
    map_height: u32,
    map_length: u32,
    star_count: u32,
    starting_credits: f32,
    created_at: String,
    last_played: String,
    factions: Vec<FactionSettings>
}
```

### Position
```rust
struct Position {
    x: i32,
    y: i32,
    z: i32
}
```

### Fleet
```rust
struct Fleet {
    name: String,
    owner_id: String,
    ships: Vec<Ship>,
    position: Position,
    current_system_id: Option<usize>,
    last_move_distance: Option<f64>
}
```

### Ship
```rust
struct Ship {
    name: String,
    ship_type: ShipType,
    size: ShipSize,
    engine: ShipEngine,
    position: Position,
    hp: i32,
    price: Option<f32>
}
```

### Market
```rust
struct Market {
    resources: HashMap<ResourceType, ResourceMarket>,
    last_updated: String
}
```

### StarSystem
```rust
struct StarSystem {
    name: String,
    position: Position,
    planets: Vec<Planet>,
    faction_influence: HashMap<String, f32>
}
```

## Game State Management

The game maintains several important state components:
- Global game world state (star systems and their contents)
- Player state (credits, owned fleets)
- Market states (per-planet resource and ship markets)
- Fleet states (position, composition, status)

### Caching System
The game implements a sophisticated caching system for performance:
- Player Cache (30 seconds TTL)
- System Cache (60 seconds TTL)
- Fleet Cache (30 seconds TTL)
- Market Cache (30 seconds TTL)

## File Structure

The game data is organized in the following directory structure:
```
data/
  ├── game/
  │   └── [game_id]/
  │       ├── settings.json
  │       ├── fleets/
  │       ├── markets/
  │       ├── players/
  │       ├── factions/
  │       └── star_systems/
  └── saves/
      └── [game_id].json
```

## Key Requirements

1. **Movement Mechanics**
   - Proper handling of system boundaries
   - Smooth transitions between deep space and system space
   - Collision detection with celestial bodies

2. **Economic System**
   - Dynamic pricing based on supply and demand
   - Market updates at regular intervals
   - Trade route optimization

3. **Combat System**
   - Fleet-to-fleet combat resolution
   - Damage calculation and ship destruction
   - Combat rewards and consequences

4. **Data Persistence**
   - Saving and loading game states
   - Market state persistence
   - Fleet position and composition tracking

5. **Performance Considerations**
   - Efficient position calculations
   - Optimized market updates
   - Cached data for frequently accessed information

## API Endpoints

The game exposes several REST endpoints for:
- Fleet management and movement
- Trade operations
- Combat initiation
- Game state management
- Market interactions

## Technical Notes

- Written in Rust for performance and safety
- Uses Rocket framework for web API
- Implements concurrent access patterns using Mutex
- Employs JSON for data serialization
- Uses a component-based architecture for game entities
- Frontend built with React and TypeScript

## Future Development Considerations

Planned features:
- Multiplayer support
- Enhanced faction AI
- More complex economic systems
- Additional ship types and customization
- Quest and mission system
- Advanced combat mechanics
- Planet colonization
- Planet development mechanics
- Planet events system
- Planet diplomacy system
- Modified random generation for starting systems
- Alien races with reputation systems
- Orbital mechanics for planets
- System-wide events

## Development Environment

### Prerequisites
- Rust (latest stable version)
- Node.js (v14 or higher)
- npm (latest version)

### Setup Instructions
1. Clone the repository
2. Install backend dependencies and build:
   ```bash
   cargo build
   ```
3. Install frontend dependencies:
   ```bash
   cd frontend
   npm install
   ```
4. Start the development servers:
   - Backend: `cargo run`
   - Frontend - go to frontend directory:  `npm start`

Runs the app in the development mode.
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.

The page will reload if you make edits.\

### Development Workflow
- Backend code is in the `src` directory
- Frontend code is in the `frontend` directory
- Game data is stored in the `data` directory
- Use `cargo test` to run backend tests
- Use `npm test` to run frontend tests 