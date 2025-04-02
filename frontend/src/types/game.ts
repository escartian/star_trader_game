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
    star: Star;
    position: Position;
    planets: Planet[];
}

export interface Player {
    name: string;
    credits: number;
    resources: Resource[];
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