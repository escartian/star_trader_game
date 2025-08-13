import React, { useEffect, useRef, useState } from 'react';
import { Fleet, Ship, Shield, Armor, Weapon, ResourceType, StarSystem, Position } from '../types/game';
import { api } from '../services/api';
import './FleetModal.css';

interface FleetModalProps {
    isOpen: boolean;
    onClose: () => void;
    fleet: Fleet;
    onMove: (fleet: Fleet, x: number, y: number, z: number, local: boolean) => Promise<void>;
}

export const FleetModal: React.FC<FleetModalProps> = ({ isOpen, onClose, fleet, onMove }) => {
    // Simplified initial log
    console.log('Opening fleet modal:', {
        name: fleet?.name,
        position: fleet?.position,
        systemId: fleet?.current_system_id,
        shipCount: fleet?.ships?.length
    });

    const modalRef = useRef<HTMLDivElement>(null);
    const [currentFleet, setCurrentFleet] = useState<Fleet>(fleet);
    const [targetX, setTargetX] = useState<number>(fleet?.position?.x || 0);
    const [targetY, setTargetY] = useState<number>(fleet?.position?.y || 0);
    const [targetZ, setTargetZ] = useState<number>(fleet?.position?.z || 0);
    const [moveMessage, setMoveMessage] = useState<string>('');
    const [moveStatus, setMoveStatus] = useState<'success' | 'error' | 'info' | null>(null);
    const [playerName, setPlayerName] = useState<string>('');
    const [error, setError] = useState<string>('');
    const [mapBounds, setMapBounds] = useState<{ min: number; max: number }>({ min: -1000, max: 1000 });
    const [currentSystemName, setCurrentSystemName] = useState<string>('');
    const [currentSystemCenter, setCurrentSystemCenter] = useState<Position | null>(null);
    const [allSystems, setAllSystems] = useState<StarSystem[]>([]);
    const [moveSpace, setMoveSpace] = useState<'galaxy' | 'system'>(fleet?.current_system_id !== null && fleet?.current_system_id !== undefined ? 'system' : 'galaxy');
    const [targetSystemId, setTargetSystemId] = useState<number | undefined>(fleet?.current_system_id ?? undefined);
    const [isLoading, setIsLoading] = useState(true);

    // Add effect to clear move message after delay
    useEffect(() => {
        if (moveMessage) {
            const timer = setTimeout(() => {
                setMoveMessage('');
                setMoveStatus(null);
            }, 3000); // Clear after 3 seconds
            return () => clearTimeout(timer);
        }
    }, [moveMessage]);

    useEffect(() => {
        const loadSettings = async () => {
            try {
                setIsLoading(true);
                const settings = await api.getGameSettings();
                setPlayerName(settings.player_name);
                const maxCoord = Math.floor(settings.map_width);
                setMapBounds({ min: -maxCoord, max: maxCoord });
                console.log('Loaded map bounds:', { min: -maxCoord, max: maxCoord });
                // Load systems for dropdown
                const systems = await api.getGalaxyMap();
                setAllSystems(systems);
            } catch (err) {
                console.error('Failed to load player settings:', err);
                setError('Failed to load player settings');
            } finally {
                setIsLoading(false);
            }
        };
        loadSettings();
    }, []);

    useEffect(() => {
        const loadSystemName = async () => {
            if (currentFleet?.current_system_id !== null && currentFleet?.current_system_id !== undefined) {
                try {
                    const system = await api.getStarSystem(currentFleet.current_system_id);
                    setCurrentSystemName(system.star.name);
                    setCurrentSystemCenter(system.position);
                } catch (err) {
                    console.error('Error loading star system:', err);
                    setCurrentSystemCenter(null);
                }
            } else {
                setCurrentSystemName('');
                setCurrentSystemCenter(null);
            }
        };
        loadSystemName();
    }, [currentFleet?.current_system_id]);

    // Keep inputs in sync with move mode and current fleet location
    useEffect(() => {
        if (!currentFleet) return;
        if (moveSpace === 'system') {
            if (currentFleet.local_position) {
                setTargetX(currentFleet.local_position.x);
                setTargetY(currentFleet.local_position.y);
                setTargetZ(currentFleet.local_position.z);
            } else if (currentSystemCenter) {
                setTargetX(currentFleet.position.x - currentSystemCenter.x);
                setTargetY(currentFleet.position.y - currentSystemCenter.y);
                setTargetZ(currentFleet.position.z - currentSystemCenter.z);
            }
        } else {
            setTargetX(currentFleet.position.x);
            setTargetY(currentFleet.position.y);
            setTargetZ(currentFleet.position.z);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [moveSpace, currentFleet, currentSystemCenter]);

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

    // Update currentFleet when fleet prop changes
    useEffect(() => {
        if (fleet) {
            console.log('Fleet updated:', {
                name: fleet.name,
                position: fleet.position,
                systemId: fleet.current_system_id
            });
            setCurrentFleet(fleet);
            if (fleet.position) {
                setTargetX(fleet.position.x);
                setTargetY(fleet.position.y);
                setTargetZ(fleet.position.z);
            }
        }
    }, [fleet]);

    const handleMove = async () => {
        console.log('=== Move Button Clicked ===');
        console.log('Button state:', {
            currentFleet: !!currentFleet,
            targetX: targetX,
            targetY: targetY,
            targetZ: targetZ,
            mapBounds
        });

        if (!currentFleet || targetX === undefined || targetY === undefined || targetZ === undefined) {
            console.log('Move prevented - missing required data:', {
                currentFleet: !currentFleet,
                targetX: targetX === undefined,
                targetY: targetY === undefined,
                targetZ: targetZ === undefined
            });
            return;
        }

        // Do not block deep-space exits on the client. Backend interprets direction and exits system.

        console.log('=== Starting Fleet Movement ===');
        console.log('Fleet:', {
            name: currentFleet.name,
            currentPosition: currentFleet.position,
            systemId: currentFleet.current_system_id
        });
        console.log('Target Position:', { x: targetX, y: targetY, z: targetZ });

        try {
            // Extract owner_id and fleet_number from fleet name
            const parts = currentFleet.name.split('_');
            const owner_id = encodeURIComponent(parts[1]);
            const fleet_number = parseInt(parts[2]);

            // Compute target coordinates depending on mode
            let sendX = targetX;
            let sendY = targetY;
            let sendZ = targetZ;
            let sendSystemId: number | undefined = undefined;

            if (moveSpace === 'system') {
                // Resolve target system: dropdown uses system.id, otherwise use current_system_id (id or index)
                const resolvedIdOrIdx =
                    targetSystemId !== undefined ? targetSystemId : currentFleet.current_system_id ?? undefined;

                if (resolvedIdOrIdx === undefined) {
                    setMoveMessage('Select a target system or move in Galaxy mode');
                    setMoveStatus('error');
                    return;
                }

                // Find system by id first, then fallback to treating the value as an index
                const system = allSystems.find(s => s.id === resolvedIdOrIdx) ?? allSystems[resolvedIdOrIdx];
                if (!system) {
                    setMoveMessage(`Target system ${resolvedIdOrIdx} not found`);
                    setMoveStatus('error');
                    return;
                }
                // Clamp local inputs to system cube bounds ±map_width
                const clampedLocalX = Math.max(mapBounds.min, Math.min(mapBounds.max, targetX));
                const clampedLocalY = Math.max(mapBounds.min, Math.min(mapBounds.max, targetY));
                const clampedLocalZ = Math.max(mapBounds.min, Math.min(mapBounds.max, targetZ));
                // Convert local (in-system) coords → galaxy coords
                sendX = system.position.x + clampedLocalX;
                sendY = system.position.y + clampedLocalY;
                sendZ = system.position.z + clampedLocalZ;
                // Always send stable system id
                sendSystemId = system.id;
            }

            console.log('Sending move request:', {
                owner_id,
                fleet_number,
                target: { x: sendX, y: sendY, z: sendZ },
                space: moveSpace,
                system_id: sendSystemId,
            });

            const response = await api.moveFleet(owner_id, fleet_number, {
                x: sendX,
                y: sendY,
                z: sendZ,
                space: moveSpace,
                system_id: sendSystemId,
            });
            console.log('Move response:', response);
            
            const responseData = JSON.parse(response);
            console.log('Parsed move response:', responseData);
            
            if (!responseData.success) {
                setMoveMessage(responseData.message || 'Move failed');
                setMoveStatus('error');
                return;
            }

            const data = responseData.data;
            console.log('Move data:', data);
            
            if (data.status === 'success') {
                setMoveMessage('Fleet moved successfully');
                setMoveStatus('success');
                // Update local state using server-reported galaxy position
                setCurrentFleet((prev) => ({
                    ...prev,
                    position: data.current_position,
                    current_system_id: data.current_system_id,
                    local_position: data.local_current_position ?? prev.local_position,
                }));
            } else if (data.status === 'transition_entry') {
                setMoveMessage(`Fleet entered System ${data.current_system_id}`);
                setMoveStatus('info');
                setCurrentFleet((prev) => ({
                    ...prev,
                    position: data.current_position,
                    current_system_id: data.current_system_id,
                    local_position: data.local_current_position ?? prev.local_position,
                }));
            } else if (data.status === 'transition_exit') {
                setMoveMessage('Fleet exited the star system');
                setMoveStatus('info');
                setCurrentFleet((prev) => ({
                    ...prev,
                    position: data.current_position,
                    current_system_id: data.current_system_id,
                    local_position: data.local_current_position ?? prev.local_position,
                }));
            } else {
                console.error('Unexpected move response status:', data.status);
                setMoveMessage(data.message || 'Move failed');
                setMoveStatus('error');
            }
        } catch (error) {
            console.error('Move failed:', error);
            const errorMessage = error instanceof Error ? error.message : 'Move failed';
            setMoveMessage(errorMessage);
            setMoveStatus('error');
        }
    };

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

    const isPlayerFleet = currentFleet.owner_id === playerName;

    return (
        <div className="fleet-modal-container" onClick={handleOverlayClick}>
            <div className="fleet-modal-content" ref={modalRef}>
                <div className="modal-header">
                    <h2>{currentFleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    {error && <div className="error">{error}</div>}
                    
                    <div className="fleet-card">
                        <div className="fleet-stats">
                            <div className="stat-item">
                                <span className="stat-label">Owner:</span>
                                <span className="stat-value">{currentFleet.owner_id}</span>
                            </div>
                            <div className="stat-item">
                                <span className="stat-label">Current Position:</span>
                                <span className="stat-value">
                                    {currentFleet?.position ? (
                                            currentFleet.current_system_id !== null && currentFleet.current_system_id !== undefined && currentSystemCenter ? (
                                                <>
                                                    <span>{`Gal: (${currentSystemCenter.x}, ${currentSystemCenter.y}, ${currentSystemCenter.z})`}</span>
                                                    {currentFleet.local_position && (
                                                        <>
                                                            <br />
                                                            <span>{`Local: (${currentFleet.local_position.x}, ${currentFleet.local_position.y}, ${currentFleet.local_position.z})`}</span>
                                                        </>
                                                    )}
                                                </>
                                            ) : (
                                            <span>{`Gal: (${currentFleet.position.x}, ${currentFleet.position.y}, ${currentFleet.position.z})`}</span>
                                        )
                                    ) : 'Unknown'}
                                </span>
                            </div>
                            <div className="stat-item">
                                <span className="stat-label">Location:</span>
                                <span className="stat-value">
                                    {currentFleet.current_system_id !== null && currentFleet.current_system_id !== undefined ? 
                                        `In ${currentSystemName || 'Star System ' + currentFleet.current_system_id}` : 'Deep Space'}
                                </span>
                            </div>
                            <div className="stat-item">
                                <span className="stat-label">Ships:</span>
                                <span className="stat-value">{currentFleet.ships.length}</span>
                            </div>
                        </div>
                    </div>
                    
                    {isPlayerFleet && (
                        <div className="movement-section">
                            {isLoading ? (
                                <div className="loading-message">Loading map settings...</div>
                            ) : (
                                <>
                                    <div className="movement-mode" style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8 }}>
                                        <label>
                                            Move Mode:
                                            <select value={moveSpace} onChange={(e) => setMoveSpace(e.target.value as 'galaxy' | 'system')}>
                                                <option value="galaxy">Galaxy (deep space)</option>
                                                <option value="system">Within system</option>
                                            </select>
                                        </label>
                                        {moveSpace === 'system' && (
                                            <label>
                                                Target System:
                                                <select
                                                    value={targetSystemId ?? ''}
                                                    onChange={(e) => setTargetSystemId(e.target.value ? Number(e.target.value) : undefined)}
                                                >
                                                    <option value="">Select system</option>
                                                    {allSystems.map((s, idx) => (
                                                        <option key={idx} value={idx}>
                                                            {s.star.name} (#{idx})
                                                        </option>
                                                    ))}
                                                </select>
                                            </label>
                                        )}
                                    </div>
                                    {currentFleet.current_system_id !== null && currentFleet.current_system_id !== undefined && (
                                        <div className="system-bounds-info">
                                            <p>
                                                <strong>Movement Guide:</strong>
                                                <br />
                                                • You are currently in {currentSystemName || `Star System ${currentFleet.current_system_id}`}.
                                                <br />
                                                • Use <em>Move Mode = Within system</em> to move to in-system coordinates (shown below).
                                                <br />
                                                • Use <em>Move Mode = Galaxy (deep space)</em> to leave this system or travel between systems using galaxy coordinates.
                                                <br />
                                                • Selecting a <em>Target System</em> in Within‑system mode will auto-travel to that system first (deep space), then apply the in‑system move.
                                            </p>
                                        </div>
                                    )}
                                    <div className="movement-controls">
                                        <div className="coordinate-input">
                                            <label>Target X {moveSpace === 'system' ? `(in-system coords)` : `(galaxy coords)`}:</label>
                                            <input
                                                type="number"
                                                value={targetX}
                                                min={mapBounds.min}
                                                max={mapBounds.max}
                                                onChange={(e) => {
                                                    const v = Number(e.target.value);
                                                    setTargetX(Math.max(mapBounds.min, Math.min(mapBounds.max, v)));
                                                }}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <div className="coordinate-input">
                                            <label>Target Y {moveSpace === 'system' ? `(in-system coords)` : `(galaxy coords)`}:</label>
                                            <input
                                                type="number"
                                                value={targetY}
                                                min={mapBounds.min}
                                                max={mapBounds.max}
                                                onChange={(e) => {
                                                    const v = Number(e.target.value);
                                                    setTargetY(Math.max(mapBounds.min, Math.min(mapBounds.max, v)));
                                                }}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <div className="coordinate-input">
                                            <label>Target Z {moveSpace === 'system' ? `(in-system coords)` : `(galaxy coords)`}:</label>
                                            <input
                                                type="number"
                                                value={targetZ}
                                                min={mapBounds.min}
                                                max={mapBounds.max}
                                                onChange={(e) => {
                                                    const v = Number(e.target.value);
                                                    setTargetZ(Math.max(mapBounds.min, Math.min(mapBounds.max, v)));
                                                }}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <button 
                                            className="move-button" 
                                            onClick={handleMove}
                                            disabled={isLoading}
                                        >
                                            {moveSpace === 'galaxy' ? 'Move Fleet (Galaxy)' : 'Move Fleet (System)'}
                                        </button>
                                        {moveSpace === 'galaxy' && (
                                            <div className="hint">{`Galaxy bounds: ${mapBounds.min}-${mapBounds.max}`}</div>
                                        )}
                                        {moveMessage && (
                                            <div className={`move-message ${moveStatus}`}>
                                                {moveMessage}
                                            </div>
                                        )}
                                    </div>
                                </>
                            )}
                        </div>
                    )}

                    <div className="ships-section">
                        <div className="ships-list">
                            {currentFleet.ships.map((ship, index) => (
                                <div key={index} className="ship-card">
                                    <h5>{ship.name}</h5>
                                    <div className="ship-stats">
                                        <div className="stat-item">
                                            <span className="stat-label">Type:</span>
                                            <span className="stat-value">{ship.specialization}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">HP:</span>
                                            <span className="stat-value">{ship.hp}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Shields:</span>
                                            <span className="stat-value">{renderShield(ship.shields)}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Armor:</span>
                                            <span className="stat-value">{renderArmor(ship.armor)}</span>
                                        </div>
                                        <div className="stat-item">
                                            <span className="stat-label">Weapons:</span>
                                            <span className="stat-value">{renderWeapon(ship.weapons)}</span>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}; 