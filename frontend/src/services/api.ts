import { Player, StarSystem, Fleet, Resource, Ship } from '../types/game';

// Get the current hostname (excluding port)
const hostname = window.location.hostname;
const API_BASE_URL = `http://${hostname}:8000/api`;

class ApiError extends Error {
    constructor(public status: number, message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

async function handleResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
        throw new ApiError(response.status, `HTTP error! status: ${response.status}`);
    }
    const data = await response.text();
    try {
        return JSON.parse(data);
    } catch (e) {
        throw new ApiError(response.status, 'Failed to parse JSON response');
    }
}

export const api = {
    // Player endpoints
    getPlayer: async (name: string): Promise<Player> => {
        const response = await fetch(`${API_BASE_URL}/player/${name}`);
        return handleResponse<Player>(response);
    },

    // Galaxy endpoints
    getGalaxyMap: async (): Promise<StarSystem[]> => {
        const response = await fetch(`${API_BASE_URL}/galaxy_map`);
        return handleResponse<StarSystem[]>(response);
    },

    getStarSystem: async (id: number): Promise<StarSystem> => {
        const response = await fetch(`${API_BASE_URL}/star_system/${id}`);
        return handleResponse<StarSystem>(response);
    },

    // Fleet endpoints
    getFleets: async (ownerId: string): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}`);
        return handleResponse<Fleet[]>(response);
    },

    getFleet: async (ownerId: string, fleetNumber: number): Promise<Fleet | null> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}`);
        return handleResponse<Fleet | null>(response);
    },

    moveFleet: async (ownerId: string, fleetNumber: number, x: number, y: number, z: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/move/${x}/${y}/${z}`);
        return handleResponse<string>(response);
    },

    // Market endpoints
    getPlanetMarket: async (systemId: number, planetId: number): Promise<Resource[]> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/market`);
        return handleResponse<Resource[]>(response);
    },

    buyFromPlanet: async (systemId: number, planetId: number, resourceType: string, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/buy/${resourceType}/${quantity}`);
        return handleResponse<string>(response);
    },

    sellToPlanet: async (systemId: number, planetId: number, resourceType: string, quantity: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/sell/${resourceType}/${quantity}`);
        return handleResponse<string>(response);
    },

    // Fleet owner endpoints
    //Returns a list of all fleet owners
    getFleetOwners: async (): Promise<string[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/owners`);
        return handleResponse<string[]>(response);
    },

    //Returns a list of all fleets owned by a specific owner
    getOwnerFleets: async (ownerId: string): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}`);
        return handleResponse<Fleet[]>(response);
    },

    // Combat endpoints
    initiateCombat: async (attackerId: string, attackerNumber: number, defenderId: string, defenderNumber: number): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${attackerId}/${attackerNumber}/attack/${defenderId}/${defenderNumber}`);
        return handleResponse<string>(response);
    },

    // Encounter endpoints
    checkForEncounter: async (ownerId: string, fleetNumber: number): Promise<Fleet[]> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/encounter`);
        return handleResponse<Fleet[]>(response);
    },

    // Trade endpoints
    tradeWithTrader: async (fleetId: string, fleetNumber: number, resourceType: string, quantity: number, tradeType: 'buy' | 'sell'): Promise<string> => {
        const response = await fetch(`${API_BASE_URL}/fleet/${fleetId}/${fleetNumber}/trade/${resourceType}/${quantity}/${tradeType}`);
        return handleResponse<string>(response);
    },

    getPlanetShipMarket: async (systemId: number, planetId: number): Promise<Ship[]> => {
        const response = await fetch(`${API_BASE_URL}/systems/${systemId}/planets/${planetId}/ship-market`);
        if (!response.ok) {
            throw new Error('Failed to fetch ship market data');
        }
        return response.json();
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
        
        console.log('Buy ship response status:', response.status);
        const responseText = await response.text();
        console.log('Buy ship response:', responseText);
        
        if (!response.ok) {
            throw new Error('Failed to buy ship');
        }
        return responseText;
    },

    sellShip: async (systemId: number, planetId: number, shipName: string, fleetName: string): Promise<string> => {
        console.log('Making sell ship request:', {
            systemId,
            planetId,
            shipName,
            fleetName
        });
        
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
        
        console.log('Sell ship response status:', response.status);
        const responseText = await response.text();
        console.log('Sell ship response:', responseText);
        
        if (!response.ok) {
            throw new Error('Failed to sell ship');
        }
        return responseText;
    },
};