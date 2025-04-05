import React, { useState, useEffect } from 'react';
import { api } from '../services/api';
import { Ship, Fleet, ShipMarketProps } from '../types';
import { ShipMarket } from '../types/game';

export const ShipMarketComponent: React.FC<ShipMarketProps> = ({ systemId, planetId }) => {
    const [ships, setShips] = useState<Ship[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [playerFleets, setPlayerFleets] = useState<Fleet[]>([]);

    useEffect(() => {
        const loadData = async () => {
            try {
                // Load player fleets first
                const fleetsResponse = await api.getPlayerFleets();
                if (fleetsResponse.success) {
                    setPlayerFleets(fleetsResponse.data);
                }

                // Then load ship market
                const response = await api.getPlanetShipMarket(systemId, planetId);
                if (response.success) {
                    setShips(response.data.ships);
                } else {
                    setError(response.message);
                }
            } catch (err) {
                setError('Failed to load ship market');
            } finally {
                setLoading(false);
            }
        };

        loadData();
    }, [systemId, planetId]);

    const handleBuyShip = async (shipIndex: number) => {
        try {
            // Use the first fleet if available, otherwise create a new one
            const fleetName = playerFleets.length > 0 
                ? playerFleets[0].name 
                : undefined;

            const response = await api.buyShip(systemId, planetId, shipIndex, fleetName);
            if (response.success) {
                // Refresh the market and fleets
                const marketResponse = await api.getPlanetShipMarket(systemId, planetId);
                if (marketResponse.success) {
                    setShips(marketResponse.data.ships);
                }

                const fleetsResponse = await api.getPlayerFleets();
                if (fleetsResponse.success) {
                    setPlayerFleets(fleetsResponse.data);
                }
            } else {
                setError(response.message);
            }
        } catch (err) {
            setError('Failed to buy ship');
        }
    };

    if (loading) return <div>Loading ship market...</div>;
    if (error) return <div className="error">{error}</div>;

    return (
        <div className="ship-market">
            <h3>Available Ships</h3>
            <div className="ships-grid">
                {ships.map((ship, index) => (
                    <div key={index} className="ship-card">
                        <h4>{ship.name}</h4>
                        <p>Type: {ship.specialization}</p>
                        <p>Size: {ship.size}</p>
                        <p>Price: {ship.price?.toFixed(2) || 'N/A'}</p>
                        <button 
                            onClick={() => handleBuyShip(index)}
                            disabled={!ship.price}
                        >
                            Buy Ship
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default ShipMarketComponent; 