import React from 'react';
import { Fleet, Ship } from '../types/game';
import './FleetModal.css';

interface FleetModalProps {
    fleet: Fleet;
    onClose: () => void;
}

export const FleetModal: React.FC<FleetModalProps> = ({ fleet, onClose }) => {
    return (
        <div className="modal-overlay">
            <div className="fleet-modal">
                <div className="fleet-modal-header">
                    <h2>{fleet.name}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                <div className="fleet-info">
                    <div className="fleet-summary">
                        <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                        <p>Current System: {fleet.current_system_id || 'In Transit'}</p>
                        <p>Total Ships: {fleet.ships.length}</p>
                    </div>
                    <div className="ships-grid">
                        {fleet.ships.map((ship) => (
                            <div key={ship.name} className="ship-card">
                                <h3>{ship.name}</h3>
                                <div className="ship-type-badge">{ship.specialization}</div>
                                <div className="ship-stats">
                                    <div className="stat-group">
                                        <h4>Basic Info</h4>
                                        <p>Type: {ship.specialization}</p>
                                        <p>Size: {ship.size}</p>
                                        <p>Engine: {ship.engine}</p>
                                        <p>HP: {ship.hp}</p>
                                        <p>Status: {ship.status}</p>
                                        <p>Combat State: {ship.combat_state}</p>
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
                                        <h4>Cargo</h4>
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
    );
}; 