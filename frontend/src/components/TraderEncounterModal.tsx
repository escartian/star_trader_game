import React, { useState } from 'react';
import { Fleet, Resource } from '../types/game';
import { api } from '../services/api';
import './TraderEncounterModal.css';

interface TraderEncounterModalProps {
    fleet: Fleet;
    encounteredFleet: Fleet;
    onClose: () => void;
    onCombat: (attacker: Fleet, defender: Fleet) => void;
}

interface CargoItem {
    resource_type: string;
    quantity: number;
}

export const TraderEncounterModal: React.FC<TraderEncounterModalProps> = ({
    fleet,
    encounteredFleet,
    onClose,
    onCombat
}) => {
    const [selectedResource, setSelectedResource] = useState<string>('');
    const [quantity, setQuantity] = useState<number>(1);
    const [success, setSuccess] = useState<string>('');
    const [error, setError] = useState<string>('');
    const [isBuying, setIsBuying] = useState<boolean>(true);

    const handleTrade = async () => {
        if (!selectedResource || quantity <= 0) {
            setError('Please select a resource and enter a valid quantity');
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

            // Determine trade type based on whether we're buying or selling
            const tradeType = isBuying ? 'buy' : 'sell';

            // Call the trade API
            const result = await api.tradeWithTrader(
                fleet.owner_id,
                fleetNumber,
                selectedResource,
                quantity,
                tradeType
            );

            if (result === "Success") {
                setSuccess('Trade successful!');
                setError('');
                setSelectedResource('');
                setQuantity(1);
                // Refresh the fleet data to show updated cargo
                const updatedFleet = await api.getFleet(fleet.owner_id, fleetNumber);
                if (updatedFleet) {
                    // Update the fleet in the parent component
                    onClose();
                }
            } else {
                setError(result);
                setSuccess('');
            }
        } catch (err) {
            console.error('Error during trade:', err);
            setError('Failed to complete trade');
            setSuccess('');
        }
    };

    const handleAttack = () => {
        onCombat(fleet, encounteredFleet);
    };

    // Get all cargo from the trader's ships and combine quantities
    const traderCargo = encounteredFleet.ships.reduce((acc: CargoItem[], ship) => {
        ship.cargo.forEach(item => {
            const existingItem = acc.find(i => i.resource_type === item.resource_type);
            if (existingItem) {
                existingItem.quantity += item.quantity || 0;
            } else {
                acc.push({
                    resource_type: item.resource_type,
                    quantity: item.quantity || 0
                });
            }
        });
        return acc;
    }, []);

    return (
        <div className="modal-overlay">
            <div className="modal-content trader-encounter">
                <div className="modal-header">
                    <h2>Trade Opportunity</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    <div className="encounter-info">
                        <h3>Trader Fleet: {encounteredFleet.name}</h3>
                        <p>Position: ({encounteredFleet.position.x}, {encounteredFleet.position.y}, {encounteredFleet.position.z})</p>
                        <p>Number of Ships: {encounteredFleet.ships.length}</p>
                    </div>

                    <div className="trader-cargo">
                        <h3>Available Resources</h3>
                        <div className="cargo-grid">
                            {traderCargo.map((item, index) => (
                                <div 
                                    key={index} 
                                    className={`cargo-item ${selectedResource === item.resource_type ? 'selected' : ''}`}
                                    onClick={() => setSelectedResource(item.resource_type)}
                                >
                                    <h4>{item.resource_type}</h4>
                                    <p className="quantity">Available: {item.quantity}</p>
                                </div>
                            ))}
                        </div>
                    </div>

                    {selectedResource && (
                        <div className="trade-controls">
                            <h3>Trade Details</h3>
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
                            <button className="trade-button" onClick={handleTrade}>
                                Trade
                            </button>
                        </div>
                    )}

                    <div className="action-buttons">
                        <button className="ignore-button" onClick={onClose}>
                            Ignore
                        </button>
                        <button className="attack-button" onClick={handleAttack}>
                            Attack Trader
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}; 