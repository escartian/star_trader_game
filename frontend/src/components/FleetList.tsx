import React, { useState, useEffect } from 'react';
import { Fleet, Ship, MoveFleetResponse } from '../types/game';
import { api } from '../services/api';
import { FleetModal } from './FleetModal';
import { CombatModal } from './CombatModal';
import { EncounterModal } from './EncounterModal';
import { TraderEncounterModal } from './TraderEncounterModal';
import './FleetList.css';

interface FleetListProps {
    onFleetSelected?: (fleet: Fleet | null) => void;
}

export const FleetList: React.FC<FleetListProps> = ({ onFleetSelected }) => {
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
        if (!selectedOwner) {
            console.log('No owner selected, cannot load fleets.');
            setFleets([]); // Clear fleets if no owner is selected
            setLoading(false);
            return;
        }
        try {
            setLoading(true);
            setError(null);
            // Pass the selectedOwner to getPlayerFleets
            const fleetsResponse = await api.getPlayerFleets(selectedOwner); 
            if (fleetsResponse.success && fleetsResponse.data) {
                 console.log(`Loaded ${fleetsResponse.data.length} fleets for ${selectedOwner}:`, fleetsResponse.data);
                 setFleets(fleetsResponse.data);
            } else {
                 console.error(`Failed to load fleets for ${selectedOwner}:`, fleetsResponse.message);
                 setError(`Failed to load fleets: ${fleetsResponse.message}`);
                 setFleets([]); // Clear fleets on error
            }
           
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
                if (settings && settings.player_name) {
                    setSelectedOwner(settings.player_name); // Default to player
                    // Now load fleets for the player
                     const fleetsResponse = await api.getPlayerFleets(settings.player_name);
                     if (fleetsResponse.success && fleetsResponse.data) {
                         console.log('Initial fleets loaded:', fleetsResponse.data);
                         setFleets(fleetsResponse.data);
                     } else {
                         setError(`Initial fleet load failed: ${fleetsResponse.message}`);
                         setFleets([]);
                     }
                } else {
                     setError('Failed to get player name from settings.');
                }
                // Fetch owners after getting settings
                const ownersData = await api.getFleetOwners();
                console.log('Loaded owners:', ownersData);
                setOwners(ownersData);

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
            console.log('=== Fleet Selection Debug ===');
            console.log('Initial fleet data:', JSON.stringify(fleet, null, 2));
            console.log('Has position:', !!fleet.position);
            console.log('Has ships:', !!fleet.ships);
            console.log('Fleet type:', fleet.owner_id);
            
            // Use the fleet directly if it has all required properties
            if (fleet.position && fleet.ships) {
                console.log('Using existing fleet data - all properties present');
                setSelectedFleet(fleet);
                onFleetSelected?.(fleet);
                return;
            }

            console.log('Missing some properties, fetching detailed fleet data');
            // Only fetch fleet details if we're missing required properties
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
                console.log('Special fleet type detected:', fleetOwnerId);
            }

            console.log('Fetching fleet details for:', { fleetNumber, fleetOwnerId });
            const fleetData = await api.getFleet(fleetOwnerId, fleetNumber);
            if (fleetData) {
                console.log('Received fleet data:', JSON.stringify(fleetData, null, 2));
                console.log('Has position:', !!fleetData.position);
                console.log('Has ships:', !!fleetData.ships);
                setSelectedFleet(fleetData);
                onFleetSelected?.(fleetData);
            } else {
                console.error('No fleet data received from API');
            }
        } catch (err) {
            console.error('Failed to load fleet details:', err);
        }
    };

    // Add logging to useEffect that handles selectedFleet changes
    useEffect(() => {
        if (selectedFleet) {
            console.log('=== Selected Fleet Updated ===');
            console.log('Current selected fleet:', JSON.stringify(selectedFleet, null, 2));
            console.log('Has position:', !!selectedFleet.position);
            console.log('Has ships:', !!selectedFleet.ships);
        }
    }, [selectedFleet]);

    const handleCloseModal = () => {
        // Close the local modal but keep the globally selected fleet
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
                    await handleMoveFleet(selectedFleet, x, y, z, false);
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

    const handleMoveFleet = async (fleet: Fleet, targetX: number, targetY: number, targetZ: number, isLocalMove: boolean) => {
        console.log('=== FleetList: Starting Fleet Movement ===');
        console.log('Fleet:', {
            name: fleet.name,
            currentPosition: fleet.position,
            systemId: fleet.current_system_id
        });
        console.log('Target Position:', { x: targetX, y: targetY, z: targetZ });

        try {
            // Extract owner_id and fleet_number from fleet name
            const parts = fleet.name.split('_');
            const owner_id = encodeURIComponent(parts[1]); // Encode the owner_id
            const fleet_number = parseInt(parts[2]);

            // Build move intent using the same API as the modal
            const payload: any = { x: targetX, y: targetY, z: targetZ };
            if (isLocalMove) {
                payload.space = 'system';
                // Use stable system id; if missing, treat as galaxy move
                if (fleet.current_system_id !== null && fleet.current_system_id !== undefined) {
                    payload.system_id = fleet.current_system_id;
                }
            } else {
                payload.space = 'galaxy';
            }

            const responseText = await api.moveFleet(owner_id, fleet_number, payload);
            const parsed = JSON.parse(responseText);
            console.log('Move API Response Data:', parsed);
            if (!parsed.success) {
                throw new Error(parsed.message || 'Failed to move fleet');
            }

            const resp = parsed.data;
            const updatedFleet = {
                ...fleet,
                position: resp.current_position,
                current_system_id: resp.current_system_id,
                local_position: resp.local_current_position ?? fleet.local_position,
            } as Fleet;

            console.log('Updated Fleet after move:', updatedFleet);
            setFleets(fleets.map(f => f.name === fleet.name ? updatedFleet : f));
            if (selectedFleet && selectedFleet.name === fleet.name) {
                setSelectedFleet(updatedFleet);
            }
            console.log('Fleet list and selected fleet updated successfully');
        } catch (error) {
            console.error('Error moving fleet:', error);
            throw error;
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
                                <p>Position: {fleet.position ? 
                                    `(${fleet.position.x}, ${fleet.position.y}, ${fleet.position.z})` : 
                                    'Unknown'}</p>
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

export default FleetList;
