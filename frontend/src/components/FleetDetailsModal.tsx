import React from 'react';
import { Fleet } from '../types/game';
import './FleetDetailsModal.css';

interface FleetDetailsModalProps {
    fleet: Fleet;
    onClose: () => void;
}

export const FleetDetailsModal: React.FC<FleetDetailsModalProps> = ({ fleet, onClose }) => {
    return (
        <div className="modal-overlay">
            <div className="modal-content fleet-modal">
                <div className="modal-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="fleet-info">
                    <div className="fleet-summary">
                        <h3>Fleet Summary</h3>
                        <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                        <p>Total Ships: {fleet.ships.length}</p>
                        <p>Current System: {fleet.current_system_id ? `System ${fleet.current_system_id}` : 'In Transit'}</p>
                    </div>
                    <div className="ships-list">
                        <h3>Ships</h3>
                        <div className="ships-grid">
                            {fleet.ships.map((ship, index) => (
                                <div key={index} className="ship-card">
                                    <h4>{ship.name}</h4>
                                    <div className="ship-stats">
                                        <div className="stat-group">
                                            <h5>Basic Info</h5>
                                            <p>Type: {ship.specialization}</p>
                                            <p>Size: {ship.size}</p>
                                            <p>Engine: {ship.engine}</p>
                                            <p>HP: {ship.hp}</p>
                                            <p>Status: {ship.status}</p>
                                            <p>Combat State: {ship.combat_state}</p>
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
                                            <h4>Weapons</h4>
                                            <div className="weapons-grid">
                                                {ship.weapons.map((weapon, i) => {
                                                    const weaponType = Object.keys(weapon)[0];
                                                    const weaponName = weaponType
                                                        .replace(/([A-Z])/g, ' $1')
                                                        .trim();
                                                    const weaponDamage = weapon[weaponType as keyof typeof weapon]?.damage || 0;
                                                    return (
                                                        <div key={i} className="weapon-card">
                                                            <div className="weapon-header">
                                                                <span className="weapon-name">
                                                                    {weaponName}
                                                                </span>
                                                                <span className="weapon-damage">
                                                                    DMG: {weaponDamage}
                                                                </span>
                                                            </div>
                                                            <div className="weapon-stats">
                                                                <div className="damage-bar" 
                                                                     style={{width: `${(weaponDamage/100)*100}%`}}>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    );
                                                })}
                                            </div>
                                        </div>
                                        <div className="stat-group">
                                            <h5>Cargo</h5>
                                            <ul>
                                                {ship.cargo.map((resource, i) => (
                                                    <li key={i}>
                                                        {resource.resource_type}: {resource.quantity || 0}
                                                    </li>
                                                ))}
                                            </ul>
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
