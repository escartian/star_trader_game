import React, { useState, useEffect } from 'react';
import { Fleet, Resource } from '../types/game';
import { api } from '../services/api';
import './TraderEncounterModal.css';

interface TraderEncounterModalProps {
    fleet: Fleet;
    encounteredFleet: Fleet;
    onClose: () => void;
}

export const TraderEncounterModal: React.FC<TraderEncounterModalProps> = ({
    fleet,
    encounteredFleet,
    onClose
}) => {
    const [selectedResource, setSelectedResource] = useState<string>('');
    const [quantity, setQuantity] = useState<number>(1);
    const [isBuying, setIsBuying] = useState<boolean>(true);
    const [error, setError] = useState<string>('');
    const [success, setSuccess] = useState<string>('');
    const [totalCost, setTotalCost] = useState<number>(0);

    useEffect(() => {
        calculateTotalCost();
    }, [selectedResource, quantity, isBuying]);

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose();
        }
    };

    const handleTrade = async () => {
        try {
            setError('');
            setSuccess('');

            // Parse fleet number based on fleet type
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

            const result = await api.tradeWithTrader(
                fleet.owner_id,
                fleetNumber,
                selectedResource,
                quantity,
                isBuying ? 'buy' : 'sell'
            );

            setSuccess(result);
            setTimeout(() => {
                onClose();
            }, 2000);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to complete trade');
        }
    };

    const getAvailableResources = () => {
        const resources: Resource[] = [];
        encounteredFleet.ships.forEach(ship => {
            ship.cargo.forEach(cargo => {
                if (!resources.find(r => r.resource_type === cargo.resource_type)) {
                    resources.push(cargo);
                }
            });
        });
        return resources;
    };

    const getPlayerResources = () => {
        const resources: Resource[] = [];
        fleet.ships.forEach(ship => {
            ship.cargo.forEach(cargo => {
                if (!resources.find(r => r.resource_type === cargo.resource_type)) {
                    resources.push(cargo);
                }
            });
        });
        return resources;
    };

    const calculateTotalCost = () => {
        if (!selectedResource) {
            setTotalCost(0);
            return;
        }

        const resource = isBuying 
            ? getAvailableResources().find(r => r.resource_type === selectedResource)
            : getPlayerResources().find(r => r.resource_type === selectedResource);

        if (!resource) {
            setTotalCost(0);
            return;
        }

        const price = isBuying ? resource.buy : resource.sell;
        const cost = (price || 0) * quantity;
        setTotalCost(cost);
    };

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="modal-content">
                <div className="modal-header">
                    <h2>Trader Encounter</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    <div className="encounter-info">
                        <h3>You've encountered {encounteredFleet.name}</h3>
                        <div className="fleet-details">
                            <div className="fleet-card">
                                <h4>Your Fleet</h4>
                                <div className="fleet-stats">
                                    <div className="stat-item">
                                        <span className="stat-label">Ships:</span>
                                        <span className="stat-value">{fleet.ships.length}</span>
                                    </div>
                                    <div className="stat-item">
                                        <span className="stat-label">Position:</span>
                                        <span className="stat-value">
                                            ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})
                                        </span>
                                    </div>
                                </div>
                            </div>
                            <div className="fleet-card">
                                <h4>Trader Fleet</h4>
                                <div className="fleet-stats">
                                    <div className="stat-item">
                                        <span className="stat-label">Ships:</span>
                                        <span className="stat-value">{encounteredFleet.ships.length}</span>
                                    </div>
                                    <div className="stat-item">
                                        <span className="stat-label">Position:</span>
                                        <span className="stat-value">
                                            ({encounteredFleet.position.x}, {encounteredFleet.position.y}, {encounteredFleet.position.z})
                                        </span>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div className="trade-controls">
                            <div className="trade-type-selector">
                                <button
                                    className={`trade-type-button ${isBuying ? 'active' : ''}`}
                                    onClick={() => setIsBuying(true)}
                                >
                                    Buy
                                </button>
                                <button
                                    className={`trade-type-button ${!isBuying ? 'active' : ''}`}
                                    onClick={() => setIsBuying(false)}
                                >
                                    Sell
                                </button>
                            </div>
                            <div className="resource-selector">
                                <select
                                    value={selectedResource}
                                    onChange={(e) => setSelectedResource(e.target.value)}
                                >
                                    <option value="">Select Resource</option>
                                    {isBuying
                                        ? getAvailableResources().map(resource => (
                                            <option key={resource.resource_type} value={resource.resource_type}>
                                                {resource.resource_type} - {resource.buy} credits/unit
                                            </option>
                                        ))
                                        : getPlayerResources().map(resource => (
                                            <option key={resource.resource_type} value={resource.resource_type}>
                                                {resource.resource_type} - {resource.sell} credits/unit
                                            </option>
                                        ))
                                    }
                                </select>
                            </div>
                            <div className="quantity-input">
                                <label>Quantity:</label>
                                <input
                                    type="number"
                                    min="1"
                                    value={quantity}
                                    onChange={(e) => setQuantity(Math.max(1, parseInt(e.target.value) || 1))}
                                />
                            </div>
                            <div className="trade-summary">
                                <div className="total-cost">
                                    Total Cost: {totalCost.toFixed(2)} credits
                                </div>
                            </div>
                        </div>
                    </div>
                    {error && <div className="error-message">{error}</div>}
                    {success && <div className="success-message">{success}</div>}
                    <div className="action-buttons">
                        <button
                            className="action-button trade-button"
                            onClick={handleTrade}
                            disabled={!selectedResource || quantity < 1}
                        >
                            {isBuying ? 'Buy' : 'Sell'}
                        </button>
                        <button className="action-button ignore-button" onClick={onClose}>
                            Cancel
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}; 