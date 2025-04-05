import React, { useEffect, useRef, useState } from 'react';
import { Planet, Resource, Player, Market } from '../types/game';
import { api } from '../services/api';
import './MarketModal.css';
import { ApiResponse } from '../types/api';

interface MarketModalProps {
    isOpen: boolean;
    onClose: () => void;
    systemId: number;
    planetId: number;
    planet: Planet;
}

export const MarketModal: React.FC<MarketModalProps> = ({ isOpen, onClose, systemId, planetId, planet }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [market, setMarket] = useState<Market | null>(null);
    const [player, setPlayer] = useState<Player | null>(null);
    const [selectedResource, setSelectedResource] = useState<Resource | null>(null);
    const [tradeAmount, setTradeAmount] = useState<number>(1);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const loadData = async () => {
            try {
                setLoading(true);
                setError(null);
                
                // Load settings first to get player name
                const settings = await api.getGameSettings();
                
                // Load market and player data in parallel
                const [marketResponse, playerResponse] = await Promise.all([
                    api.getPlanetMarket(systemId, planetId),
                    api.getPlayer(settings.player_name)
                ]);

                setMarket(marketResponse);
                setPlayer(playerResponse);
            } catch (err) {
                console.error('Error loading market data:', err);
                setError(err instanceof Error ? err.message : 'Failed to load market data');
            } finally {
                setLoading(false);
            }
        };

        if (isOpen) {
            loadData();
        }
    }, [systemId, planetId, isOpen]);

    const handleBuy = async () => {
        if (!selectedResource || !market) return;

        try {
            setTradeMessage(null);
            const response = await api.buyResource(
                systemId,
                planetId,
                selectedResource.resource_type,
                tradeAmount
            );

            // Refresh data
            const settings = await api.getGameSettings();
            const [updatedMarket, updatedPlayer] = await Promise.all([
                api.getPlanetMarket(systemId, planetId),
                api.getPlayer(settings.player_name)
            ]);

            setMarket(updatedMarket);
            setPlayer(updatedPlayer);
            setTradeMessage(response);
            // Keep the selection and only reset the trade amount
            setTradeAmount(1);
        } catch (err) {
            console.error('Buy error:', err);
            setTradeMessage(err instanceof Error ? err.message : 'Failed to buy resource');
        }
    };

    const handleSell = async () => {
        if (!selectedResource || !market) return;

        try {
            setTradeMessage(null);
            const response = await api.sellResource(
                systemId,
                planetId,
                selectedResource.resource_type,
                tradeAmount
            );

            // Refresh data
            const settings = await api.getGameSettings();
            const [updatedMarket, updatedPlayer] = await Promise.all([
                api.getPlanetMarket(systemId, planetId),
                api.getPlayer(settings.player_name)
            ]);

            setMarket(updatedMarket);
            setPlayer(updatedPlayer);
            setTradeMessage(response);
            // Keep the selection and only reset the trade amount
            setTradeAmount(1);
        } catch (err) {
            console.error('Sell error:', err);
            setTradeMessage(err instanceof Error ? err.message : 'Failed to sell resource');
        }
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === modalRef.current?.parentElement) {
            onClose();
        }
    };

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const calculateTotalPrice = (price: number | null | undefined, amount: number) => {
        return price ? (price * amount).toFixed(2) : 'N/A';
    };

    if (!isOpen) return null;
    if (loading) return <div className="modal-overlay"><div className="market-modal">Loading market...</div></div>;
    if (error) return <div className="modal-overlay"><div className="market-modal error">{error}</div></div>;
    if (!market) return <div className="modal-overlay"><div className="market-modal">No market available</div></div>;

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="market-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="market-modal-header">
                    <h2>{planet.name} Market</h2>
                    <div className="market-info">
                        <span className="market-spec">
                            <strong>Specialization:</strong> {planet.specialization}
                        </span>
                        <span className="market-econ">
                            <strong>Economy:</strong> {planet.economy}
                        </span>
                        {player && (
                            <span className="player-credits">
                                <strong>Credits:</strong> {player.credits.toLocaleString()} cr
                            </span>
                        )}
                    </div>
                    <button className="close-button" onClick={onClose}>&times;</button>
                </div>

                <div className="market-sections">
                    <div className="market-section">
                        <h3>Available Resources</h3>
                        <table className="resources-grid">
                            <thead>
                                <tr>
                                    <th>Resource</th>
                                    <th>Available</th>
                                    <th>Buy Price (cr)</th>
                                    <th>Sell Price (cr)</th>
                                    <th>Your Stock</th>
                                </tr>
                            </thead>
                            <tbody>
                                {market.resources.map((resource) => {
                                    const playerResource = player?.resources.find(r => r.resource_type === resource.resource_type);
                                    const playerQuantity = playerResource?.quantity || 0;
                                    const isSelected = selectedResource?.resource_type === resource.resource_type;
                                    const isUpdated = isSelected && tradeMessage?.includes('Successfully');
                                    
                                    return (
                                        <tr 
                                            key={resource.resource_type} 
                                            className={`${isSelected ? 'selected' : ''} ${isUpdated ? 'updated' : ''}`}
                                            onClick={() => setSelectedResource(resource)}
                                        >
                                            <td>{resource.resource_type}</td>
                                            <td className={isUpdated ? 'updated-value' : ''}>
                                                {resource.quantity?.toLocaleString() || '0'}
                                            </td>
                                            <td>{resource.buy ? `${resource.buy.toFixed(2)}` : 'N/A'}</td>
                                            <td>{resource.sell ? `${resource.sell.toFixed(2)}` : 'N/A'}</td>
                                            <td className={isUpdated ? 'updated-value' : ''}>
                                                {playerQuantity.toLocaleString()}
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>

                    {selectedResource ? (
                        <div className="trade-actions">
                            <h3>Trade Actions</h3>
                            <div className="trade-amount">
                                <label>Amount:</label>
                                <input
                                    type="number"
                                    min="1"
                                    value={tradeAmount}
                                    onChange={(e) => setTradeAmount(Math.max(1, parseInt(e.target.value) || 1))}
                                    className="amount-input"
                                />
                            </div>
                            <div className="trade-buttons">
                                {selectedResource.buy && (
                                    <button 
                                        className="buy-button"
                                        onClick={handleBuy}
                                    >
                                        Buy ({calculateTotalPrice(selectedResource.buy, tradeAmount)} cr)
                                    </button>
                                )}
                                {selectedResource.sell && (
                                    <button 
                                        className="sell-button"
                                        onClick={handleSell}
                                    >
                                        Sell ({calculateTotalPrice(selectedResource.sell, tradeAmount)} cr)
                                    </button>
                                )}
                            </div>
                            {tradeMessage && (
                                <div className={`trade-message ${tradeMessage.includes('Successfully') ? 'success' : 'error'}`}>
                                    {tradeMessage}
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="trade-actions">
                            <h3>Trade Actions</h3>
                            <div className="no-selection-message">
                                Select a resource to trade
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}; 