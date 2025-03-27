import React, { useState, useEffect } from 'react';
import { Fleet, Ship } from '../types/game';
import { api } from '../services/api';
import { FleetModal } from './FleetModal';
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

    const handleCloseModal = () => {
        setSelectedFleet(null);
    };

    if (loading) return <div className="fleet-loading">Loading fleets...</div>;
    if (error) return <div className="fleet-error">{error}</div>;

    return (
        <div className="fleet-container">
            <div className="fleet-header">
                <h2>Fleet Management</h2>
            </div>
            <div className="search-controls">
                <div className="fleet-controls">
                    <select
                        className="faction-select"
                        value={ownerId}
                        onChange={(e) => setOwnerId(e.target.value)}
                    >
                        <option value="Igor">Igor</option>
                        <option value="The Galactic Empire">Galactic Empire</option>
                        <option value="The Rebel Alliance">Rebel Alliance</option>
                        <option value="The Trade Federation">Trade Federation</option>
                    </select>
                </div>
            </div>
            <div className="fleet-grid">
                {fleets.map((fleet) => (
                    <div key={fleet.name} className="fleet-card">
                        <h3>{fleet.name}</h3>
                        <div className="fleet-info">
                            <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                            <p>Ships: {fleet.ships.length}</p>
                            <p>Owner: {fleet.owner_id}</p>
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
                <FleetModal 
                    fleet={selectedFleet}
                    onClose={handleCloseModal}
                />
            )}
        </div>
    );
};
