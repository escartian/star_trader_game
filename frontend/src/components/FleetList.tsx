import React, { useState, useEffect } from 'react';
import { Fleet, Ship } from '../types/game';
import { api } from '../services/api';
import { FleetModal } from './FleetModal';
import { CombatModal } from './CombatModal';
import { EncounterModal } from './EncounterModal';
import './FleetList.css';

export const FleetList: React.FC = () => {
    const [fleets, setFleets] = useState<Fleet[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedFleet, setSelectedFleet] = useState<Fleet | null>(null);
    const [ownerId, setOwnerId] = useState<string>('');
    const [fleetOwners, setFleetOwners] = useState<string[]>([]);
    const [combatAttacker, setCombatAttacker] = useState<Fleet | null>(null);
    const [combatDefender, setCombatDefender] = useState<Fleet | null>(null);
    const [encounterFleets, setEncounterFleets] = useState<Fleet[]>([]);
    const [currentEncounterIndex, setCurrentEncounterIndex] = useState<number>(0);

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
            // Handle different fleet naming schemes
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
            console.log('Fetching fleet details for number:', fleetNumber);
            const fleetData = await api.getFleet(fleet.owner_id, fleetNumber);
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

    const handleCloseCombatModal = () => {
        setCombatAttacker(null);
        setCombatDefender(null);
        loadFleets(); // Refresh fleets after combat
    };

    const handleCloseEncounterModal = () => {
        if (currentEncounterIndex < encounterFleets.length - 1) {
            setCurrentEncounterIndex(prev => prev + 1);
        } else {
            // Clear encounters and reset state
            setEncounterFleets([]);
            setCurrentEncounterIndex(0);
            // Refresh fleets to get updated state
            loadFleets();
        }
    };

    const handleAttackClick = (fleet: Fleet) => {
        setCombatAttacker(fleet);
    };

    const handleDefenderSelect = (fleet: Fleet) => {
        setCombatDefender(fleet);
    };

    const handleCombatFromEncounter = async (attacker: Fleet, defender: Fleet) => {
        try {
            // Parse fleet numbers based on fleet type
            let attackerNumber: number;
            let defenderNumber: number;

            if (attacker.name.startsWith('Pirate_Fleet_')) {
                attackerNumber = parseInt(attacker.name.split('Pirate_Fleet_')[1]);
            } else if (attacker.name.startsWith('Trader_Fleet_')) {
                attackerNumber = parseInt(attacker.name.split('Trader_Fleet_')[1]);
            } else if (attacker.name.startsWith('Military_Fleet_')) {
                attackerNumber = parseInt(attacker.name.split('Military_Fleet_')[1]);
            } else if (attacker.name.startsWith('Mercenary_Fleet_')) {
                attackerNumber = parseInt(attacker.name.split('Mercenary_Fleet_')[1]);
            } else {
                attackerNumber = parseInt(attacker.name.split('Fleet_')[1].split('_')[1]);
            }

            if (defender.name.startsWith('Pirate_Fleet_')) {
                defenderNumber = parseInt(defender.name.split('Pirate_Fleet_')[1]);
            } else if (defender.name.startsWith('Trader_Fleet_')) {
                defenderNumber = parseInt(defender.name.split('Trader_Fleet_')[1]);
            } else if (defender.name.startsWith('Military_Fleet_')) {
                defenderNumber = parseInt(defender.name.split('Military_Fleet_')[1]);
            } else if (defender.name.startsWith('Mercenary_Fleet_')) {
                defenderNumber = parseInt(defender.name.split('Mercenary_Fleet_')[1]);
            } else {
                defenderNumber = parseInt(defender.name.split('Fleet_')[1].split('_')[1]);
            }
            
            // Initiate combat through the API
            const result = await api.initiateCombat(
                attacker.owner_id,
                attackerNumber,
                defender.owner_id,
                defenderNumber
            );
            
            // After combat, refresh the fleets
            loadFleets();
            
            // If there are more encounters, show the next one
            if (currentEncounterIndex < encounterFleets.length - 1) {
                setCurrentEncounterIndex(prev => prev + 1);
            } else {
                // Clear encounters and reset state
                setEncounterFleets([]);
                setCurrentEncounterIndex(0);
                // Refresh fleets again to get final state
                loadFleets();
            }
            
            // Close the combat modal
            setCombatAttacker(null);
            setCombatDefender(null);
        } catch (err) {
            console.error('Error handling combat encounter:', err);
        }
    };

    const handleMoveFleet = async (fleet: Fleet, x: number, y: number, z: number) => {
        try {
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

            const response = await api.moveFleet(fleet.owner_id, fleetNumber, x, y, z);
            
            // Parse the response to check for encounters
            try {
                const responseData = JSON.parse(response);
                if (responseData.encounters && responseData.encounters.length > 0) {
                    // If we have encounters, show them immediately
                    setEncounterFleets(responseData.encounters);
                    setCurrentEncounterIndex(0);
                    // Set the selected fleet to ensure the encounter modal has the correct context
                    setSelectedFleet(fleet);
                    // Close the fleet modal to show the encounter modal
                    handleCloseModal();
                }
            } catch (e) {
                // If the response is not JSON, it's just a success message
                console.log('No encounters found');
            }
            
            // Update the fleet data
            const updatedFleet = await api.getFleet(fleet.owner_id, fleetNumber);
            if (updatedFleet) {
                setFleets(prevFleets => 
                    prevFleets.map(f => 
                        f.name === fleet.name ? updatedFleet : f
                    )
                );
                // Only update selected fleet if we don't have encounters
                if (encounterFleets.length === 0) {
                    setSelectedFleet(updatedFleet);
                }
            }
            
            // Refresh the full fleet list
            loadFleets();
        } catch (err) {
            console.error('Error moving fleet:', err);
        }
    };

    const handleEncounter = (encounters: Fleet[]) => {
        setEncounterFleets(encounters);
        setCurrentEncounterIndex(0);
    };

    const formatFleetName = (fleetName: string): string => {
        if (fleetName.startsWith('Fleet_Pirate_')) {
            const num = fleetName.split('Fleet_Pirate_')[1];
            return `Pirate Fleet #${num}`;
        } else if (fleetName.startsWith('Fleet_Trader_')) {
            const num = fleetName.split('Fleet_Trader_')[1];
            return `Trader Fleet #${num}`;
        } else if (fleetName.startsWith('Fleet_Military_')) {
            const num = fleetName.split('Fleet_Military_')[1];
            return `Military Fleet #${num}`;
        } else if (fleetName.startsWith('Fleet_Mercenary_')) {
            const num = fleetName.split('Fleet_Mercenary_')[1];
            return `Mercenary Fleet #${num}`;
        } else {
            const parts = fleetName.split('Fleet_')[1].split('_');
            return `${parts[0]} Fleet #${parts[1]}`;
        }
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
                    <div 
                        key={fleet.name} 
                        className="fleet-card"
                        onClick={() => handleFleetSelect(fleet)}
                    >
                        <h3>{formatFleetName(fleet.name)}</h3>
                        <div className="fleet-info">
                            <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                            <p>Ships: {fleet.ships.length}</p>
                            <p>Owner: {fleet.owner_id}</p>
                        </div>
                    </div>
                ))}
            </div>

            {selectedFleet && (
                <FleetModal 
                    fleet={selectedFleet}
                    onClose={handleCloseModal}
                    onMove={handleMoveFleet}
                />
            )}

            {combatAttacker && combatDefender && (
                <CombatModal
                    attacker={combatAttacker}
                    defender={combatDefender}
                    onClose={handleCloseCombatModal}
                />
            )}

            {encounterFleets.length > 0 && selectedFleet && (
                <EncounterModal
                    fleet={selectedFleet}
                    encounteredFleet={encounterFleets[currentEncounterIndex]}
                    onClose={handleCloseEncounterModal}
                    onCombat={handleCombatFromEncounter}
                />
            )}
        </div>
    );
};
