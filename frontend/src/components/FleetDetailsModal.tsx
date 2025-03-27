import React, { useRef, useEffect } from 'react';
import { Fleet } from '../types/game';
import './FleetDetailsModal.css';

interface FleetDetailsModalProps {
    fleet: Fleet;
    onClose: () => void;
}

export const FleetDetailsModal: React.FC<FleetDetailsModalProps> = ({ fleet, onClose }) => {
    const modalRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        console.log('Fleet data:', JSON.stringify(fleet, null, 2));
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                onClose();
            }
        };

        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose]);

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose();
        }
    };

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="fleet-details-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="fleet-details-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="fleet-details-content">
                    <div className="fleet-info">
                        <h3>Fleet Information</h3>
                        <p><strong>Owner:</strong> {fleet.owner}</p>
                        <p><strong>Position:</strong> ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                        <p><strong>Total Ships:</strong> {fleet.ships.length}</p>
                    </div>
                    <div className="ships-list">
                        <h3>Ships</h3>
                        <div className="ships-grid">
                            {fleet.ships.map((ship, index) => (
                                <div key={index} className="ship-card">
                                    <h4>{ship.name}</h4>
                                    <div className="ship-details">
                                        <div className="stat-group">
                                            <h5>Status</h5>
                                            <p>{ship.status}</p>
                                            <p>Combat State: {ship.combat_state}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Health</h5>
                                            <p>HP: {ship.hp}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Shields</h5>
                                            <p>Capacity: {ship.shields.capacity}</p>
                                            <p>Current: {ship.shields.current}</p>
                                            <p>Regen: {ship.shields.regen}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Armor</h5>
                                            <p>Capacity: {ship.armor.capacity}</p>
                                            <p>Current: {ship.armor.current}</p>
                                            <p>Regen: {ship.armor.regen}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Specifications</h5>
                                            <p>Type: {ship.specialization}</p>
                                            <p>Size: {ship.size}</p>
                                            <p>Engine: {ship.engine}</p>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Weapons</h5>
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
                                            <h5>Cargo</h5>
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
        </div>
    );
};
