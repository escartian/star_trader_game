export interface Ship {
    name: string;
    specialization: string;
    size: string;
    price?: number;
}

export interface Fleet {
    name: string;
    ships: Ship[];
}

export interface ShipMarketProps {
    systemId: number;
    planetId: number;
    isOpen: boolean;
    onClose: () => void;
} 