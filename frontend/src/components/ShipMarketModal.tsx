import React, { useEffect, useRef, useState } from 'react';
import { Planet, Ship, Player, Fleet } from '../types/game';
import { api } from '../services/api';
import './StarSystemModal.css';
import { HOST_PLAYER_NAME } from '../constants';

interface ShipMarketModalProps {
    planet: Planet;
    systemId: number;
    planetId: number;
    selectedFleet: Fleet | null;
    onClose: () => void;
}

export const ShipMarketModal: React.FC<ShipMarketModalProps> = ({ 
    planet, 
    systemId, 
    planetId, 
    selectedFleet,
    onClose 
}) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [availableShips, setAvailableShips] = useState<Ship[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [player, setPlayer] = useState<Player | null>(null);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);
    const [selectedShip, setSelectedShip] = useState<Ship | null>(null);
    const [playerFleets, setPlayerFleets] = useState<Fleet[]>([]);
    const [currentFleet, setCurrentFleet] = useState<Fleet | null>(null);
    const [selectedTradeIn, setSelectedTradeIn] = useState<Ship | null>(null);
    const [selectedSellShip, setSelectedSellShip] = useState<Ship | null>(null);

    useEffect(() => {
        const loadData = async () => {
            try {
                setLoading(true);
                setError(null);
                const [shipsData, playerData, fleetsData] = await Promise.all([
                    api.getPlanetShipMarket(systemId, planetId),
                    api.getPlayer(HOST_PLAYER_NAME),
                    api.getFleets(HOST_PLAYER_NAME)
                ]);
                setAvailableShips(shipsData);
                setPlayer(playerData);
                setPlayerFleets(fleetsData);
                
                // Set the first fleet as default if available
                if (fleetsData.length > 0) {
                    setCurrentFleet(fleetsData[0]);
                }
            } catch (error) {
                console.error('Failed to load data:', error);
                setError('Failed to load ship market data. Please try again.');
            } finally {
                setLoading(false);
            }
        };

        loadData();
    }, [systemId, planetId]);

    const handleBuyShip = async (ship: Ship) => {
        if (!currentFleet) {
            setError('No fleet selected to add ship to');
            return;
        }

        if (!player || player.credits < calculateFinalPrice(ship)) {
            setError('Not enough credits to purchase this ship');
            return;
        }

        try {
            console.log('Starting ship purchase:', {
                ship: ship.name,
                fleet: currentFleet.name,
                systemId,
                planetId,
                tradeIn: selectedTradeIn?.name
            });
            
            setLoading(true);
            setError(null);
            setTradeMessage(null);
            
            const result = await api.buyShip(
                systemId, 
                planetId, 
                ship.name, 
                currentFleet.name,
                selectedTradeIn?.name || undefined
            );
            console.log('Purchase result:', result);
            setTradeMessage(result);
            
            console.log('Refreshing data after purchase...');
            // Refresh market data, player data, and fleet data
            const [updatedShips, updatedPlayer, updatedFleets] = await Promise.all([
                api.getPlanetShipMarket(systemId, planetId),
                api.getPlayer(HOST_PLAYER_NAME),
                api.getFleets(HOST_PLAYER_NAME)
            ]);
            
            console.log('Updated data received:', {
                ships: updatedShips.length,
                playerCredits: updatedPlayer.credits,
                fleets: updatedFleets.length
            });
            
            setAvailableShips(updatedShips);
            setPlayer(updatedPlayer);
            setPlayerFleets(updatedFleets);
            
            // Update current fleet with the latest data
            const updatedCurrentFleet = updatedFleets.find(f => f.name === currentFleet.name);
            console.log('Updated current fleet:', updatedCurrentFleet);
            
            if (updatedCurrentFleet) {
                setCurrentFleet(updatedCurrentFleet);
            }
            
            setSelectedShip(null);
            setSelectedTradeIn(null);
        } catch (err) {
            console.error('Ship purchase failed:', err);
            setError('Failed to complete purchase. Please try again.');
        } finally {
            setLoading(false);
        }
    };

    const handleSellShip = async (ship: Ship) => {
        if (!currentFleet) {
            setError('No fleet selected to sell ship from');
            return;
        }

        // Check if this is the last ship in the player's ownership
        const totalShips = playerFleets.reduce((total, fleet) => total + fleet.ships.length, 0);
        if (totalShips <= 1) {
            setError('Cannot sell your last ship. At least one ship is required to continue playing.');
            return;
        }

        try {
            setLoading(true);
            setError(null);
            setTradeMessage(null);
            
            const result = await api.sellShip(systemId, planetId, ship.name, currentFleet.name);
            setTradeMessage(result);
            
            // Refresh market data and player data
            const [updatedShips, updatedPlayer, updatedFleets] = await Promise.all([
                api.getPlanetShipMarket(systemId, planetId),
                api.getPlayer(HOST_PLAYER_NAME),
                api.getFleets(HOST_PLAYER_NAME)
            ]);
            
            setAvailableShips(updatedShips);
            setPlayer(updatedPlayer);
            setPlayerFleets(updatedFleets);
            
            // Update current fleet with the latest data
            const updatedCurrentFleet = updatedFleets.find(f => f.name === currentFleet.name);
            if (updatedCurrentFleet) {
                setCurrentFleet(updatedCurrentFleet);
            }

            // Clear any selected ships
            setSelectedShip(null);
            setSelectedTradeIn(null);
        } catch (err) {
            console.error('Ship sale failed:', err);
            setError('Failed to complete sale. Please try again.');
        } finally {
            setLoading(false);
        }
    };

    const calculateSellPrice = (ship: Ship): number => {
        if (ship.price !== null && ship.price !== undefined) {
            return Math.round(ship.price * 0.75 * 100) / 100; // 75% of original price
        }

        // Fallback calculation if no price stored
        const baseValue = {
            Tiny: 500,
            Small: 1250,
            Medium: 2500,
            Large: 5000,
            Huge: 10000,
            Planetary: 25000
        }[ship.size] || 0;

        const specializationMultiplier = {
            Fighter: 1.1,
            Battleship: 1.8,
            Freighter: 1.3,
            Explorer: 1.5,
            Shuttle: 0.7,
            Capital: 2.5
        }[ship.specialization] || 1;

        const engineMultiplier = {
            Basic: 0.8,
            Advanced: 1.2,
            Experimental: 1.5
        }[ship.engine] || 1;

        const conditionMultiplier = Math.max(ship.hp / 100, 0.5);

        return Math.round(baseValue * specializationMultiplier * engineMultiplier * conditionMultiplier * 100) / 100;
    };

    const calculateFinalPrice = (ship: Ship): number => {
        if (!selectedTradeIn) return Math.round(ship.price || 0);
        
        const tradeInValue = calculateSellPrice(selectedTradeIn);
        // Round to 2 decimal places to avoid floating point issues
        return Math.round(Math.max(0, (ship.price || 0) - tradeInValue) * 100) / 100;
    };

    useEffect(() => {
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                onClose();
            }
        };

        document.addEventListener('keydown', handleEscape);
        return () => document.removeEventListener('keydown', handleEscape);
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
                    <h2>{planet.name} Ship Market</h2>
                    <button className="close-button" onClick={onClose}>&times;</button>
                </div>
                <div className="market-content">
                    <div className="market-header">
                        <select 
                            value={currentFleet?.name || ''} 
                            onChange={(e) => {
                                const fleet = playerFleets.find(f => f.name === e.target.value);
                                if (fleet) {
                                    setCurrentFleet(fleet);
                                    setSelectedShip(null);
                                    setSelectedTradeIn(null);
                                }
                            }}
                            className="fleet-selector"
                        >
                            <option value="">Select Fleet</option>
                            {playerFleets.map((fleet, index) => (
                                <option key={index} value={fleet.name}>
                                    {fleet.name}
                                </option>
                            ))}
                        </select>
                        {player && (
                            <div className="player-credits">
                                Credits: {player.credits.toLocaleString()} cr
                            </div>
                        )}
                    </div>

                    <div className="market-grid">
                        <div className="market-list">
                            <table className="ships-table">
                                <thead>
                                    <tr>
                                        <th>Ship Name</th>
                                        <th>Type</th>
                                        <th>Size</th>
                                        <th>Price</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {availableShips.map((ship, index) => (
                                        <tr 
                                            key={index} 
                                            className={`ship-row ${selectedShip?.name === ship.name ? 'selected-row' : ''}`}
                                            onClick={() => {
                                                if (selectedShip?.name === ship.name) {
                                                    setSelectedShip(null);
                                                    setSelectedTradeIn(null);
                                                } else {
                                                    setSelectedShip(ship);
                                                    setSelectedSellShip(null);
                                                }
                                            }}
                                            style={{ cursor: 'pointer' }}
                                        >
                                            <td>{ship.name}</td>
                                            <td>{ship.specialization}</td>
                                            <td>{ship.size}</td>
                                            <td className="price-cell">
                                                {selectedShip?.name === ship.name && selectedTradeIn ? (
                                                    <div className="price-breakdown">
                                                        <span className="base-price">{ship.price?.toLocaleString()} cr</span>
                                                        <span className="trade-value">-{(ship.price! - calculateFinalPrice(ship)).toLocaleString()} cr</span>
                                                        <span className="final-price">{calculateFinalPrice(ship).toLocaleString()} cr</span>
                                                    </div>
                                                ) : (
                                                    <span className="base-price">{ship.price?.toLocaleString()} cr</span>
                                                )}
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>

                        <div className="details-panel">
                            <div className="details-section">
                                {selectedShip ? (
                                    <div className="ship-details">
                                        <h3>{selectedShip.name}</h3>
                                        <div className="stats-grid">
                                            <div className="stat-box">
                                                <span className="stat-label">Type</span>
                                                <span className="stat-value">{selectedShip.specialization}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">Size</span>
                                                <span className="stat-value">{selectedShip.size}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">Engine</span>
                                                <span className="stat-value">{selectedShip.engine}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">HP</span>
                                                <span className="stat-value">{selectedShip.hp}</span>
                                            </div>
                                        </div>
                                        
                                        <div className="price-summary">
                                            {selectedTradeIn ? (
                                                <>
                                                    <div className="price-line">
                                                        <span>Base Price:</span>
                                                        <span>{selectedShip.price?.toLocaleString()} cr</span>
                                                    </div>
                                                    <div className="price-line trade-in">
                                                        <span>Trade-in Value:</span>
                                                        <span>-{(selectedShip.price! - calculateFinalPrice(selectedShip)).toLocaleString()} cr</span>
                                                    </div>
                                                    <div className="price-line final">
                                                        <span>Final Price:</span>
                                                        <span>{calculateFinalPrice(selectedShip).toLocaleString()} cr</span>
                                                    </div>
                                                </>
                                            ) : (
                                                <div className="price-line final">
                                                    <span>Price:</span>
                                                    <span>{selectedShip.price?.toLocaleString()} cr</span>
                                                </div>
                                            )}
                                        </div>

                                        <div className="action-buttons">
                                            <button 
                                                onClick={() => handleBuyShip(selectedShip)}
                                                disabled={Boolean(!currentFleet || !player || player.credits < calculateFinalPrice(selectedShip))}
                                                className="confirm-button"
                                            >
                                                Confirm Purchase
                                            </button>
                                            {selectedTradeIn && (
                                                <button 
                                                    onClick={() => setSelectedTradeIn(null)}
                                                    className="cancel-button"
                                                >
                                                    Remove Trade-in
                                                </button>
                                            )}
                                        </div>
                                    </div>
                                ) : selectedSellShip ? (
                                    <div className="ship-details sell-mode">
                                        <h3>Sell Ship</h3>
                                        <div className="stats-grid">
                                            <div className="stat-box">
                                                <span className="stat-label">Type</span>
                                                <span className="stat-value">{selectedSellShip.specialization}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">Size</span>
                                                <span className="stat-value">{selectedSellShip.size}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">Engine</span>
                                                <span className="stat-value">{selectedSellShip.engine}</span>
                                            </div>
                                            <div className="stat-box">
                                                <span className="stat-label">HP</span>
                                                <span className="stat-value">{selectedSellShip.hp}</span>
                                            </div>
                                        </div>
                                        
                                        <div className="price-summary">
                                            <div className="price-line final">
                                                <span>Sell Price:</span>
                                                <span>{calculateSellPrice(selectedSellShip).toLocaleString()} cr</span>
                                            </div>
                                        </div>

                                        <div className="action-buttons">
                                            <button 
                                                onClick={() => {
                                                    // Check if this is the last ship
                                                    const totalShips = playerFleets.reduce((total, fleet) => total + fleet.ships.length, 0);
                                                    if (totalShips <= 1) {
                                                        setError('Cannot sell your last ship. At least one ship is required to continue playing.');
                                                        return;
                                                    }
                                                    handleSellShip(selectedSellShip);
                                                }}
                                                className="confirm-button"
                                            >
                                                Confirm Sale
                                            </button>
                                            <button 
                                                onClick={() => setSelectedSellShip(null)}
                                                className="cancel-button"
                                            >
                                                Cancel
                                            </button>
                                        </div>
                                    </div>
                                ) : (
                                    <div className="no-selection">
                                        <p>Select a ship to view details</p>
                                    </div>
                                )}
                            </div>

                            {currentFleet && (
                                <div className="trade-in-panel">
                                    <h3>Your Ships</h3>
                                    <div className="trade-in-grid">
                                        {currentFleet.ships.map((ship, index) => (
                                            <div 
                                                key={index} 
                                                className={`trade-in-card ${
                                                    (selectedTradeIn?.name === ship.name || selectedSellShip?.name === ship.name) 
                                                    ? 'selected' : ''
                                                }`}
                                                onClick={() => {
                                                    if (selectedShip) {
                                                        // In buy mode - handle trade-in
                                                        if (selectedTradeIn?.name === ship.name) {
                                                            setSelectedTradeIn(null);
                                                        } else {
                                                            setSelectedTradeIn(ship);
                                                        }
                                                        setSelectedSellShip(null);
                                                    } else {
                                                        // In sell mode
                                                        if (selectedSellShip?.name === ship.name) {
                                                            setSelectedSellShip(null);
                                                        } else {
                                                            setSelectedSellShip(ship);
                                                        }
                                                        setSelectedTradeIn(null);
                                                    }
                                                }}
                                                style={{ cursor: 'pointer' }}
                                            >
                                                <div className="ship-info">
                                                    <h4>{ship.name}</h4>
                                                    <div className="ship-quick-stats">
                                                        <span>{ship.specialization}</span>
                                                        <span>{ship.size}</span>
                                                        <span>HP: {ship.hp}</span>
                                                    </div>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}; 