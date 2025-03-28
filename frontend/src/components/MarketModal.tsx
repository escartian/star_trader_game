import React, { useEffect, useRef, useState } from 'react';
import { Planet, Resource, Player } from '../types/game';
import { api } from '../services/api';
import './StarSystemModal.css';

interface MarketModalProps {
    planet: Planet;
    systemId: number;
    planetId: number;
    onClose: () => void;
}

export const MarketModal: React.FC<MarketModalProps> = ({ planet, systemId, planetId, onClose }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [market, setMarket] = useState<Resource[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [player, setPlayer] = useState<Player | null>(null);
    const [tradeAmount, setTradeAmount] = useState<number>(1);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);
    const [selectedResource, setSelectedResource] = useState<Resource | null>(null);
    const [totalCost, setTotalCost] = useState<number>(0);

    useEffect(() => {
        const loadData = async () => {
            try {
                setLoading(true);
                setError(null);
                const [marketData, playerData] = await Promise.all([
                    api.getPlanetMarket(systemId, planetId),
                    api.getPlayer('Igor')
                ]);
                setMarket(marketData);
                setPlayer(playerData);
            } catch (error) {
                console.error('Failed to load data:', error);
                setError('Failed to load market data. Please try again.');
            } finally {
                setLoading(false);
            }
        };

        loadData();
    }, [systemId, planetId]);

    useEffect(() => {
        if (selectedResource) {
            const cost = selectedResource.buy ? selectedResource.buy * tradeAmount : 0;
            setTotalCost(cost);
        }
    }, [selectedResource, tradeAmount]);

    const handleTrade = async (resource: Resource, isBuying: boolean) => {
        try {
            setLoading(true);
            setError(null);
            setTradeMessage(null);
            
            const result = isBuying
                ? await api.buyFromPlanet(systemId, planetId, resource.resource_type, tradeAmount)
                : await api.sellToPlanet(systemId, planetId, resource.resource_type, tradeAmount);

            setTradeMessage(result);
            
            // Refresh market data and player data
            const [updatedResources, updatedPlayer] = await Promise.all([
                api.getPlanetMarket(systemId, planetId),
                api.getPlayer('Igor')
            ]);
            
            setMarket(updatedResources);
            setPlayer(updatedPlayer);
            
            // Reset selected resource and trade amount after successful trade
            setSelectedResource(null);
            setTradeAmount(1);
            setTotalCost(0);
        } catch (err) {
            console.error('Trade failed:', err);
            setError('Failed to complete trade. Please try again.');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                onClose();
            }
        };

        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose]);

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose();
        }
    };

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="market-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="market-modal-header">
                    <h2>{planet.name} Market</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="market-content">
                    <div className="planet-info">
                        <div className="planet-details">
                            <p><strong>Biome:</strong> {planet.biome}</p>
                            <p><strong>Economy:</strong> {planet.economy}</p>
                            <p><strong>Specialization:</strong> {planet.specialization}</p>
                            <p><strong>Danger Level:</strong> {planet.danger}</p>
                        </div>
                        {player && (
                            <div className="player-info">
                                <h3>Your Resources</h3>
                                <div className="player-resources">
                                    <table className="resources-table">
                                        <thead>
                                            <tr>
                                                <th>Resource</th>
                                                <th>Amount</th>
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
                                    <div className="credits">
                                        <strong>Credits:</strong> {player.credits.toFixed(2)}
                                    </div>
                                </div>
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
                                onChange={(e) => {
                                    const newAmount = Math.max(1, parseInt(e.target.value) || 1);
                                    setTradeAmount(newAmount);
                                    if (selectedResource) {
                                        const buyCost = selectedResource.buy ? selectedResource.buy * newAmount : 0;
                                        const sellValue = selectedResource.sell ? selectedResource.sell * newAmount : 0;
                                        setTotalCost(buyCost);
                                    }
                                }}
                            />
                        </label>
                        {selectedResource && (
                            <div className="trade-prices">
                                {selectedResource.buy && (
                                    <div className="buy-price">
                                        Buy Cost: {(selectedResource.buy * tradeAmount).toFixed(2)} credits
                                    </div>
                                )}
                                {selectedResource.sell && (
                                    <div className="sell-price">
                                        Sell Value: {(selectedResource.sell * tradeAmount).toFixed(2)} credits
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                    {loading ? (
                        <div className="loading">Loading market data...</div>
                    ) : error ? (
                        <div className="error">{error}</div>
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
                                    {market.map((resource, index) => (
                                        <tr 
                                            key={index}
                                            className={selectedResource?.resource_type === resource.resource_type ? 'selected' : ''}
                                            onClick={() => {
                                                setSelectedResource(resource);
                                                const buyCost = resource.buy ? resource.buy * tradeAmount : 0;
                                                const sellValue = resource.sell ? resource.sell * tradeAmount : 0;
                                                setTotalCost(buyCost);
                                            }}
                                        >
                                            <td>{resource.resource_type}</td>
                                            <td>{resource.buy ? (resource.buy * tradeAmount).toFixed(2) : 'N/A'}</td>
                                            <td>{resource.sell ? (resource.sell * tradeAmount).toFixed(2) : 'N/A'}</td>
                                            <td>{resource.quantity || 0}</td>
                                            <td className="trade-actions">
                                                <button 
                                                    onClick={() => handleTrade(resource, true)}
                                                    disabled={
                                                        !resource.buy || 
                                                        (resource.quantity || 0) < tradeAmount ||
                                                        (player ? resource.buy * tradeAmount > player.credits : true)
                                                    }
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
        </div>
    );
}; 