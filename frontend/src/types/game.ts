export interface Position {
    x: number;
    y: number;
    z: number;
}

export interface Resource {
    resource_type: string;
    buy: number | null;
    sell: number | null;
    quantity: number | null;
}

export interface Ship {
    name: string;
    ship_type: string;
    size: string;
    health: number;
    shields: number;
    cargo: Resource[];
}

export interface Fleet {
    name: string;
    owner_id: string;
    position: Position;
    ships: Ship[];
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

export interface StarSystem {
    position: Position;
    planets: Planet[];
}

export interface Player {
    name: string;
    credits: number;
    resources: Resource[];
}