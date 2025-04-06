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
    console.log('=== FleetModal Render Debug ===');
    console.log('Fleet data received:', JSON.stringify(fleet, null, 2));
    console.log('Has position:', !!fleet?.position);
    console.log('Has ships:', !!fleet?.ships);
    console.log('Fleet type:', fleet?.owner_id);

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

    const handleMove = async () => {
        // Check if we have position data
        if (!currentFleet.position) {
            setMoveMessage('Cannot move fleet: position data is missing');
            setMoveStatus('error');
            return;
        }

        // Calculate distance
        const dx = targetX - currentFleet.position.x;
        const dy = targetY - currentFleet.position.y;
        const dz = targetZ - currentFleet.position.z;
        const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);

        // Check if within game world bounds
        if (targetX < mapBounds.min || targetX > mapBounds.max || 
            targetY < mapBounds.min || targetY > mapBounds.max || 
            targetZ < mapBounds.min || targetZ > mapBounds.max) {
            setMoveMessage(`Target position is outside the game world bounds (${mapBounds.min} to ${mapBounds.max})`);
            setMoveStatus('error');
            return;
        }

        try {
            // This is a move (either in system or deep space)
            const isLocalMove = !!currentFleet.current_system_id;
            setMoveMessage(`Moving ${isLocalMove ? 'within star system' : 'through deep space'} (${distance.toFixed(2)} units)`);
            setMoveStatus('info');
            
            // Call the move function
            await onMove(currentFleet, targetX, targetY, targetZ, isLocalMove);
            
            // Update the current fleet's position
            const updatedFleet = {
                ...currentFleet,
                position: { x: targetX, y: targetY, z: targetZ }
            };
            setCurrentFleet(updatedFleet);
            
            setMoveMessage('Move completed successfully');
            setMoveStatus('success');

            // Fetch updated fleet data
            try {
                const fleetNumber = parseInt(currentFleet.name.split('_').pop() || '0');
                const fleetData = await api.getFleet(currentFleet.owner_id, fleetNumber);
                if (fleetData) {
                    console.log('Updated fleet data after move:', fleetData);
                    setCurrentFleet(fleetData);
                }
            } catch (err) {
                console.error('Failed to fetch updated fleet data:', err);
            }
        } catch (err) {
            console.error('Move failed:', err);
            setMoveMessage('Failed to move fleet');
            setMoveStatus('error');
        }
    };

    // Update currentFleet when fleet prop changes
    useEffect(() => {
        console.log('Fleet prop changed, updating currentFleet');
        if (fleet) {
            console.log('New fleet data:', JSON.stringify(fleet, null, 2));
            setCurrentFleet(fleet);
            if (fleet.position) {
                setTargetX(fleet.position.x);
                setTargetY(fleet.position.y);
                setTargetZ(fleet.position.z);
            }
        }
    }, [fleet]);

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
                                    {currentFleet.current_system_id ? `In Star System ${currentFleet.current_system_id}` : 'Deep Space'}
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
                            <div className="movement-controls">
                                <div className="coordinate-input">
                                    <label>Target X ({mapBounds.min} to {mapBounds.max}):</label>
                                    <input
                                        type="number"
                                        min={mapBounds.min}
                                        max={mapBounds.max}
                                        value={targetX}
                                        onChange={(e) => setTargetX(Number(e.target.value))}
                                    />
                                </div>
                                <div className="coordinate-input">
                                    <label>Target Y ({mapBounds.min} to {mapBounds.max}):</label>
                                    <input
                                        type="number"
                                        min={mapBounds.min}
                                        max={mapBounds.max}
                                        value={targetY}
                                        onChange={(e) => setTargetY(Number(e.target.value))}
                                    />
                                </div>
                                <div className="coordinate-input">
                                    <label>Target Z ({mapBounds.min} to {mapBounds.max}):</label>
                                    <input
                                        type="number"
                                        min={mapBounds.min}
                                        max={mapBounds.max}
                                        value={targetZ}
                                        onChange={(e) => setTargetZ(Number(e.target.value))}
                                    />
                                </div>
                                <button 
                                    className="move-button" 
                                    onClick={handleMove}
                                >
                                    Move Fleet
                                </button>
                                {moveMessage && (
                                    <div className={`move-message ${moveStatus}`}>
                                        {moveMessage}
                                    </div>
                                )}
                            </div>
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