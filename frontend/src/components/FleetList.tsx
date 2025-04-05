import React, { useState, useEffect } from 'react';
import { Fleet, Ship } from '../types/game';
import { api } from '../services/api';
import { FleetModal } from './FleetModal';
import { CombatModal } from './CombatModal';
import { EncounterModal } from './EncounterModal';
import { TraderEncounterModal } from './TraderEncounterModal';
import './FleetList.css';

export const FleetList: React.FC = () => {
    const [fleets, setFleets] = useState<Fleet[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedFleet, setSelectedFleet] = useState<Fleet | null>(null);
    const [selectedOwner, setSelectedOwner] = useState<string>('');
    const [owners, setOwners] = useState<string[]>([]);
    const [combatAttacker, setCombatAttacker] = useState<Fleet | null>(null);
    const [combatDefender, setCombatDefender] = useState<Fleet | null>(null);
    const [encounterFleets, setEncounterFleets] = useState<Fleet[]>([]);
    const [currentEncounterIndex, setCurrentEncounterIndex] = useState<number>(0);
    const [targetPosition, setTargetPosition] = useState<{ x: number; y: number; z: number } | null>(null);

    const loadFleets = async () => {
        try {
            setLoading(true);
            setError(null);
            const settings = await api.getGameSettings();
            const fleetsData = await api.getFleets(settings.player_name);
            console.log('Loaded fleets:', fleetsData);
            setFleets(fleetsData);
        } catch (err) {
            console.error('Failed to load fleets:', err);
            setError('Failed to load fleets');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        const loadData = async () => {
            try {
                setLoading(true);
                setError(null);
                const settings = await api.getGameSettings();
                const [ownersData, fleetsData] = await Promise.all([
                    api.getFleetOwners(),
                    api.getFleets(settings.player_name)
                ]);
                console.log('Loaded owners:', ownersData);
                console.log('Loaded fleets:', fleetsData);
                
                // Sort owners to put player first
                const sortedOwners = ownersData.sort((a, b) => {
                    if (a === settings.player_name) return -1;
                    if (b === settings.player_name) return 1;
                    return a.localeCompare(b);
                });
                
                setOwners(sortedOwners);
                setFleets(fleetsData);
                
                // Set player as default selected owner
                if (sortedOwners.includes(settings.player_name)) {
                    setSelectedOwner(settings.player_name);
                }
            } catch (err) {
                console.error('Failed to load fleet data:', err);
                setError('Failed to load fleet data');
            } finally {
                setLoading(false);
            }
        };
        loadData();
    }, []);

    useEffect(() => {
        if (selectedOwner) {
            const loadFleets = async () => {
                try {
                    setLoading(true);
                    setError(null);
                    const fleetsData = await api.getFleets(selectedOwner);
                    console.log('Loaded fleets for owner:', selectedOwner, fleetsData);
                    setFleets(fleetsData);
                } catch (err) {
                    console.error('Failed to load fleets:', err);
                    setError('Failed to load fleets');
                } finally {
                    setLoading(false);
                }
            };
            loadFleets();
        }
    }, [selectedOwner]);

    useEffect(() => {
        if (encounterFleets.length > 0 && selectedFleet) {
            console.log('Encounter detected, displaying modal');
            // No need to force a re-render, just ensure the index is set
            setCurrentEncounterIndex(0);
        }
    }, [encounterFleets, selectedFleet]);

    const handleFleetSelect = async (fleet: Fleet) => {
        try {
            console.log('Selected fleet:', fleet);
            // Handle different fleet naming schemes
            let fleetNumber: number;
            let fleetOwnerId = fleet.owner_id;

            // Parse fleet number from the name
            const parts = fleet.name.split('_');
            if (parts.length >= 3) {
                fleetNumber = parseInt(parts[parts.length - 1]);
            } else {
                fleetNumber = parseInt(fleet.name.split('_').pop() || '0');
            }
            
            console.log('Parsed fleet info:', { fleetNumber, fleetOwnerId });
            
            // For special fleet types, use the type as owner ID
            if (fleet.owner_id === 'Pirate' || fleet.owner_id === 'Trader' || 
                fleet.owner_id === 'Military' || fleet.owner_id === 'Mercenary') {
                fleetOwnerId = fleet.owner_id;
            }

            console.log('Fetching fleet details for:', { fleetNumber, fleetOwnerId });
            const fleetData = await api.getFleet(fleetOwnerId, fleetNumber);
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

    const handleCloseEncounterModal = async () => {
        console.log('Closing encounter modal:', {
            currentIndex: currentEncounterIndex,
            totalEncounters: encounterFleets.length
        });

        if (currentEncounterIndex < encounterFleets.length - 1) {
            console.log('Showing next encounter...');
            setCurrentEncounterIndex(prev => prev + 1);
        } else {
            console.log('No more encounters, completing move...');
            // Clear encounters and reset state
            setEncounterFleets([]);
            setCurrentEncounterIndex(0);
            
            // If we have a target position, continue the move
            if (targetPosition) {
                console.log('Continuing move to target position:', targetPosition);
                const { x, y, z } = targetPosition;
                if (selectedFleet) {
                    // Recursively call handleMoveFleet to continue the path
                    await handleMoveFleet(selectedFleet, x, y, z);
                }
            }
            
            // Clear target position
            setTargetPosition(null);
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
            console.log('Starting combat from encounter:', {
                attacker: attacker.name,
                defender: defender.name
            });

            // Parse fleet numbers based on fleet type
            let attackerNumber: number;
            let defenderNumber: number;
            let defenderOwnerId: string;

            // For the attacker (player's fleet), use the selected fleet's number
            if (selectedFleet) {
                console.log('Selected fleet for attacker:', selectedFleet);
                if (selectedFleet.name.startsWith('Fleet_Pirate_')) {
                    attackerNumber = parseInt(selectedFleet.name.split('Fleet_Pirate_')[1]);
                } else if (selectedFleet.name.startsWith('Fleet_Trader_')) {
                    attackerNumber = parseInt(selectedFleet.name.split('Fleet_Trader_')[1]);
                } else if (selectedFleet.name.startsWith('Fleet_Military_')) {
                    attackerNumber = parseInt(selectedFleet.name.split('Fleet_Military_')[1]);
                } else if (selectedFleet.name.startsWith('Fleet_Mercenary_')) {
                    attackerNumber = parseInt(selectedFleet.name.split('Fleet_Mercenary_')[1]);
                } else {
                    attackerNumber = parseInt(selectedFleet.name.split('Fleet_')[1].split('_')[1]);
                }
                console.log('Parsed attacker number:', attackerNumber);
            } else {
                console.error('No selected fleet for combat');
                return;
            }

            // For the defender (encountered fleet), determine the owner ID and number
            console.log('Processing defender fleet:', defender);
            if (defender.name.startsWith('Fleet_Pirate_')) {
                defenderNumber = parseInt(defender.name.split('Fleet_Pirate_')[1]);
                defenderOwnerId = "Pirate";
            } else if (defender.name.startsWith('Fleet_Trader_')) {
                defenderNumber = parseInt(defender.name.split('Fleet_Trader_')[1]);
                defenderOwnerId = "Trader";
            } else if (defender.name.startsWith('Fleet_Military_')) {
                defenderNumber = parseInt(defender.name.split('Fleet_Military_')[1]);
                defenderOwnerId = "Military";
            } else if (defender.name.startsWith('Fleet_Mercenary_')) {
                defenderNumber = parseInt(defender.name.split('Fleet_Mercenary_')[1]);
                defenderOwnerId = "Mercenary";
            } else {
                defenderNumber = parseInt(defender.name.split('Fleet_')[1].split('_')[1]);
                defenderOwnerId = defender.owner_id;
            }
            console.log('Parsed defender info:', { defenderNumber, defenderOwnerId });

            // Validate numbers before making the API call
            if (isNaN(attackerNumber) || isNaN(defenderNumber)) {
                console.error('Invalid fleet numbers:', { attackerNumber, defenderNumber });
                return;
            }

            console.log('Initiating combat with:', {
                attacker: { ownerId: selectedFleet.owner_id, number: attackerNumber },
                defender: { ownerId: defenderOwnerId, number: defenderNumber }
            });
            
            // Initiate combat through the API
            console.log('Calling initiateCombat API...');
            const result = await api.initiateCombat(
                selectedFleet.owner_id,
                attackerNumber,
                defenderOwnerId,
                defenderNumber
            );
            
            console.log('Combat result:', result);
            
            // After combat, refresh the fleets
            console.log('Refreshing fleets after combat...');
            loadFleets();
            
            // If there are more encounters, show the next one
            if (currentEncounterIndex < encounterFleets.length - 1) {
                console.log('More encounters available, showing next one...');
                setCurrentEncounterIndex(prev => prev + 1);
            } else {
                console.log('No more encounters, clearing state...');
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

    const handleMoveFleet = async (fleet: Fleet, targetX: number, targetY: number, targetZ: number) => {
        console.log('Starting fleet move:', { fleet: fleet.name, owner: fleet.owner_id, targetPosition: { x: targetX, y: targetY, z: targetZ } });
        
        try {
            // Parse fleet number based on fleet type
            let fleetNumber: number;
            let fleetOwnerId = fleet.owner_id;

            // Handle special fleet types
            if (fleet.owner_id === 'Pirate' || fleet.owner_id === 'Trader' || 
                fleet.owner_id === 'Military' || fleet.owner_id === 'Mercenary') {
                fleetNumber = parseInt(fleet.name.split('_').pop() || '0');
                fleetOwnerId = fleet.owner_id;
            } else {
                // Regular player fleet
                fleetNumber = parseInt(fleet.name.split('_').pop() || '0');
            }

            // Set the selected fleet before making the API call
            setSelectedFleet(fleet);

            console.log('Parsed fleet info:', { fleetNumber, fleetOwnerId });

            // Call moveFleet API
            console.log('Calling moveFleet API...');
            const response = await api.moveFleet(fleetOwnerId, fleetNumber, targetX, targetY, targetZ);
            console.log('Move fleet API response:', response);

            try {
                // Try to parse encounters from response
                const responseData = response && typeof response === 'string' && response !== 'Fleet moved successfully' 
                    ? JSON.parse(response) 
                    : null;
                
                if (responseData?.status === "encounter") {
                    console.log('Encounters found:', responseData.encounters);
                    setEncounterFleets(responseData.encounters);
                    
                    // Update fleet position to current position from response
                    const updatedFleet = {
                        ...fleet,
                        position: responseData.current_position,
                        last_move_distance: responseData.remaining_distance
                    };
                    setFleets(prevFleets => 
                        prevFleets.map(f => f.name === fleet.name ? updatedFleet : f)
                    );
                    setSelectedFleet(updatedFleet);
                    
                    // Store target position for after encounters
                    setTargetPosition(responseData.target_position);
                    return;
                }
            } catch (parseError) {
                console.log('No encounters in response:', parseError);
            }

            // If we get here, either there were no encounters or the response was just a success message
            console.log('No encounters, proceeding with move...');
            
            // Update the fleet's position
            const updatedFleet = await api.getFleet(fleetOwnerId, fleetNumber);
            if (updatedFleet) {
                console.log('Fleet updated successfully:', updatedFleet);
                setFleets(prevFleets => 
                    prevFleets.map(f => f.name === fleet.name ? updatedFleet : f)
                );
                setSelectedFleet(updatedFleet);
            }
            
            // Refresh fleets
            loadFleets();
            
        } catch (error) {
            console.error('Error moving fleet:', error);
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

    if (loading && !selectedOwner) {
        return <div className="fleet-loading">Loading fleet owners...</div>;
    }

    if (loading && selectedOwner) {
        return <div className="fleet-loading">Loading fleets for {selectedOwner}...</div>;
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
                        value={selectedOwner}
                        onChange={(e) => setSelectedOwner(e.target.value)}
                        disabled={loading}
                    >
                        {owners.map((owner) => (
                            <option key={owner} value={owner}>
                                {owner}
                            </option>
                        ))}
                    </select>
                </div>
            </div>
            
            {loading ? (
                <div className="fleet-loading">Loading fleets...</div>
            ) : error ? (
                <div className="fleet-error">{error}</div>
            ) : fleets.length === 0 ? (
                <div className="fleet-empty">No fleets found</div>
            ) : (
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
            )}

            {selectedFleet && (
                <FleetModal 
                    isOpen={true}
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
