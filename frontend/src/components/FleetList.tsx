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
    const [ownerId, setOwnerId] = useState<string>('');
    const [fleetOwners, setFleetOwners] = useState<string[]>([]);

    useEffect(() => {
        console.log('Component mounted, loading fleet owners...');
        loadFleetOwners();
    }, []);

    useEffect(() => {
        if (ownerId) {
            console.log('Owner ID changed, loading fleets for:', ownerId);
            loadFleets();
        }
    }, [ownerId]);

    const loadFleetOwners = async () => {
        try {
            console.log('Fetching fleet owners...');
            const owners = await api.getFleetOwners();
            console.log('Fleet owners received:', owners);
            setFleetOwners(owners);
            if (owners.length > 0) {
                console.log('Setting default owner:', owners[0]);
                setOwnerId(owners[0]); // Set first owner as default
            } else {
                console.log('No fleet owners found');
                setError('No fleet owners found');
            }
        } catch (err) {
            console.error('Error loading fleet owners:', err);
            setError('Failed to load fleet owners');
        }
    };

    const loadFleets = async () => {
        try {
            console.log('Loading fleets for owner:', ownerId);
            setLoading(true);
            const data = await api.getFleets(ownerId);
            console.log('Fleets received:', data);
            setFleets(data);
            setError(null);
        } catch (err) {
            console.error('Error loading fleets:', err);
            setError('Failed to load fleets');
        } finally {
            setLoading(false);
        }
    };

    const handleFleetSelect = async (fleet: Fleet) => {
        try {
            console.log('Selected fleet:', fleet);
            const fleetNumber = parseInt(fleet.name.split('_')[2]);
            console.log('Fetching fleet details for number:', fleetNumber);
            const fleetData = await api.getFleet(ownerId, fleetNumber);
            if (fleetData) {
                console.log('Fleet details received:', fleetData);
                setSelectedFleet(fleetData);
            }
        } catch (err) {
            console.error('Failed to load fleet details:', err);
        }
    };

    const handleCloseModal = () => {
        setSelectedFleet(null);
    };

    if (loading && !ownerId) {
        return <div className="fleet-loading">Loading fleet owners...</div>;
    }

    if (loading && ownerId) {
        return <div className="fleet-loading">Loading fleets for {ownerId}...</div>;
    }

    if (error) {
        return <div className="fleet-error">{error}</div>;
    }

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
                        {fleetOwners.map((owner) => (
                            <option key={owner} value={owner}>
                                {owner}
                            </option>
                        ))}
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
