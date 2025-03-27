import React, { useState } from 'react';
import { Fleet, Resource } from '../types/game';
import { api } from '../services/api';
import './EncounterModal.css';

interface EncounterModalProps {
    fleet: Fleet;
    encounteredFleet: Fleet;
    onClose: () => void;
    onCombat: (attacker: Fleet, defender: Fleet) => void;
}

interface CargoItem {
    resource_type: string;
    quantity: number;
    buy: number | null;
    sell: number | null;
}

export const EncounterModal: React.FC<EncounterModalProps> = ({
    fleet,
    encounteredFleet,
    onClose,
    onCombat
}) => {
    const [selectedResource, setSelectedResource] = useState<string | null>(null);
    const [quantity, setQuantity] = useState<number>(1);
    const [error, setError] = useState<string | null>(null);
    const [success, setSuccess] = useState<string | null>(null);
    const [isSearching, setIsSearching] = useState(false);
    const [searchResults, setSearchResults] = useState<CargoItem[]>([]);
    const [tradeType, setTradeType] = useState<'buy' | 'sell' | null>(null);

    const handleTrade = async () => {
        if (!selectedResource || quantity <= 0 || !tradeType) {
            setError('Please select a resource, enter a valid quantity, and choose trade type');
            return;
        }

        try {
            // Parse fleet numbers
            let fleetNumber: number;
            if (fleet.name.startsWith('Fleet_Pirate_')) {
                fleetNumber = parseInt(fleet.name.split('Fleet_Pirate_')[1]);
            } else if (fleet.name.startsWith('Fleet_Trader_')) {
                fleetNumber = parseInt(fleet.name.split('Fleet_Trader_')[1]);
            } else if (fleet.name.startsWith('Fleet_Military_')) {
                fleetNumber = parseInt(fleet.name.split('Fleet_Military_')[1]);
            } else if (fleet.name.startsWith('Fleet_Mercenary_')) {
                fleetNumber = parseInt(fleet.name.split('Fleet_Mercenary_')[1]);
            } else {
                fleetNumber = parseInt(fleet.name.split('Fleet_')[1].split('_')[1]);
            }

            // Call the appropriate API based on trade type
            const result = await api.tradeWithTrader(
                fleet.owner_id,
                fleetNumber,
                selectedResource,
                quantity,
                tradeType
            );

            if (result === "Success") {
                setSuccess('Trade successful!');
                setError(null);
                onClose();
            } else {
                setError(result);
                setSuccess(null);
            }
        } catch (err) {
            console.error('Error during trade:', err);
            setError('Failed to complete trade');
            setSuccess(null);
        }
    };

    const handleAttack = () => {
        onCombat(fleet, encounteredFleet);
    };

    const handleSearch = () => {
        setIsSearching(true);
        // Get all cargo from the encountered fleet's ships
        const cargo = encounteredFleet.ships.reduce((acc: CargoItem[], ship) => {
            ship.cargo.forEach(item => {
                const existingItem = acc.find(i => i.resource_type === item.resource_type);
                if (existingItem) {
                    existingItem.quantity += item.quantity || 0;
                } else {
                    acc.push({
                        resource_type: item.resource_type,
                        quantity: item.quantity || 0,
                        buy: item.buy || null,
                        sell: item.sell || null
                    });
                }
            });
            return acc;
        }, []);
        setSearchResults(cargo);
    };

    const handleDiplomacy = async () => {
        try {
            // TODO: Implement diplomacy actions
            setSuccess('Diplomatic relations established');
            onClose();
        } catch (err) {
            console.error('Error during diplomacy:', err);
            setError('Failed to establish diplomatic relations');
        }
    };

    // Get all cargo from the encountered fleet's ships with buy/sell prices
    const encounteredCargo = encounteredFleet.ships.reduce((acc: CargoItem[], ship) => {
        ship.cargo.forEach(item => {
            const existingItem = acc.find(i => i.resource_type === item.resource_type);
            if (existingItem) {
                existingItem.quantity += item.quantity || 0;
            } else {
                acc.push({
                    resource_type: item.resource_type,
                    quantity: item.quantity || 0,
                    buy: item.buy || null,
                    sell: item.sell || null
                });
            }
        });
        return acc;
    }, []);

    const formatCredits = (amount: number | null) => {
        if (amount === null) return 'N/A';
        return `${amount.toFixed(2)} credits`;
    };

    const renderEncounterContent = () => {
        switch (encounteredFleet.owner_id) {
            case "Trader":
                return (
                    <>
                        <div className="trader-cargo">
                            <h3>Trader's Market</h3>
                            <div className="market-table">
                                <div className="market-header">
                                    <div className="market-column">Resource</div>
                                    <div className="market-column">Available</div>
                                    <div className="market-column">Buy Price</div>
                                    <div className="market-column">Sell Price</div>
                                    <div className="market-column">Actions</div>
                                </div>
                                {encounteredCargo.map((item, index) => (
                                    <div key={index} className="market-row">
                                        <div className="market-column">{item.resource_type}</div>
                                        <div className="market-column">{item.quantity}</div>
                                        <div className="market-column">{formatCredits(item.buy)}</div>
                                        <div className="market-column">{formatCredits(item.sell)}</div>
                                        <div className="market-column">
                                            <div className="trade-buttons">
                                                <button 
                                                    className="buy-button"
                                                    onClick={() => {
                                                        setSelectedResource(item.resource_type);
                                                        setQuantity(1);
                                                        setTradeType('buy');
                                                    }}
                                                >
                                                    Buy
                                                </button>
                                                <button 
                                                    className="sell-button"
                                                    onClick={() => {
                                                        setSelectedResource(item.resource_type);
                                                        setQuantity(1);
                                                        setTradeType('sell');
                                                    }}
                                                >
                                                    Sell
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>

                        {selectedResource && (
                            <div className="trade-controls">
                                <h3>Trade Details</h3>
                                <div className="trade-form">
                                    <div className="input-group">
                                        <label>Resource:</label>
                                        <span className="resource-name">{selectedResource}</span>
                                    </div>
                                    <div className="input-group">
                                        <label>Type:</label>
                                        <span className="trade-type">{tradeType === 'buy' ? 'Buying' : 'Selling'}</span>
                                    </div>
                                    <div className="input-group">
                                        <label>Quantity:</label>
                                        <input
                                            type="number"
                                            min="1"
                                            value={quantity}
                                            onChange={(e) => setQuantity(parseInt(e.target.value) || 1)}
                                        />
                                    </div>
                                    {error && <div className="error-message">{error}</div>}
                                    {success && <div className="success-message">{success}</div>}
                                    <div className="trade-actions">
                                        <button className="confirm-button" onClick={handleTrade}>
                                            Confirm {tradeType === 'buy' ? 'Purchase' : 'Sale'}
                                        </button>
                                        <button className="cancel-button" onClick={() => {
                                            setSelectedResource(null);
                                            setTradeType(null);
                                        }}>
                                            Cancel
                                        </button>
                                    </div>
                                </div>
                            </div>
                        )}
                    </>
                );
            case "Military":
                return (
                    <>
                        <div className="military-options">
                            <h3>Military Fleet Options</h3>
                            <div className="option-buttons">
                                <button className="search-button" onClick={handleSearch}>
                                    Search Ships
                                </button>
                                <button className="diplomacy-button" onClick={handleDiplomacy}>
                                    Diplomatic Relations
                                </button>
                            </div>
                        </div>

                        {isSearching && (
                            <div className="search-results">
                                <h3>Search Results</h3>
                                <div className="cargo-grid">
                                    {searchResults.map((item, index) => (
                                        <div key={index} className="cargo-item">
                                            <h4>{item.resource_type}</h4>
                                            <p className="quantity">Quantity: {item.quantity}</p>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}
                    </>
                );
            case "Pirate":
                return (
                    <div className="pirate-warning">
                        <h3>⚠️ Pirate Fleet Detected</h3>
                        <p>This fleet appears to be a pirate fleet. Exercise caution!</p>
                    </div>
                );
            case "Mercenary":
                return (
                    <div className="mercenary-info">
                        <h3>Mercenary Fleet</h3>
                        <p>This fleet is available for hire. Would you like to negotiate a contract?</p>
                    </div>
                );
            default:
                return (
                    <div className="unknown-fleet">
                        <h3>Unknown Fleet Type</h3>
                        <p>This fleet's intentions are unclear.</p>
                    </div>
                );
        }
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content encounter-modal">
                <div className="modal-header">
                    <h2>Fleet Encounter</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    <div className="encounter-info">
                        <h3>Encountered Fleet: {encounteredFleet.name}</h3>
                        <p>Position: ({encounteredFleet.position.x}, {encounteredFleet.position.y}, {encounteredFleet.position.z})</p>
                        <p>Number of Ships: {encounteredFleet.ships.length}</p>
                    </div>

                    {renderEncounterContent()}

                    <div className="action-buttons">
                        <button className="ignore-button" onClick={onClose}>
                            Ignore
                        </button>
                        <button className="attack-button" onClick={handleAttack}>
                            Attack Fleet
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}; 