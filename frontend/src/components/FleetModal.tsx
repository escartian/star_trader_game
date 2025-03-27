import React, { useEffect, useRef, useState } from 'react';
import { Fleet, Ship, Shield, Armor, Weapon } from '../types/game';
import { api } from '../services/api';
import { MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH } from '../constants';
import './FleetModal.css';

interface FleetModalProps {
    fleet: Fleet;
    onClose: () => void;
    onMove: (fleet: Fleet, x: number, y: number, z: number) => void;
}

export const FleetModal: React.FC<FleetModalProps> = ({ fleet, onClose, onMove }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [moveCoords, setMoveCoords] = useState({ x: 0, y: 0, z: 0 });
    const [isMoving, setIsMoving] = useState(false);
    const [error, setError] = useState<string | null>(null);

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

    const calculateDistance = () => {
        const dx = fleet.position.x - moveCoords.x;
        const dy = fleet.position.y - moveCoords.y;
        const dz = fleet.position.z - moveCoords.z;
        return Math.sqrt(dx * dx + dy * dy + dz * dz);
    };

    const handleMove = async () => {
        try {
            // Check world borders using MAP constants
            if (moveCoords.x < -MAP_WIDTH || moveCoords.x > MAP_WIDTH || 
                moveCoords.y < -MAP_HEIGHT || moveCoords.y > MAP_HEIGHT || 
                moveCoords.z < -MAP_LENGTH || moveCoords.z > MAP_LENGTH) {
                setError(`Cannot move beyond world borders (-${MAP_WIDTH} to ${MAP_WIDTH}, -${MAP_HEIGHT} to ${MAP_HEIGHT}, -${MAP_LENGTH} to ${MAP_LENGTH})`);
                return;
            }

            const distance = calculateDistance();
            if (distance === 0) {
                setError("No movement detected");
                return;
            }

            setIsMoving(true);
            setError(null);
            await onMove(fleet, moveCoords.x, moveCoords.y, moveCoords.z);
        } catch (err) {
            console.error('Error moving fleet:', err);
            setError('Failed to move fleet');
        } finally {
            setIsMoving(false);
        }
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose();
        }
    };

    const renderShield = (shield: Shield) => {
        return `${shield.current}/${shield.capacity} (${shield.regen}/s)`;
    };

    const renderArmor = (armor: Armor) => {
        return `${armor.current}/${armor.capacity} (${armor.regen}/s)`;
    };

    const renderWeapon = (weapon: Weapon) => {
        const weaponType = Object.keys(weapon)[0];
        const damage = weapon[weaponType as keyof Weapon]?.damage || 0;
        return `${weaponType.replace(/([A-Z])/g, ' $1').trim()} (${damage} DMG)`;
    };

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="modal-content">
                <div className="modal-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="modal-body">
                    <div className="fleet-details">
                        <h3>Fleet Details</h3>
                        <div className="fleet-info">
                            <div className="stat-group">
                                <h4>Position</h4>
                                <p>X: {fleet.position.x}</p>
                                <p>Y: {fleet.position.y}</p>
                                <p>Z: {fleet.position.z}</p>
                            </div>
                            <div className="stat-group">
                                <h4>Fleet Info</h4>
                                <p>Owner: {fleet.owner_id}</p>
                                <p>Number of Ships: {fleet.ships.length}</p>
                            </div>
                        </div>
                    </div>
                    <div className="ships-grid">
                        {fleet.ships.map((ship, index) => (
                            <div key={index} className="ship-card">
                                <h4>{ship.name}</h4>
                                <div className="ship-details">
                                    <div className="stat-group">
                                        <h4>Ship Stats</h4>
                                        <p>Type: {ship.specialization}</p>
                                        <p>Size: {ship.size}</p>
                                        <p>HP: {ship.hp}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Defenses</h4>
                                        <p>Shields: {renderShield(ship.shields)}</p>
                                        <p>Armor: {renderArmor(ship.armor)}</p>
                                    </div>
                                    {ship.weapons.length > 0 && (
                                        <div className="stat-group">
                                            <h4>Weapons</h4>
                                            {ship.weapons.map((weapon, idx) => (
                                                <p key={idx}>{renderWeapon(weapon)}</p>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            </div>
                        ))}
                    </div>
                    <div className="move-controls">
                        <h3>Move Fleet</h3>
                        <div className="move-inputs">
                            <div className="input-group">
                                <label>X:</label>
                                <input
                                    type="number"
                                    value={moveCoords.x}
                                    onChange={(e) => setMoveCoords(prev => ({ ...prev, x: parseInt(e.target.value) || 0 }))}
                                />
                            </div>
                            <div className="input-group">
                                <label>Y:</label>
                                <input
                                    type="number"
                                    value={moveCoords.y}
                                    onChange={(e) => setMoveCoords(prev => ({ ...prev, y: parseInt(e.target.value) || 0 }))}
                                />
                            </div>
                            <div className="input-group">
                                <label>Z:</label>
                                <input
                                    type="number"
                                    value={moveCoords.z}
                                    onChange={(e) => setMoveCoords(prev => ({ ...prev, z: parseInt(e.target.value) || 0 }))}
                                />
                            </div>
                        </div>
                        {error && <div className="move-message error">{error}</div>}
                        <div className="distance-info">
                            Distance: {calculateDistance().toFixed(2)}
                        </div>
                        <button className="move-button" onClick={handleMove} disabled={isMoving}>
                            {isMoving ? 'Moving...' : 'Move Fleet'}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}; 