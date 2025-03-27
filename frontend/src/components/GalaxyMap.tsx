import React, { useState, useEffect } from 'react';
import { StarSystem, Planet, Resource } from '../types/game';
import { api } from '../services/api';
import './GalaxyMap.css';

interface Player {
    name: string;
    credits: number;
    resources: Resource[];
}

export const GalaxyMap: React.FC = () => {
    const [systems, setSystems] = useState<StarSystem[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedSystem, setSelectedSystem] = useState<StarSystem | null>(null);
    const [selectedPlanet, setSelectedPlanet] = useState<Planet | null>(null);
    const [marketResources, setMarketResources] = useState<Resource[]>([]);
    const [marketLoading, setMarketLoading] = useState(false);
    const [marketError, setMarketError] = useState<string | null>(null);
    const [tradeAmount, setTradeAmount] = useState<number>(1);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);
    const [currentSystemId, setCurrentSystemId] = useState<number>(-1);
    const [player, setPlayer] = useState<Player | null>(null);
    const [showMarket, setShowMarket] = useState(false);

    useEffect(() => {
        const fetchGalaxyMap = async () => {
            try {
                const data = await api.getGalaxyMap();
                setSystems(data);
            } catch (err) {
                setError('Failed to load galaxy map');
                console.error(err);
            } finally {
                setLoading(false);
            }
        };

        const fetchPlayer = async () => {
            try {
                const data = await api.getPlayer('Igor');
                setPlayer(data);
            } catch (err) {
                console.error('Failed to load player data:', err);
            }
        };

        fetchGalaxyMap();
        fetchPlayer();
    }, []);

    const handleViewDetails = async (systemId: number) => {
        try {
            const system = await api.getStarSystem(systemId);
            setSelectedSystem(system);
            setCurrentSystemId(systemId);
        } catch (err) {
            console.error('Failed to load star system details:', err);
        }
    };

    const handleViewMarket = async (systemId: number, planetId: number) => {
        try {
            setMarketLoading(true);
            setMarketError(null);
            const resources = await api.getPlanetMarket(systemId, planetId);
            setMarketResources(resources);
            if (selectedSystem) {
                setSelectedPlanet(selectedSystem.planets[planetId]);
            }
            setShowMarket(true);
        } catch (err) {
            setMarketError('Failed to load market data');
            console.error('Failed to load market:', err);
        } finally {
            setMarketLoading(false);
        }
    };

    const handleCloseMarket = () => {
        setShowMarket(false);
        setSelectedPlanet(null);
        setMarketResources([]);
    };

    const handleTrade = async (resource: Resource, isBuying: boolean) => {
        if (!selectedPlanet || currentSystemId === -1) return;

        try {
            setMarketLoading(true);
            setTradeMessage(null);
            
            const planetId = selectedSystem?.planets.findIndex(p => p.name === selectedPlanet.name) ?? -1;
            if (planetId === -1) return;

            const result = isBuying
                ? await api.buyFromPlanet(currentSystemId, planetId, resource.resource_type, tradeAmount)
                : await api.sellToPlanet(currentSystemId, planetId, resource.resource_type, tradeAmount);

            setTradeMessage(result);
            
            // Refresh market data and player data
            const [updatedResources, updatedPlayer] = await Promise.all([
                api.getPlanetMarket(currentSystemId, planetId),
                api.getPlayer('Igor')
            ]);
            
            setMarketResources(updatedResources);
            setPlayer(updatedPlayer);
        } catch (err) {
            setTradeMessage('Failed to complete trade');
            console.error('Trade failed:', err);
        } finally {
            setMarketLoading(false);
        }
    };

    if (loading) return <div>Loading galaxy map...</div>;
    if (error) return <div className="error">{error}</div>;

    return (
        <div className="galaxy-map">
            <h2>Galaxy Map</h2>
            <div className="systems-grid">
                {systems.map((system, index) => (
                    <div key={index} className="star-system-card">
                        <h3>Star System {index + 1}</h3>
                        <p>Position: ({system.position.x}, {system.position.y}, {system.position.z})</p>
                        <p>Planets: {system.planets.length}</p>
                        <button onClick={() => handleViewDetails(index)}>
                            View Details
                        </button>
                    </div>
                ))}
            </div>

            {selectedSystem && (
                <div className="star-system-details">
                    <h2>Star System Details</h2>
                    <div className="details-content">
                        <h3>Position: ({selectedSystem.position.x}, {selectedSystem.position.y}, {selectedSystem.position.z})</h3>
                        <h3>Planets</h3>
                        <table className="planets-table">
                            <thead>
                                <tr>
                                    <th>Name</th>
                                    <th>Biome</th>
                                    <th>Economy</th>
                                    <th>Specialization</th>
                                    <th>Danger Level</th>
                                    <th>Actions</th>
                                </tr>
                            </thead>
                            <tbody>
                                {selectedSystem.planets.map((planet, planetIndex) => (
                                    <tr key={planetIndex}>
                                        <td>{planet.name}</td>
                                        <td>{planet.biome}</td>
                                        <td>{planet.economy}</td>
                                        <td>{planet.specialization}</td>
                                        <td>{planet.danger}</td>
                                        <td>
                                            <button onClick={() => handleViewMarket(currentSystemId, planetIndex)}>
                                                View Market
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}

            {showMarket && selectedPlanet && (
                <div className="market-overlay">
                    <div className="market-popup">
                        <button className="close-button" onClick={handleCloseMarket}>×</button>
                        <div className="market-header">
                            <div className="planet-info">
                                <h2>{selectedPlanet.name}</h2>
                                <div className="planet-details">
                                    <p><strong>Biome:</strong> {selectedPlanet.biome}</p>
                                    <p><strong>Economy:</strong> {selectedPlanet.economy}</p>
                                    <p><strong>Specialization:</strong> {selectedPlanet.specialization}</p>
                                    <p><strong>Danger Level:</strong> {selectedPlanet.danger}</p>
                                    <p><strong>Position:</strong> ({selectedPlanet.position.x}, {selectedPlanet.position.y}, {selectedPlanet.position.z})</p>
                                </div>
                            </div>
                            {player && (
                                <div className="player-info">
                                    <h3>{player.name}</h3>
                                    <p>Credits: {player.credits.toFixed(2)}</p>
                                    <h4>Resources</h4>
                                    <table className="resource-table">
                                        <thead>
                                            <tr>
                                                <th>Resource</th>
                                                <th>Quantity</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {player.resources.map((resource, index) => (
                                                <tr key={index}>
                                                    <td>{resource.resource_type}</td>
                                                    <td>{resource.quantity || 0}</td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            )}
                        </div>
                        <div className="trade-controls">
                            <label>
                                Trade Amount:
                                <input
                                    type="number"
                                    min="1"
                                    value={tradeAmount}
                                    onChange={(e) => setTradeAmount(Math.max(1, parseInt(e.target.value) || 1))}
                                />
                            </label>
                        </div>
                        {marketLoading ? (
                            <div>Loading market data...</div>
                        ) : marketError ? (
                            <div className="error">{marketError}</div>
                        ) : (
                            <>
                                {tradeMessage && (
                                    <div className={`trade-message ${tradeMessage.includes('Successfully') ? 'success' : 'error'}`}>
                                        {tradeMessage}
                                    </div>
                                )}
                                <table className="market-table">
                                    <thead>
                                        <tr>
                                            <th>Resource</th>
                                            <th>Buy Price</th>
                                            <th>Sell Price</th>
                                            <th>Available</th>
                                            <th>Actions</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {marketResources.map((resource, index) => (
                                            <tr key={index}>
                                                <td>{resource.resource_type}</td>
                                                <td>{resource.buy?.toFixed(2) || 'N/A'}</td>
                                                <td>{resource.sell?.toFixed(2) || 'N/A'}</td>
                                                <td>{resource.quantity || 0}</td>
                                                <td className="trade-actions">
                                                    <button 
                                                        onClick={() => handleTrade(resource, true)}
                                                        disabled={!resource.buy || (resource.quantity || 0) < tradeAmount}
                                                        className="buy-button"
                                                    >
                                                        Buy
                                                    </button>
                                                    <button 
                                                        onClick={() => handleTrade(resource, false)}
                                                        disabled={!resource.sell || !player?.resources.find(r => 
                                                            r.resource_type === resource.resource_type && 
                                                            (r.quantity || 0) >= tradeAmount
                                                        )}
                                                        className="sell-button"
                                                    >
                                                        Sell
                                                    </button>
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
};
