import { StarSystem, Player, GameSettings, SavedGame, Fleet, Resource, ResourceType, Market, ShipMarket } from '../types/game';
import { Ship } from '../types';
import type { ApiResponse } from '../types/api.js';

// Get the current hostname (excluding port)
const hostname = window.location.hostname;
const API_BASE_URL = `http://${hostname}:8000/api`;

class ApiError extends Error {
    constructor(public status: number, message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

async function handleApiResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data: ApiResponse<T> = await response.json();
    if (!data.success) {
        throw new Error(data.message);
    }
    if (!data.data) {
        throw new Error('No data returned from server');
    }
    return data.data;
}

export const api = {
    // Player endpoints
    getPlayer: async (name: string): Promise<Player> => {
        const response = await fetch(`${API_BASE_URL}/player/${name}`);
        if (!response.ok) {
            throw new Error(`Failed to load player: ${response.status}`);
        }
        const data = await response.json();
        if (!data.success || !data.data) {
            throw new Error(data.message || 'Failed to load player data');
        }
        return data.data;
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
        console.log(`Moving fleet ${ownerId}_${fleetNumber} to (${x}, ${y}, ${z}) in deep space`);
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/move`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ x, y, z }),
        });

        const responseText = await response.text();
        if (!response.ok) {
            console.error('Move fleet failed:', responseText);
            throw new Error(`Failed to move fleet: ${response.statusText}`);
        }

        return responseText;
    },

    moveLocal: async (ownerId: string, fleetNumber: number, x: number, y: number, z: number): Promise<string> => {
        console.log(`Moving fleet ${ownerId}_${fleetNumber} to (${x}, ${y}, ${z}) within system`);
        const response = await fetch(`${API_BASE_URL}/fleet/${ownerId}/${fleetNumber}/move_local`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ x, y, z }),
        });

        const responseText = await response.text();
        if (!response.ok) {
            console.error('Move local failed:', responseText);
            throw new Error(`Failed to move fleet: ${response.statusText}`);
        }

        return responseText;
    },

    // Market endpoints
    getPlanetMarket: async (systemId: number, planetId: number): Promise<Market> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/market`);
        if (!response.ok) {
            throw new Error(`Failed to load market: ${response.status}`);
        }
        const data = await response.json();
        if (!data.success || !data.data) {
            throw new Error(data.message || 'Failed to load market data');
        }
        return data.data;
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

    getPlanetShipMarket: async (systemId: number, planetId: number): Promise<ApiResponse<ShipMarket>> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/ships`);
        if (!response.ok) {
            throw new Error(`Failed to load ship market: ${response.status}`);
        }
        const data = await response.json();
        if (!data.success || !data.data) {
            throw new Error(data.message || 'Failed to load ship market data');
        }
        return data;
    },

    buyShip: async (systemId: number, planetId: number, shipIndex: number, fleetName?: string): Promise<ApiResponse<string>> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/buy_ship`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ ship_index: shipIndex, fleet_name: fleetName }),
        });
        return response.json();
    },

    sellShip: async (systemId: number, planetId: number, shipIndex: number, fleetName: string): Promise<ApiResponse<string>> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/sell_ship`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ 
                ship_index: shipIndex,
                fleet_name: fleetName
            }),
        });
        
        return response.json();
    },

    tradeInShip: async (systemId: number, planetId: number, shipIndex: number, fleetName: string, tradeInShipIndex: number): Promise<ApiResponse<string>> => {
        const response = await fetch(`${API_BASE_URL}/planet/${systemId}/${planetId}/trade_in_ship`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                ship_index: shipIndex,
                fleet_name: fleetName,
                trade_in_ship_index: tradeInShipIndex
            }),
        });
        return response.json();
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

        const data = await response.json();
        if (!data.success) {
            throw new Error(data.message || 'Failed to buy resource');
        }
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

        const data = await response.json();
        if (!data.success) {
            throw new Error(data.message || 'Failed to sell resource');
        }
        return data.message;
    },

    // Game Settings and Save Management
    listSavedGames: async (): Promise<SavedGame[]> => {
        const response = await fetch(`${API_BASE_URL}/games`);
        return handleApiResponse<SavedGame[]>(response);
    },

    loadGame: async (gameId: string): Promise<void> => {
        const response = await fetch(`${API_BASE_URL}/games/${gameId}/load`);
        return handleApiResponse<void>(response);
    },

    createNewGame: async (settings: GameSettings): Promise<void> => {
        const response = await fetch(`${API_BASE_URL}/games/new`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(settings),
        });
        return handleApiResponse<void>(response);
    },

    getGameSettings: async (): Promise<GameSettings> => {
        const response = await fetch(`${API_BASE_URL}/settings`);
        if (!response.ok) {
            throw new Error(`Failed to load game settings: ${response.status}`);
        }
        const data = await response.json();
        if (!data.success || !data.data) {
            throw new Error('Invalid game settings response');
        }
        return data.data;
    },

    updateGameSettings: async (settings: GameSettings): Promise<void> => {
        const response = await fetch(`${API_BASE_URL}/settings`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(settings),
        });
        return handleApiResponse<void>(response);
    },

    deleteGame: async (gameId: string): Promise<void> => {
        const response = await fetch(`${API_BASE_URL}/games/${gameId}`, {
            method: 'DELETE',
        });
        return handleApiResponse<void>(response);
    },

    getPlayerFleets: async (ownerId?: string): Promise<ApiResponse<Fleet[]>> => {
        try {
            // If no ownerId is provided, get it from settings
            const targetOwnerId = ownerId || (await api.getGameSettings()).player_name;
            if (!targetOwnerId) {
                throw new Error('No player name available');
            }
            
            console.log(`api.getPlayerFleets called for owner: ${targetOwnerId}`);
            const playerFleets = await api.getOwnerFleets(targetOwnerId);
            return {
                success: true,
                message: 'Successfully loaded fleets',
                data: playerFleets
            };
        } catch (error) {
            console.error('Failed to get fleets:', error);
            const errorMessage = error instanceof Error ? error.message : 'Unknown error loading fleets';
            return {
                success: false,
                message: errorMessage,
                data: []
            };
        }
    },
    async clearCaches(): Promise<void> {
        await fetch(`${API_BASE_URL}/clear-caches`, { method: 'POST' });
    },
};