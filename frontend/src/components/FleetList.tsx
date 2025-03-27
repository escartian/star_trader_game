import React, { useState, useEffect } from 'react';
import { Fleet, Ship } from '../types/game';
import { api } from '../services/api';
import './FleetList.css';

export const FleetList: React.FC = () => {
    const [fleets, setFleets] = useState<Fleet[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedFleet, setSelectedFleet] = useState<Fleet | null>(null);
    const [ownerId, setOwnerId] = useState('Igor');

    useEffect(() => {
        loadFleets();
    }, [ownerId]);

    const loadFleets = async () => {
        try {
            setLoading(true);
            const data = await api.getFleets(ownerId);
            setFleets(data);
            setError(null);
        } catch (err) {
            setError('Failed to load fleets');
            console.error(err);
        } finally {
            setLoading(false);
        }
    };

    const handleFleetSelect = async (fleet: Fleet) => {
        try {
            const fleetNumber = parseInt(fleet.name.split('_')[2]);
            const fleetData = await api.getFleet(ownerId, fleetNumber);
            if (fleetData) {
                setSelectedFleet(fleetData);
            }
        } catch (err) {
            console.error('Failed to load fleet details:', err);
        }
    };

    const handleOwnerChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setOwnerId(e.target.value);
    };

    if (loading) return <div className="fleet-loading">Loading fleets...</div>;
    if (error) return <div className="fleet-error">{error}</div>;

    return (
        <div className="fleet-container">
            <div className="fleet-header">
                <h2>Fleet Management</h2>
                <div className="owner-input">
                    <label htmlFor="ownerId">Owner/Faction:</label>
                    <input
                        type="text"
                        id="ownerId"
                        value={ownerId}
                        onChange={handleOwnerChange}
                        placeholder="Enter owner or faction name"
                    />
                </div>
            </div>
            <div className="fleet-grid">
                {fleets.map((fleet) => (
                    <div key={fleet.name} className="fleet-card">
                        <h3>{fleet.name}</h3>
                        <div className="fleet-info">
                            <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                            <p>Ships: {fleet.ships.length}</p>
                            <p>Current System: {fleet.current_system_id || 'In Transit'}</p>
                        </div>
                        <button 
                            className="view-details-btn"
                            onClick={() => handleFleetSelect(fleet)}
                        >
                            View Details
                        </button>
                    </div>
                ))}
            </div>

            {selectedFleet && (
                <div className="fleet-details-modal">
                    <div className="fleet-details-content">
                        <h2>{selectedFleet.name}</h2>
                        <div className="fleet-summary">
                            <p>Position: ({selectedFleet.position.x}, {selectedFleet.position.y}, {selectedFleet.position.z})</p>
                            <p>Current System: {selectedFleet.current_system_id || 'In Transit'}</p>
                        </div>
                        <div className="ships-grid">
                            {selectedFleet.ships.map((ship) => (
                                <div key={ship.name} className="ship-card">
                                    <h3>{ship.name}</h3>
                                    <div className="ship-stats">
                                        <div className="stat-group">
                                            <h4>Basic Info</h4>
                                            <p>Type: {ship.specialization}</p>
                                            <p>Size: {ship.size}</p>
                                            <p>Engine: {ship.engine}</p>
                                            <p>HP: {ship.hp}</p>
                                            <p>Status: {ship.status}</p>
                                            <p>Combat State: {ship.combat_state}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h4>Shields</h4>
                                            <p>Capacity: {ship.shields.capacity}</p>
                                            <p>Current: {ship.shields.current}</p>
                                            <p>Regen: {ship.shields.regen}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h4>Armor</h4>
                                            <p>Capacity: {ship.armor.capacity}</p>
                                            <p>Current: {ship.armor.current}</p>
                                            <p>Regen: {ship.armor.regen}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h4>Weapons</h4>
                                            <ul>
                                                {ship.weapons.map((weapon, i) => (
                                                    <li key={i}>{weapon.name} (Damage: {weapon.damage})</li>
                                                ))}
                                            </ul>
                                        </div>
                                        <div className="stat-group">
                                            <h4>Cargo</h4>
                                            <ul>
                                                {ship.cargo.map((resource, i) => (
                                                    <li key={i}>
                                                        {resource.resource_type}: {resource.quantity || 0}
                                                    </li>
                                                ))}
                                            </ul>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                        <button 
                            className="close-details-btn"
                            onClick={() => setSelectedFleet(null)}
                        >
                            Close
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}; 