import React, { useEffect, useRef, useState } from 'react';
import { Fleet, Ship, Shield, Armor, Weapon, ResourceType } from '../types/game';
import { api } from '../services/api';
import './FleetModal.css';

interface FleetModalProps {
    isOpen: boolean;
    onClose: () => void;
    fleet: Fleet;
    onMove: (fleet: Fleet, x: number, y: number, z: number) => Promise<void>;
}

export const FleetModal: React.FC<FleetModalProps> = ({ isOpen, onClose, fleet, onMove }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [targetX, setTargetX] = useState<number>(fleet.position.x);
    const [targetY, setTargetY] = useState<number>(fleet.position.y);
    const [targetZ, setTargetZ] = useState<number>(fleet.position.z);
    const [moveMessage, setMoveMessage] = useState<string>('');
    const [moveStatus, setMoveStatus] = useState<'success' | 'error' | 'info' | null>(null);
    const [encounterFleets, setEncounterFleets] = useState<Fleet[]>([]);
    const [currentEncounterIndex, setCurrentEncounterIndex] = useState(0);
    const [playerName, setPlayerName] = useState<string>('');
    const [error, setError] = useState<string>('');
    const [mapBounds, setMapBounds] = useState<{ min: number; max: number }>({ min: -1000, max: 1000 });

    useEffect(() => {
        const loadSettings = async () => {
            try {
                const settings = await api.getGameSettings();
                setPlayerName(settings.player_name);
                const maxCoord = Math.floor(settings.map_width);
                setMapBounds({ min: -maxCoord, max: maxCoord });
            } catch (err) {
                setError('Failed to load player settings');
            }
        };
        loadSettings();
    }, []);

    useEffect(() => {
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                onClose();
            }
        };

        const handleClickOutside = (event: MouseEvent) => {
            if (modalRef.current && !modalRef.current.contains(event.target as Node)) {
                onClose();
            }
        };

        document.addEventListener('keydown', handleEscape);
        document.addEventListener('mousedown', handleClickOutside);

        return () => {
            document.removeEventListener('keydown', handleEscape);
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, [onClose]);

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose();
        }
    };

    const handleTestEncounter = (type: 'Pirate' | 'Trader' | 'Military' | 'Mercenary') => {
        // Create a test encounter fleet
        const testFleet: Fleet = {
            name: `Fleet_${type}_1`,
            owner_id: type,
            ships: [
                {
                    name: `${type} Ship`,
                    owner: type,
                    specialization: type.toLowerCase() as any,
                    position: fleet.position,
                    cargo: [
                        {
                            resource_type: ResourceType.Minerals,
                            quantity: 100,
                            buy: 50,
                            sell: 40
                        },
                        {
                            resource_type: ResourceType.Luxury,
                            quantity: 50,
                            buy: 100,
                            sell: 80
                        }
                    ],
                    shields: { capacity: 100, current: 100, regen: 5 },
                    weapons: [{ PhotonSingularityBeam: { damage: 50 } }],
                    armor: { capacity: 50, current: 50, regen: 3 },
                    status: "Stationary",
                    hp: 100,
                    combat_state: "NotInCombat",
                    size: "Medium",
                    engine: "Basic"
                }
            ],
            position: fleet.position,
            current_system_id: fleet.current_system_id,
            last_move_distance: null
        };

        // Create encounter response format
        const encounterResponse = {
            status: "encounter",
            message: "Encounter detected during movement",
            encounters: [testFleet],
            current_position: fleet.position,
            target_position: fleet.position,
            remaining_distance: 0
        };

        // Simulate the encounter by directly setting the encounter fleets
        setEncounterFleets([testFleet]);
        setCurrentEncounterIndex(0);
    };

    const handleMove = async () => {
        // Calculate distance
        const dx = targetX - fleet.position.x;
        const dy = targetY - fleet.position.y;
        const dz = targetZ - fleet.position.z;
        const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);

        // Check if within game world bounds
        if (targetX < mapBounds.min || targetX > mapBounds.max || 
            targetY < mapBounds.min || targetY > mapBounds.max || 
            targetZ < mapBounds.min || targetZ > mapBounds.max) {
            setMoveMessage(`Target position is outside the game world bounds (${mapBounds.min} to ${mapBounds.max})`);
            setMoveStatus('error');
            return;
        }

        // Only allow moving player's own fleet
        if (!fleet.owner_id.includes(playerName)) {
            setMoveMessage('You can only move your own fleet');
            setMoveStatus('error');
            return;
        }

        setMoveMessage(`Distance to travel: ${distance.toFixed(2)} units`);
        setMoveStatus('success');
        await onMove(fleet, targetX, targetY, targetZ);
    };

    const calculateDistance = () => {
        const dx = targetX - fleet.position.x;
        const dy = targetY - fleet.position.y;
        const dz = targetZ - fleet.position.z;
        return Math.sqrt(dx * dx + dy * dy + dz * dz);
    };

    // Update distance whenever target coordinates change
    useEffect(() => {
        const distance = calculateDistance();
        setMoveMessage(`Distance to travel: ${distance.toFixed(2)} units`);
        setMoveStatus('info');
    }, [targetX, targetY, targetZ]);

    const renderShield = (shield: Shield) => {
        return `${shield.current}/${shield.capacity} (${shield.regen}/s)`;
    };

    const renderArmor = (armor: Armor) => {
        return `${armor.current}/${armor.capacity} (${armor.regen}/s)`;
    };

    const renderWeapon = (weapons: Weapon[]) => {
        return weapons.map((weapon, index) => {
            const weaponType = Object.keys(weapon)[0];
            const damage = weapon[weaponType as keyof Weapon]?.damage || 0;
            return `${weaponType.replace(/([A-Z])/g, ' $1').trim()} (${damage} DMG)`;
        }).join(', ');
    };

    if (!isOpen) return null;

    const isPlayerFleet = fleet.owner_id === playerName;

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="modal-content">
                <div className="modal-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    {error && <div className="error">{error}</div>}
                    <div className="fleet-info">
                        <div className="fleet-card">
                            <h4>Fleet Details</h4>
                            <div className="fleet-stats">
                                <div className="stat-item">
                                    <span className="stat-label">Owner:</span>
                                    <span className="stat-value">{fleet.owner_id}</span>
                                </div>
                                <div className="stat-item">
                                    <span className="stat-label">Current Position:</span>
                                    <span className="stat-value">
                                        ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})
                                    </span>
                                </div>
                                <div className="stat-item">
                                    <span className="stat-label">Ships:</span>
                                    <span className="stat-value">{fleet.ships.length}</span>
                                </div>
                            </div>
                        </div>
                        
                        {isPlayerFleet && (
                            <div className="movement-section">
                                <h4>Movement Controls</h4>
                                <div className="movement-controls">
                                    <div className="coordinate-input">
                                        <label>Target X ({mapBounds.min} to {mapBounds.max}):</label>
                                        <input
                                            type="number"
                                            min={mapBounds.min}
                                            max={mapBounds.max}
                                            value={targetX}
                                            onChange={(e) => setTargetX(parseInt(e.target.value) || 0)}
                                        />
                                    </div>
                                    <div className="coordinate-input">
                                        <label>Target Y ({mapBounds.min} to {mapBounds.max}):</label>
                                        <input
                                            type="number"
                                            min={mapBounds.min}
                                            max={mapBounds.max}
                                            value={targetY}
                                            onChange={(e) => setTargetY(parseInt(e.target.value) || 0)}
                                        />
                                    </div>
                                    <div className="coordinate-input">
                                        <label>Target Z ({mapBounds.min} to {mapBounds.max}):</label>
                                        <input
                                            type="number"
                                            min={mapBounds.min}
                                            max={mapBounds.max}
                                            value={targetZ}
                                            onChange={(e) => setTargetZ(parseInt(e.target.value) || 0)}
                                        />
                                    </div>
                                    <div className="movement-info">
                                        {moveMessage && (
                                            <div className={`move-message ${moveStatus}`}>
                                                {moveMessage}
                                            </div>
                                        )}
                                    </div>
                                    <button 
                                        className="move-button"
                                        onClick={handleMove}
                                        disabled={!isPlayerFleet}
                                    >
                                        Move Fleet
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>

                    <div className="fleet-card">
                        <h4>Ships</h4>
                        <div className="ship-list">
                            {fleet.ships.map((ship, index) => (
                                <div key={index} className="ship-card">
                                    <h4>{ship.name}</h4>
                                    <div className="ship-stats">
                                        <div className="stat-item">
                                            <span className="stat-label">Type:</span>
                                            <span className="stat-value">{ship.specialization}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Position:</span>
                                            <span className="stat-value">
                                                ({ship.position.x}, {ship.position.y}, {ship.position.z})
                                            </span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Shields:</span>
                                            <span className="stat-value">
                                                {renderShield(ship.shields)}
                                            </span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Weapons:</span>
                                            <span className="stat-value">
                                                {renderWeapon(ship.weapons)}
                                            </span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Armor:</span>
                                            <span className="stat-value">{renderArmor(ship.armor)}</span>
                                        </div>
                                    </div>
                                    {ship.cargo.length > 0 && (
                                        <div className="cargo-list">
                                            {ship.cargo.map((cargo, cargoIndex) => (
                                                <div key={cargoIndex} className="cargo-item">
                                                    <h4>{cargo.resource_type}</h4>
                                                    <div className="cargo-stats">
                                                        <div className="stat-item">
                                                            <span className="stat-label">Quantity:</span>
                                                            <span className="stat-value">
                                                                {cargo.quantity || 0}
                                                            </span>
                                                        </div>
                                                        {cargo.buy && (
                                                            <div className="stat-item">
                                                                <span className="stat-label">Buy Price:</span>
                                                                <span className="stat-value">
                                                                    {cargo.buy} credits
                                                                </span>
                                                            </div>
                                                        )}
                                                        {cargo.sell && (
                                                            <div className="stat-item">
                                                                <span className="stat-label">Sell Price:</span>
                                                                <span className="stat-value">
                                                                    {cargo.sell} credits
                                                                </span>
                                                            </div>
                                                        )}
                                                    </div>
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}; 