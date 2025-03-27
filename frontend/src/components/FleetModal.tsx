import React, { useEffect, useRef } from 'react';
import { Fleet, Ship } from '../types/game';
import './FleetModal.css';

interface FleetModalProps {
    fleet: Fleet;
    onClose: () => void;
}

export const FleetModal: React.FC<FleetModalProps> = ({ fleet, onClose }) => {
    const modalRef = useRef<HTMLDivElement>(null);

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

    return (
        <div className="modal-overlay">
            <div className="fleet-modal" ref={modalRef}>
                <div className="fleet-modal-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="fleet-info">
                    <div className="fleet-summary">
                        <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                        <p>Owner: {fleet.owner_id}</p>
                        <p>Total Ships: {fleet.ships.length}</p>
                    </div>
                    <div className="ships-grid">
                        {fleet.ships.map((ship, index) => (
                            <div key={index} className="ship-card">
                                <h4>{ship.name}</h4>
                                <div className="ship-details">
                                    <div className="stat-group">
                                        <h4>Status</h4>
                                        <p>{ship.status}</p>
                                        <p>Combat State: {ship.combat_state}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Health</h4>
                                        <p>HP: {ship.hp}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Shields</h4>
                                        <p>Capacity: {ship.shields.capacity}</p>
                                        <p>Current: {ship.shields.current}</p>
                                        <p>Regen: {ship.shields.regen}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Armor</h4>
                                        <p>Capacity: {ship.armor.capacity}</p>
                                        <p>Current: {ship.armor.current}</p>
                                        <p>Regen: {ship.armor.regen}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Specifications</h4>
                                        <p>Type: {ship.specialization}</p>
                                        <p>Size: {ship.size}</p>
                                        <p>Engine: {ship.engine}</p>
                                    </div>
                                    <div className="stat-group">
                                        <h4>Weapons</h4>
                                        {ship.weapons.map((weapon, weaponIndex) => (
                                            <div key={weaponIndex} className="weapon-info">
                                                {Object.entries(weapon).map(([name, data]) => (
                                                    <p key={name}>
                                                        {name.replace(/([A-Z])/g, ' $1').trim()}: {data.damage} DMG
                                                    </p>
                                                ))}
                                            </div>
                                        ))}
                                    </div>
                                    <div className="stat-group">
                                        <h4>Cargo</h4>
                                        {ship.cargo.map((resource, cargoIndex) => (
                                            <p key={cargoIndex}>
                                                {resource.resource_type}: {resource.quantity || 0}
                                            </p>
                                        ))}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}; 