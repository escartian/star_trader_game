import React, { useEffect, useRef, useState } from 'react';
import { Fleet, Ship, Shield, Armor, Weapon, ResourceType } from '../types/game';
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
                } catch (err) {
                    console.error('Error loading star system:', err);
                }
            } else {
                setCurrentSystemName('');
            }
        };
        loadSystemName();
    }, [currentFleet?.current_system_id]);

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
            targetZ: targetZ
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

            console.log('Sending move request:', {
                owner_id,
                fleet_number,
                target: { x: targetX, y: targetY, z: targetZ }
            });

            const response = await api.moveFleet(owner_id, fleet_number, targetX, targetY, targetZ);
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
                if (onMove) {
                    onMove(currentFleet, targetX, targetY, targetZ, false);
                }
            } else if (data.status === 'transition_entry') {
                setMoveMessage(`Fleet entered System ${data.current_system_id}`);
                setMoveStatus('info');
                if (onMove) {
                    onMove(currentFleet, targetX, targetY, targetZ, false);
                }
            } else if (data.status === 'transition_exit') {
                setMoveMessage('Fleet exited the star system');
                setMoveStatus('info');
                if (onMove) {
                    onMove(currentFleet, targetX, targetY, targetZ, false);
                }
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
                                    {currentFleet?.position ? 
                                        `(${currentFleet.position.x.toFixed(2)}, ${currentFleet.position.y.toFixed(2)}, ${currentFleet.position.z.toFixed(2)})` : 
                                        'Unknown'
                                    }
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
                                    {currentFleet.current_system_id !== null && currentFleet.current_system_id !== undefined && (
                                        <div className="system-bounds-info">
                                            <p>
                                                <strong>System Movement Guide:</strong>
                                                <br />
                                                • You are currently in {currentSystemName || `Star System ${currentFleet.current_system_id}`}
                                                <br />
                                                • To move within the system, use coordinates between {mapBounds.min} and {mapBounds.max}
                                                <br />
                                                • To exit the system, set coordinates beyond these bounds
                                                <br />
                                                • When exiting, your fleet will transition to the galaxy map near this system's location
                                            </p>
                                        </div>
                                    )}
                                    <div className="movement-controls">
                                        <div className="coordinate-input">
                                            <label>Target X {currentFleet.current_system_id !== null ? 
                                                `(System bounds: ${mapBounds.min} to ${mapBounds.max}, exceed to exit)` : 
                                                `(${mapBounds.min} to ${mapBounds.max})`}:
                                            </label>
                                            <input
                                                type="number"
                                                value={targetX}
                                                onChange={(e) => setTargetX(Number(e.target.value))}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <div className="coordinate-input">
                                            <label>Target Y {currentFleet.current_system_id !== null ? 
                                                `(System bounds: ${mapBounds.min} to ${mapBounds.max}, exceed to exit)` : 
                                                `(${mapBounds.min} to ${mapBounds.max})`}:
                                            </label>
                                            <input
                                                type="number"
                                                value={targetY}
                                                onChange={(e) => setTargetY(Number(e.target.value))}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <div className="coordinate-input">
                                            <label>Target Z {currentFleet.current_system_id !== null ? 
                                                `(System bounds: ${mapBounds.min} to ${mapBounds.max}, exceed to exit)` : 
                                                `(${mapBounds.min} to ${mapBounds.max})`}:
                                            </label>
                                            <input
                                                type="number"
                                                value={targetZ}
                                                onChange={(e) => setTargetZ(Number(e.target.value))}
                                                disabled={isLoading}
                                            />
                                        </div>
                                        <button 
                                            className="move-button" 
                                            onClick={handleMove}
                                            disabled={isLoading}
                                        >
                                            {currentFleet.current_system_id !== null && 
                                             (Math.abs(targetX) > mapBounds.max || 
                                              Math.abs(targetY) > mapBounds.max || 
                                              Math.abs(targetZ) > mapBounds.max) 
                                                ? "Exit System" 
                                                : "Move Fleet"}
                                        </button>
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