import React, { useState } from 'react';
import { Fleet } from '../types/game';
import { api } from '../services/api';
import './CombatModal.css';

interface CombatModalProps {
    attacker: Fleet;
    defender: Fleet;
    onClose: () => void;
}

export const CombatModal: React.FC<CombatModalProps> = ({ attacker, defender, onClose }) => {
    const [combatResult, setCombatResult] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleInitiateCombat = async () => {
        setIsLoading(true);
        setError(null);
        try {
            const attackerNumber = parseInt(attacker.name.split('_')[2]);
            const defenderNumber = parseInt(defender.name.split('_')[2]);
            
            const result = await api.initiateCombat(
                attacker.owner_id,
                attackerNumber,
                defender.owner_id,
                defenderNumber
            );
            
            setCombatResult(result);
        } catch (err) {
            setError('Failed to initiate combat');
            console.error(err);
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div className="modal-overlay">
            <div className="combat-modal">
                <div className="combat-modal-header">
                    <h2>Combat</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                
                <div className="combat-info">
                    <div className="fleet-comparison">
                        <div className="attacker-info">
                            <h3>Attacker: {attacker.name}</h3>
                            <p>Ships: {attacker.ships.length}</p>
                            <p>Position: ({attacker.position.x}, {attacker.position.y}, {attacker.position.z})</p>
                        </div>
                        <div className="defender-info">
                            <h3>Defender: {defender.name}</h3>
                            <p>Ships: {defender.ships.length}</p>
                            <p>Position: ({defender.position.x}, {defender.position.y}, {defender.position.z})</p>
                        </div>
                    </div>

                    {!combatResult && !isLoading && !error && (
                        <button 
                            className="initiate-combat-btn"
                            onClick={handleInitiateCombat}
                        >
                            Initiate Combat
                        </button>
                    )}

                    {isLoading && (
                        <div className="combat-loading">
                            Resolving combat...
                        </div>
                    )}

                    {error && (
                        <div className="combat-error">
                            {error}
                        </div>
                    )}

                    {combatResult && (
                        <div className="combat-result">
                            <h3>Combat Results</h3>
                            <pre>{combatResult}</pre>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}; 