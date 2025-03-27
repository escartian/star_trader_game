import { Player, StarSystem, Fleet, Resource } from '../types/game';

const API_BASE_URL = 'http://localhost:8000/api';

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
    }
};