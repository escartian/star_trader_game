import { ReactNode } from "react";

export interface Position {
    x: number;
    y: number;
    z: number;
}

export interface Resource {
    resource_type: ResourceType;
    quantity: number | null;
    buy: number | null;
    sell: number | null;
}

export interface Shield {
    capacity: number;
    current: number;
    regen: number;
}

export interface Armor {
    capacity: number;
    current: number;
    regen: number;
}

export interface Weapon {
    PhotonSingularityBeam?: { damage: number };
    QuantumEntanglementTorpedo?: { damage: number };
    NeutronBeam?: { damage: number };
    GravitonPulse?: { damage: number };
    MagneticResonanceDisruptor?: { damage: number };
}

export interface Ship {
    name: string;
    owner: string;
    position: Position;
    status: 'OnPlanetRough' | 'Docked' | 'Launching' | 'Landing' | 'OrbitingPlanet' | 'SubLightTravel' | 'Warp' | 'Stationary';
    hp: number;
    combat_state: 'NotInCombat' | 'Aggressive' | 'Default' | 'Evasive' | 'Passive';
    specialization: 'Fighter' | 'Battleship' | 'Freighter' | 'Explorer' | 'Shuttle' | 'Capital';
    size: 'Tiny' | 'Small' | 'Medium' | 'Large' | 'Huge' | 'Planetary';
    engine: 'Basic' | 'Advanced' | 'Experimental';
    weapons: Weapon[];
    cargo: Resource[];
    shields: Shield;
    armor: Armor;
    price?: number;
}

export interface Fleet {
    name: string;
    owner_id: string;
    ships: Ship[];
    position: Position;
    current_system_id: number | null;
    last_move_distance: number | null;
}

export interface Planet {
    name: string;
    position: Position;
    economy: string;
    specialization: string;
    danger: string;
    biome: string;
    market: Resource[];
}

export interface Star {
    name: string;
    star_type: string;
    position: Position;
}

export interface StarSystem {
    id: number;
    position: Position;
    star: Star;
    planets: Planet[];
}

export interface Player {
    name: string;
    credits: number;
    resources: Resource[];
    fleets: Fleet[];
}

export enum ResourceType {
    Minerals = 'Minerals',
    Food = 'Food',
    Technology = 'Technology',
    Luxury = 'Luxury',
    RawMaterials = 'RawMaterials',
    Energy = 'Energy',
    Water = 'Water',
    Medicine = 'Medicine',
    Weapons = 'Weapons',
    Electronics = 'Electronics'
}

export interface Faction {
    name: string;
    influence: number;
}

export interface GameSettings {
    game_id: string;
    display_name: string;
    player_name: string;
    map_width: number;
    map_height: number;
    map_length: number;
    star_count: number;
    starting_credits: number;
    print_debug: boolean;
    max_combat_time: number;
    factions: Faction[];
    created_at: string;
    last_played: string;
}

export interface SavedGame {
    game_id: string;
    created_at: string;
    last_played: string;
    settings: GameSettings;
}

export interface Market {
    resources: Resource[];
}

export interface ShipMarket {
    ships: Ship[];
}