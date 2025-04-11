import React, { useEffect, useRef, useState } from 'react';
import { Ship, Player, Fleet, ShipMarket } from '../types/game';
import { api } from '../services/api';
import './ShipMarketModal.css';
import { ApiResponse } from '../types/api';

const calculateShipPrice = (ship: Ship): number => {
    const basePrice = {
        Tiny: 1000,
        Small: 2500,
        Medium: 5000,
        Large: 10000,
        Huge: 20000,
        Planetary: 50000
    }[ship.size] || 1000;

    const specializationMultiplier = {
        Fighter: 1.1,
        Battleship: 1.8,
        Freighter: 1.3,
        Explorer: 1.5,
        Shuttle: 0.7,
        Capital: 2.5
    }[ship.specialization] || 1.0;

    const engineMultiplier = {
        Basic: 0.8,
        Advanced: 1.2,
        Experimental: 1.5
    }[ship.engine] || 1.0;

    const conditionMultiplier = Math.max(ship.hp / 100, 0.5);

    return Math.floor(basePrice * specializationMultiplier * engineMultiplier * conditionMultiplier);
};

interface ShipMarketModalProps {
    isOpen: boolean;
    onClose: () => void;
    systemId: number;
    planetId: number;
}

export const ShipMarketModal: React.FC<ShipMarketModalProps> = ({ isOpen, onClose, systemId, planetId }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [shipMarket, setShipMarket] = useState<ShipMarket | null>(null);
    const [player, setPlayer] = useState<Player | null>(null);
    const [fleets, setFleets] = useState<Fleet[]>([]);
    const [selectedFleet, setSelectedFleet] = useState<string | null>(null);
    const [selectedMarketShip, setSelectedMarketShip] = useState<Ship | null>(null);
    const [selectedFleetShip, setSelectedFleetShip] = useState<Ship | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);

    useEffect(() => {
        const loadData = async () => {
            try {
                setLoading(true);
                setError(null);
                setSelectedMarketShip(null);
                setSelectedFleetShip(null);
                
                // Load settings first to get player name
                const settings = await api.getGameSettings();
                
                // Load ship market and player data in parallel
                const [shipMarketResponse, playerResponse] = await Promise.all([
                    api.getPlanetShipMarket(systemId, planetId),
                    api.getPlayer(settings.player_name)
                ]);

                if (shipMarketResponse.success && shipMarketResponse.data) {
                    setShipMarket(shipMarketResponse.data);
                } else {
                    setError(shipMarketResponse.message);
                }

                if (playerResponse) {
                    setPlayer(playerResponse);
                    console.log('Player credits loaded:', playerResponse.credits);
                }

                // Load player's fleets
                const fleetsResponse = await api.getPlayerFleets(settings.player_name);
                if (fleetsResponse.success && fleetsResponse.data) {
                    setFleets(fleetsResponse.data);

                    // Select the first fleet by default if available
                    if (fleetsResponse.data.length > 0) {
                        setSelectedFleet(fleetsResponse.data[0].name);
                    }
                } else {
                    setError('Failed to load fleets');
                }
            } catch (err) {
                console.error('Error loading ship market data:', err);
                setError(err instanceof Error ? err.message : 'Failed to load ship market data');
            } finally {
                setLoading(false);
            }
        };

        if (isOpen) {
            loadData();
        }
    }, [systemId, planetId, isOpen]);

    const handleBuyShip = async (ship: Ship, index: number) => {
        if (!selectedFleet) {
            setTradeMessage('Please select a fleet first');
            return;
        }

        try {
            setTradeMessage('Processing purchase...');
            
            console.log('Buying ship:', {
                systemId,
                planetId,
                shipIndex: index,
                fleetName: selectedFleet
            });
            
            // Buy the ship
            const result = await api.buyShip(systemId, planetId, index, selectedFleet);
            console.log('Buy ship response:', result);
            
            // Update all data after the purchase
            await updateAllData();
            
            // Show result and clear selections
            const message = result?.message || 'Purchase completed successfully';
            setTradeMessage(message);
            setSelectedMarketShip(null);
            setSelectedFleetShip(null);
        } catch (err) {
            console.error('Buy ship error:', err);
            setTradeMessage('Failed to buy ship');
        }
    };

    const handleTradeInShip = async () => {
        if (!selectedFleet || !selectedFleetShip || !selectedMarketShip || !shipMarket) return;

        try {
            const selectedFleetData = fleets.find(f => f.name === selectedFleet);
            if (!selectedFleetData) {
                setTradeMessage('Selected fleet not found');
                return;
            }

            // Find the correct indices in the arrays
            const fleetShipIndex = selectedFleetData.ships.findIndex(s => 
                s.name === selectedFleetShip.name && 
                s.specialization === selectedFleetShip.specialization && 
                s.size === selectedFleetShip.size
            );
            
            const marketShipIndex = shipMarket.ships.findIndex(s => 
                s.name === selectedMarketShip.name && 
                s.specialization === selectedMarketShip.specialization && 
                s.size === selectedMarketShip.size
            );
            
            if (fleetShipIndex === -1) {
                setTradeMessage('Selected trade-in ship not found in fleet');
                return;
            }

            if (marketShipIndex === -1) {
                setTradeMessage('Selected market ship not found');
                return;
            }

            console.log('Trading in ship:', { 
                fleetShipIndex, 
                marketShipIndex,
                fleetShip: selectedFleetShip.name,
                marketShip: selectedMarketShip.name
            });

            const tradeInValue = Math.floor(calculateShipPrice(selectedFleetShip) * 0.7);
            const marketShipPrice = selectedMarketShip.price || calculateShipPrice(selectedMarketShip);
            const finalCost = marketShipPrice - tradeInValue;

            if (player && player.credits < finalCost) {
                setTradeMessage('Not enough credits for trade after trade-in value');
                return;
            }

            // Show that we're processing the trade
            setTradeMessage('Processing trade...');

            // Execute the trade
            const result = await api.tradeInShip(
                systemId,
                planetId,
                marketShipIndex,
                selectedFleet,
                fleetShipIndex
            );
            
            console.log('Trade-in response:', result);
            
            // Update all data after the trade
            await updateAllData();
            
            // Show result and clear selections
            const message = result?.message || 'Trade completed successfully';
            setTradeMessage(message);
            setSelectedMarketShip(null);
            setSelectedFleetShip(null);
        } catch (err) {
            console.error('Trade in error:', err);
            setTradeMessage('Failed to trade in ship');
        }
    };

    const handleSellShip = async (ship: Ship, index: number) => {
        if (!selectedFleet) {
            setTradeMessage('Please select a fleet first');
            return;
        }

        try {
            setTradeMessage('Processing sale...');
            
            console.log('Selling ship:', {
                systemId,
                planetId, 
                shipIndex: index,
                fleetName: selectedFleet
            });
            
            // Sell the ship
            const result = await api.sellShip(systemId, planetId, index, selectedFleet);
            console.log('Sell ship response:', result);
            
            // Update all data after the sale
            await updateAllData();
            
            // Show result and clear selections
            setTradeMessage('Sale completed successfully');
            setSelectedMarketShip(null);
            setSelectedFleetShip(null);
        } catch (err) {
            console.error('Sell ship error:', err);
            setTradeMessage('Failed to sell ship');
        }
    };

    // Helper function to update all data
    const updateAllData = async () => {
        try {
            console.log('Updating all data after transaction...');
            
            // Get settings to get player name
            const settings = await api.getGameSettings();
            console.log('Player name from settings:', settings.player_name);
            
            try {
                // Get updated player data (with credits)
                const playerResponse = await api.getPlayer(settings.player_name);
                if (playerResponse) {
                    setPlayer(playerResponse);
                    console.log('Updated player credits:', playerResponse.credits);
                } else {
                    console.warn('Player response was empty');
                }
            } catch (playerError) {
                console.error('Error updating player data:', playerError);
            }
            
            try {
                // Get updated fleets
                const fleetsResponse = await api.getPlayerFleets(settings.player_name);
                if (fleetsResponse.success && fleetsResponse.data) {
                    setFleets(fleetsResponse.data);
                    console.log('Updated fleets:', fleetsResponse.data.length);
                    
                    // Re-select the same fleet if it still exists
                    if (selectedFleet && fleetsResponse.data.find(f => f.name === selectedFleet)) {
                        setSelectedFleet(selectedFleet);
                    } else if (fleetsResponse.data.length > 0) {
                        // Otherwise select the first fleet
                        setSelectedFleet(fleetsResponse.data[0].name);
                    } else {
                        setSelectedFleet(null);
                    }
                } else {
                    console.warn('Fleets response was unsuccessful or empty');
                }
            } catch (fleetsError) {
                console.error('Error updating fleets data:', fleetsError);
            }
            
            try {
                // Get updated ship market
                const marketResponse = await api.getPlanetShipMarket(systemId, planetId);
                if (marketResponse.success && marketResponse.data) {
                    setShipMarket(marketResponse.data);
                    console.log('Updated ship market:', marketResponse.data.ships.length, 'ships');
                } else {
                    console.warn('Market response was unsuccessful or empty');
                }
            } catch (marketError) {
                console.error('Error updating ship market data:', marketError);
            }
            
            console.log('All data updated successfully');
        } catch (error) {
            console.error('Error in updateAllData:', error);
            setTradeMessage('Error updating data after transaction');
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

    const handleMarketShipClick = (ship: Ship) => {
        if (selectedMarketShip && selectedMarketShip.name === ship.name) {
            setSelectedMarketShip(null);
        } else {
            setSelectedMarketShip(ship);
        }
    };

    const handleFleetShipClick = (ship: Ship) => {
        if (selectedFleetShip && selectedFleetShip.name === ship.name) {
            setSelectedFleetShip(null);
        } else {
            setSelectedFleetShip(ship);
        }
    };

    const refreshPlayerData = async () => {
        try {
            const settings = await api.getGameSettings();
            const [playerResponse, fleetsResponse] = await Promise.all([
                api.getPlayer(settings.player_name),
                api.getPlayerFleets(settings.player_name)
            ]);

            if (playerResponse) {
                setPlayer(playerResponse);
            }
            if (fleetsResponse.success && fleetsResponse.data) {
                setFleets(fleetsResponse.data);
                // If the selected fleet no longer exists, clear the selection
                if (!fleetsResponse.data.find(f => f.name === selectedFleet)) {
                    setSelectedFleet(null);
                }
            }
        } catch (err) {
            console.error('Error refreshing player data:', err);
            setTradeMessage('Error refreshing data');
        }
    };

    if (!isOpen) return null;
    if (loading) return <div className="modal-overlay"><div className="ship-market-modal">Loading ship market...</div></div>;
    if (error) return <div className="modal-overlay"><div className="ship-market-modal error">{error}</div></div>;
    if (!shipMarket) return <div className="modal-overlay"><div className="ship-market-modal">Loading ship market data...</div></div>;

    const selectedFleetData = fleets.find(f => f.name === selectedFleet);
    const tradeInValue = selectedFleetShip ? Math.floor(calculateShipPrice(selectedFleetShip) * 0.7) : 0;
    const marketShipPrice = selectedMarketShip?.price || 0;
    const finalCost = marketShipPrice - tradeInValue;
    const canAfford = player && player.credits >= finalCost;

    // Display current player credits with formatting
    const displayCredits = player ? (
        <span className="player-credits">
            <strong>Credits:</strong> {player.credits.toLocaleString()} cr
        </span>
    ) : null;

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="ship-market-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="ship-market-modal-header">
                    <h2>Ship Market</h2>
                    <div className="fleet-selection">
                        <h3>Fleet:</h3>
                        <div className="fleet-buttons">
                            {fleets.map((fleet) => (
                                <button
                                    key={fleet.name}
                                    className={`fleet-button ${selectedFleet === fleet.name ? 'selected' : ''}`}
                                    onClick={() => setSelectedFleet(fleet.name)}
                                >
                                    {fleet.name} ({fleet.ships.length})
                                </button>
                            ))}
                        </div>
                    </div>
                    {displayCredits}
                    <button className="close-button" onClick={onClose}>&times;</button>
                </div>

                <div className="ship-market-content">
                    {tradeMessage && (
                        <div 
                            className={`trade-message ${tradeMessage.includes('Successfully') ? 'success' : 'error'}`}
                            onClick={() => setTradeMessage(null)}
                        >
                            {tradeMessage}
                        </div>
                    )}

                    {/* Left side - Market ships */}
                    <div className="market-section">
                        <h3>Available Ships</h3>
                        <div className="market-ships">
                            {shipMarket.ships.map((ship, index) => (
                                <div 
                                    key={`market-${ship.name}`} 
                                    className={`ship-card ${selectedMarketShip?.name === ship.name ? 'selected' : ''}`}
                                    onClick={() => handleMarketShipClick(ship)}
                                >
                                    <div className="ship-card-content">
                                        <h4>{ship.name}</h4>
                                        <div className="ship-stats">
                                            <div className="stat-item">
                                                <span className="stat-label">Type:</span>
                                                <span className="stat-value">{ship.specialization}</span>
                                            </div>
                                            <div className="stat-item">
                                                <span className="stat-label">Size:</span>
                                                <span className="stat-value">{ship.size}</span>
                                            </div>
                                            <div className="stat-item">
                                                <span className="stat-label">Engine:</span>
                                                <span className="stat-value">{ship.engine}</span>
                                            </div>
                                            <div className="stat-item">
                                                <span className="stat-label">Price:</span>
                                                <span className="stat-value">{ship.price?.toLocaleString() || 'N/A'} cr</span>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="ship-actions">
                                        <button
                                            className="buy-button"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                handleBuyShip(ship, index);
                                            }}
                                            disabled={!selectedFleet || !ship.price || !player || player.credits < ship.price}
                                        >
                                            Buy
                                        </button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>

                    {/* Right side - Selected ship info and fleet ships */}
                    <div className="right-panel">
                        <div className="selected-ship-info">
                            <h3>Selected Ships</h3>
                            {selectedMarketShip && (
                                <div className="selected-market-ship">
                                    <h4>{selectedMarketShip.name} (Market Ship)</h4>
                                    <div className="ship-stats">
                                        <div className="stat-item">
                                            <span className="stat-label">Type:</span>
                                            <span className="stat-value">{selectedMarketShip.specialization}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Price:</span>
                                            <span className="stat-value">{selectedMarketShip.price?.toLocaleString() || 'N/A'} cr</span>
                                        </div>
                                    </div>
                                </div>
                            )}
                            
                            {selectedFleetShip && (
                                <div className="selected-fleet-ship">
                                    <h4>{selectedFleetShip.name} (Your Ship)</h4>
                                    <div className="ship-stats">
                                        <div className="stat-item">
                                            <span className="stat-label">Type:</span>
                                            <span className="stat-value">{selectedFleetShip.specialization}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Trade Value:</span>
                                            <span className="stat-value">{tradeInValue.toLocaleString()} cr</span>
                                        </div>
                                    </div>
                                </div>
                            )}

                            {selectedMarketShip && selectedFleetShip && (
                                <div className="trade-info">
                                    <div>Ship Cost: {marketShipPrice.toLocaleString()} cr</div>
                                    <div>Trade-in Value: {tradeInValue.toLocaleString()} cr</div>
                                    <div>Final Cost: {finalCost.toLocaleString()} cr</div>
                                    <button
                                        className="buy-button"
                                        onClick={() => handleTradeInShip()}
                                        disabled={!selectedFleet || !canAfford}
                                    >
                                        Complete Trade
                                    </button>
                                </div>
                            )}
                            
                            {!selectedMarketShip && !selectedFleetShip && (
                                <div className="no-selection-message">
                                    Select market and fleet ships to trade
                                </div>
                            )}
                        </div>

                        {selectedFleet && (
                            <div className="fleet-ships">
                                <h3>Your Fleet Ships</h3>
                                {selectedFleetData && selectedFleetData.ships.length > 0 ? (
                                    selectedFleetData.ships.map((ship, index) => (
                                        <div 
                                            key={`fleet-${ship.name}`} 
                                            className={`ship-card ${selectedFleetShip?.name === ship.name ? 'selected' : ''}`}
                                            onClick={() => handleFleetShipClick(ship)}
                                        >
                                            <div className="ship-card-content">
                                                <h4>{ship.name}</h4>
                                                <div className="ship-stats">
                                                    <div className="stat-item">
                                                        <span className="stat-label">Type:</span>
                                                        <span className="stat-value">{ship.specialization}</span>
                                                    </div>
                                                    <div className="stat-item">
                                                        <span className="stat-label">Size:</span>
                                                        <span className="stat-value">{ship.size}</span>
                                                    </div>
                                                    <div className="stat-item">
                                                        <span className="stat-label">Engine:</span>
                                                        <span className="stat-value">{ship.engine}</span>
                                                    </div>
                                                    <div className="stat-item">
                                                        <span className="stat-label">HP:</span>
                                                        <span className="stat-value">{ship.hp}</span>
                                                    </div>
                                                </div>
                                            </div>
                                            <div className="ship-actions">
                                                <button
                                                    className="sell-button"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        handleSellShip(ship, index);
                                                    }}
                                                    disabled={!selectedFleet}
                                                >
                                                    Sell
                                                </button>
                                            </div>
                                        </div>
                                    ))
                                ) : (
                                    <div className="no-selection-message">
                                        No ships in this fleet
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}; 