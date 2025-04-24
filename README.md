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
- [API Response Structure](#api-response-structure)
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
- The game world consists of multiple star systems arranged in a 3D cubic space
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
  - Default galaxy size is ±100 units

#### Movement Types
1. **Local Movement**
   - Within star system bounds (configurable in settings.json)
   - Default system size: ±100 units forming a cube
   - Direct movement to target position
   - No special calculations needed
   - Fleet remains in system space

2. **System Exit**
   - Triggered when target position is initially inside System bounds
   - Target moves outside system bounds
   - Calculates exit point based on movement direction
   - Fleet is placed just outside system boundary
        (system coordinates +1 in directions moved)
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
   - Checks for system at end of movement
   - Game boundary calculations needed

4. **System Entry**
   - Only occurs when fleet is explicitly moved to a system's coordinates
   - No automatic detection or interception of fleet paths
   - Fleet transitions to system space state
   - Entry point calculation:
     ```rust
     entry_x = system.position.x - norm_dx
     entry_y = system.position.y - norm_dy
     entry_z = system.position.z - norm_dz
     ```

### Economy
- Unique market system per planet
- Resource trading (TODO: price fluctuations)
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

## API Response Structure

All API responses follow a consistent JSON structure:

```json
{
    "success": true|false,
    "message": "Human readable message",
    "data": {
        // Response data specific to the endpoint
    }
}
```

### Common Response Types

1. **Success Response**
```json
{
    "success": true,
    "message": "Operation completed successfully",
    "data": {
        // Operation-specific data
    }
}
```

2. **Error Response**
```json
{
    "success": false,
    "message": "Detailed error message",
    "data": null
}
```

### Specific (Symplified) Response Examples

1. **Fleet Movement Response**
```json
{
    "success": true,
    "message": "Fleet moved successfully within system",
    "data": {
        "status": "success|transition_exit|transition_entry",
        "message": "Detailed movement status",
        "encounters": [],
        "current_position": {
            "x": 0,
            "y": 0,
            "z": 0
        },
        "target_position": {
            "x": 0,
            "y": 0,
            "z": 0
        },
        "remaining_distance": 0.0,
        "current_system_id": 0
    }
}
```

2. **Trade Response**
```json
{
    "success": true,
    "message": "Trade completed successfully",
    "data": {
        "credits_spent": 100.0,
        "resources_received": {
            "type": "Fuel",
            "quantity": 10
        }
    }
}
```

3. **Combat Response**
```json
{
    "success": true,
    "message": "Combat resolved successfully",
    "data": {
        "attacker_losses": 2,
        "defender_losses": 1,
        "combat_log": [
            "Ship A fired at Ship B",
            "Ship B's shields took 50 damage"
        ]
    }
}
```

4. **Resource Market Response**
```json
{
    "success": true,
    "message": "Resource market data retrieved successfully",
    "data": {
        "resources": {
            "Fuel": {
                "price": 10.0,
                "quantity": 100
            },
            "Metals": {
                "price": 15.0,
                "quantity": 50
            },
            "Electronics": {
                "price": 25.0,
                "quantity": 30
            }
        },
        "last_updated": "2024-03-20T12:00:00Z"
    }
}
```

5. **Ship Market Response**
```json
{
    "success": true,
    "message": "Ship market data retrieved successfully",
    "data": {
        "ships": [
            {
                "name": "Gamma 285-type",
                "type": "Capital",
                "size": "Tiny",
                "engine": "Experimental",
                "price": 1000.0,
                "weapons": [
                    {
                        "type": "PhotonSingularityBeam",
                        "damage": 10
                    }
                ],
                "shields": {
                    "current": 225,
                    "max": 225
                },
                "armor": {
                    "current": 300,
                    "max": 300
                }
            }
        ],
        "last_updated": "2024-03-20T12:00:00Z"
    }
}
```

### Error Handling

The API uses HTTP status codes and detailed error messages:

- 200: Success
- 400: Bad Request (invalid input)
- 404: Not Found (resource doesn't exist)
- 500: Internal Server Error

Error responses include:
- Descriptive error message
- Error type/category
- Additional context when available

## Technical Notes

- Written in Rust for performance and safety
- Uses Rocket framework for web API
- Implements concurrent access patterns using Mutex
- Employs JSON for data serialization
- Uses a component-based architecture for game entities
- Frontend built with React and TypeScript

## Feature List

### Currently Working On
- Traders + personality
- Player data
- Fleet Management System
  - Fleet viewing and management in React frontend
  - Fleet movement between star systems
  - Fleet transition from star systems to deep space and from deep space to star system
  - Fleet cargo and ship details

### Future Features (Prioritized)
1. Combat System
   - Implement the combat system that's already defined in the backend
   - Add fleet combat UI
   - Show combat results and ship damage

2. Faction System
   - Add faction relationships and reputation
   - Show faction territories and influence
   - Implement faction-specific missions or trading opportunities

3. Ship Management
   - Add ship customization and upgrades
   - Implement ship repair and maintenance
   - Show detailed ship statistics and capabilities

4. Mission System
   - Add trading missions (Transport goods)
   - Implement combat missions
   - Add exploration missions (eg: anomaly investigation)
   - Bountyhunter missions
   - Pirating goods/people missions

### Potential Features
- Multipayer support
- Enchanced faction AI
- More complex economic systems
- Modified random generation for starting systems
- Modified random generation for planet/star types
- Alien races with reputation systems
- Planet colonization
- Planet development mechanics
- Planet events system
- Planet diplomacy system

### Highly Unlikely Features
- Double star systems
- Multiple galaxies
- Galactic orbits/movements
- Orbital mechanics (due to cubic coordinate system)

### Currently Implemented MVP Features
- World map generation
- Persistent game state
- Position system
- Star system generation
- Galaxy generation
- Basic resources
- Trade system (buying and selling with planets)
- Game state persistence

### Feature Complete Elements
- Ship model
- Planet model
- Star model
- Galaxy model
- Position structure

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

The page will reload if you make edits.

### Development Workflow
- Backend code is in the `src` directory
- Frontend code is in the `frontend` directory
- Game data is stored in the `data` directory
- Use `cargo test` to run backend tests
- Use `npm test` to run frontend tests 