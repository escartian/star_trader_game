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

        const resources = isBuying ? getAvailableResources() : getPlayerResources();
        const resource = resources.find(r => r.resource_type === selectedResource);
        if (resource) {
            const price = isBuying ? resource.buy : resource.sell;
            setTotalCost(price ? price * quantity : 0);
        } else {
            setTotalCost(0);
        }
    };

    const handleResourceSelect = (resource: Resource, buying: boolean) => {
        setSelectedResource(resource.resource_type);
        setIsBuying(buying);
        setQuantity(1);
        calculateTotalCost();
    };

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="trader-encounter">
                <div className="modal-header">
                    <h2>Trading with {encounteredFleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    <div className="trade-instructions">
                        <p>Click "Buy" or "Sell" next to a resource to start trading. Prices are shown in credits per unit.</p>
                    </div>
                    <div className="trader-layout">
                        <div className="trader-section">
                            <div className="trader-cargo">
                                <h3>Available Resources</h3>
                                <div className="market-table-container">
                                    <table className="market-table">
                                        <thead>
                                            <tr>
                                                <th>Resource</th>
                                                <th>Available</th>
                                                <th>Buy Price</th>
                                                <th>Sell Price</th>
                                                <th>Actions</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {getAvailableResources().map((item, index) => (
                                                <tr 
                                                    key={index}
                                                    className={selectedResource === item.resource_type ? 'selected' : ''}
                                                >
                                                    <td>{item.resource_type}</td>
                                                    <td>{item.quantity}</td>
                                                    <td className="buy-price">
                                                        {item.buy ? `${(item.buy * quantity).toFixed(2)} cr (${item.buy} per unit)` : 'N/A'}
                                                    </td>
                                                    <td className="sell-price">
                                                        {item.sell ? `${(item.sell * quantity).toFixed(2)} cr (${item.sell} per unit)` : 'N/A'}
                                                    </td>
                                                    <td>
                                                        <div className="trade-actions">
                                                            <button 
                                                                className={`buy-button ${selectedResource === item.resource_type && isBuying ? 'selected' : ''}`}
                                                                onClick={() => handleResourceSelect(item, true)}
                                                                disabled={!item.buy}
                                                            >
                                                                Buy
                                                            </button>
                                                            <button 
                                                                className={`sell-button ${selectedResource === item.resource_type && !isBuying ? 'selected' : ''}`}
                                                                onClick={() => handleResourceSelect(item, false)}
                                                                disabled={!item.sell}
                                                            >
                                                                Sell
                                                            </button>
                                                        </div>
                                                    </td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                        <div className="player-section">
                            <div className="player-info">
                                <h3>Your Cargo</h3>
                                <div className="player-resources">
                                    <table className="resources-table">
                                        <thead>
                                            <tr>
                                                <th>Resource</th>
                                                <th>Amount</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {getPlayerResources().map((resource, index) => (
                                                <tr 
                                                    key={index}
                                                    className={selectedResource === resource.resource_type ? 'selected' : ''}
                                                >
                                                    <td>{resource.resource_type}</td>
                                                    <td>{resource.quantity}</td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                            {selectedResource && (
                                <div className="trade-controls">
                                    <div className="trade-amount">
                                        <label>Quantity:</label>
                                        <input
                                            type="number"
                                            min="1"
                                            value={quantity}
                                            onChange={(e) => {
                                                const newAmount = Math.max(1, parseInt(e.target.value) || 1);
                                                setQuantity(newAmount);
                                            }}
                                        />
                                    </div>
                                    <h3>{isBuying ? 'Buy' : 'Sell'} {selectedResource}</h3>
                                    <div className="trade-summary">
                                        <div className="total-cost">
                                            Total: {totalCost.toFixed(2)} credits
                                        </div>
                                    </div>
                                    <div className="action-buttons">
                                        <button
                                            className={`action-button trade-button ${isBuying ? 'buy-button' : 'sell-button'}`}
                                            onClick={handleTrade}
                                            disabled={!selectedResource || quantity < 1}
                                        >
                                            {isBuying ? 'Confirm Purchase' : 'Confirm Sale'}
                                        </button>
                                        <button className="action-button ignore-button" onClick={onClose}>
                                            Leave Trader
                                        </button>
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                    {error && <div className="error-message">{error}</div>}
                    {success && <div className="success-message">{success}</div>}
                </div>
            </div>
        </div>
    );
}; 