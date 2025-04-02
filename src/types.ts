export interface Ship {
    name: string;
    owner: string;
    position: {
        x: number;
        y: number;
        z: number;
    };
    status: 'Stationary' | 'Moving' | 'Docked' | 'InCombat';
    hp: number;
    combat_state: 'Default' | 'Aggressive' | 'Passive' | 'Fleeing';
    specialization: 'Fighter' | 'Battleship' | 'Freighter' | 'Explorer' | 'Shuttle' | 'Capital';
    size: 'Tiny' | 'Small' | 'Medium' | 'Large' | 'Huge' | 'Planetary';
    engine: 'Basic' | 'Advanced' | 'Experimental';
    weapons: Array<{
        type: string;
        damage: number;
    }>;
    cargo: Array<{
        type: string;
        amount: number;
    }>;
    shields: {
        max: number;
        current: number;
    };
    armor: {
        max: number;
        current: number;
    };
    price?: number;
}

export interface Fleet {
    name: string;
    owner: string;
    position: {
        x: number;
        y: number;
        z: number;
    };
    ships: Ship[];
}

export interface Planet {
    name: string;
    economy: string;
    population: number;
    credits: number;
    ship_market?: Ship[];
} 