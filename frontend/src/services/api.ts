import { Player, StarSystem, Fleet, Resource, ResourceType, Ship } from '../types/game';

// Get the current hostname (excluding port)
const hostname = window.location.hostname;
const API_BASE_URL = `http://${hostname}:8000/api`;

class ApiError extends Error {
    constructor(public status: number, message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

interface ApiResponse<T> {
    success: boolean;
    message: string;
    data: T | null;
}

async function handleApiResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    if (!data.success) {
        throw new Error(data.message);
    }
    if (data.data === null) {
        throw new Error('No data returned from server');
    }
    return data.data;
}

export const api = {
    // Player endpoints
    getPlayer: async (name: string): Promise<Player> => {
        const response = await fetch(`${API_BASE_URL}/player/${name}`);
        return handleApiResponse<Player>(response);
    },

    // Galaxy endpoints
    getGalaxyMap: async (): Promise<StarSystem[]> => {
        const response = await fetch(`${API_BASE_URL}/galaxy_map`);
        return handleApiResponse<StarSystem[]>(response);
    },

    getStarSystem: async (id: number): Promise<StarSystem> => {
        const response = await fetch(`${API_BASE_URL}/star_system/${id}`);
        return handleApiResponse<StarSystem>(response);
    },

    // Fleet endpoints
    getFleets: async (ownerId: string): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}`);
        return handleApiResponse<Fleet[]>(response);
    },

    getFleet: async (ownerId: string, fleetNumber: number): Promise<Fleet | null> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}`);
        return handleApiResponse<Fleet | null>(response);
    },

    moveFleet: async (ownerId: string, fleetNumber: number, x: number, y: number, z: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/move/${x}/${y}/${z}`);
        return handleApiResponse<string>(response);
    },

    // Market endpoints
    getPlanetMarket: async (systemId: number, planetId: number): Promise<Resource[]> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/market`);
        return handleApiResponse<Resource[]>(response);
    },

    buyResource: async (systemId: number, planetId: number, resourceType: ResourceType, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/buy`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                resource_type: resourceType,
                quantity: quantity
            }),
        });
        
        return handleApiResponse<string>(response);
    },

    sellResource: async (systemId: number, planetId: number, resourceType: ResourceType, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/sell`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                resource_type: resourceType,
                quantity: quantity
            }),
        });
        
        return handleApiResponse<string>(response);
    },

    // Fleet owner endpoints
    //Returns a list of all fleet owners
    getFleetOwners: async (): Promise<string[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/owners`);
        return handleApiResponse<string[]>(response);
    },

    //Returns a list of all fleets owned by a specific owner
    getOwnerFleets: async (ownerId: string): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}`);
        return handleApiResponse<Fleet[]>(response);
    },

    // Combat endpoints
    initiateCombat: async (attackerId: string, attackerNumber: number, defenderId: string, defenderNumber: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${attackerId}/${attackerNumber}/attack/${defenderId}/${defenderNumber}`);
        return handleApiResponse<string>(response);
    },

    // Encounter endpoints
    checkForEncounter: async (ownerId: string, fleetNumber: number): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/encounter`);
        return handleApiResponse<Fleet[]>(response);
    },

    // Trade endpoints
    tradeWithTrader: async (fleetId: string, fleetNumber: number, resourceType: string, quantity: number, tradeType: 'buy' | 'sell'): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${fleetId}/${fleetNumber}/trade/${resourceType}/${quantity}/${tradeType}`);
        return handleApiResponse<string>(response);
    },

    getPlanetShipMarket: async (systemId: number, planetId: number): Promise<Ship[]> => {
        const response = await fetch(`${API_BASE_URL}/systems/${systemId}/planets/${planetId}/ship-market`);
        if (!response.ok) {
            throw new Error('Failed to fetch ship market data');
        }
        return handleApiResponse<Ship[]>(response);
    },

    buyShip: async (systemId: number, planetId: number, shipName: string, fleetName: string, tradeInShipName?: string): Promise<string> => {
        console.log('Making buy ship request:', {
            systemId,
            planetId,
            shipName,
            fleetName,
            tradeInShipName
        });
        
        const response = await fetch(`${API_BASE_URL}/systems/${systemId}/planets/${planetId}/ship-market/buy`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ 
                ship_name: shipName,
                fleet_name: fleetName,
                trade_in_ship: tradeInShipName
            }),
        });
        
        return handleApiResponse<string>(response);
    },

    sellShip: async (systemId: number, planetId: number, shipName: string, fleetName: string): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/systems/${systemId}/planets/${planetId}/ship-market/sell`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ 
                ship_name: shipName,
                fleet_name: fleetName
            }),
        });
        
        return handleApiResponse<string>(response);
    },

    // Resource trading
    buyFromPlanet: async (systemId: number, planetId: number, resourceType: ResourceType, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/buy`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                resource_type: resourceType,
                quantity: quantity
            })
        });

        if (!response.ok) {
            throw new Error('Failed to buy resource');
        }

        const data = await response.json();
        return data.message;
    },

    sellToPlanet: async (systemId: number, planetId: number, resourceType: ResourceType, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/sell`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                resource_type: resourceType,
                quantity: quantity
            })
        });

        if (!response.ok) {
            throw new Error('Failed to sell resource');
        }

        const data = await response.json();
        return data.message;
    },
};