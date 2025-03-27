import React, { useState, useEffect } from 'react';
import { Fleet } from '../types/game';
import { api } from '../services/api';
import './EncounterModal.css';

interface EncounterModalProps {
    fleet: Fleet;
    encounteredFleet: Fleet;
    onClose: () => void;
    onCombat: (attacker: Fleet, defender: Fleet) => void;
}

type EncounterType = 'ambush' | 'trade' | 'diplomatic' | 'pirate' | 'military';

interface EncounterScenario {
    type: EncounterType;
    title: string;
    description: string;
    options: {
        text: string;
        action: () => void;
        requiresCombat?: boolean;
    }[];
}

export const EncounterModal: React.FC<EncounterModalProps> = ({ 
    fleet, 
    encounteredFleet, 
    onClose,
    onCombat 
}) => {
    const [scenario, setScenario] = useState<EncounterScenario | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        determineEncounterType();
    }, []);

    const determineEncounterType = () => {
        // Randomly determine encounter type based on fleet types and positions
        const types: EncounterType[] = ['ambush', 'trade', 'diplomatic', 'pirate', 'military'];
        const randomType = types[Math.floor(Math.random() * types.length)];
        
        const scenarios: Record<EncounterType, EncounterScenario> = {
            ambush: {
                type: 'ambush',
                title: 'Ambush!',
                description: `A stealth-equipped fleet has emerged from the shadows! Their FTL disruptors are preventing your escape.`,
                options: [
                    {
                        text: 'Fight for survival!',
                        action: () => onCombat(encounteredFleet, fleet),
                        requiresCombat: true
                    }
                ]
            },
            trade: {
                type: 'trade',
                title: 'Trade Opportunity',
                description: `A merchant fleet has offered to trade. They seem well-armed but peaceful.`,
                options: [
                    {
                        text: 'Engage in peaceful trade',
                        action: () => {
                            // TODO: Implement trade system
                            onClose();
                        }
                    },
                    {
                        text: 'Attempt to pirate their cargo',
                        action: () => onCombat(fleet, encounteredFleet),
                        requiresCombat: true
                    }
                ]
            },
            diplomatic: {
                type: 'diplomatic',
                title: 'Diplomatic Encounter',
                description: `A diplomatic envoy wishes to discuss terms. They are heavily armed but appear to be seeking peaceful resolution.`,
                options: [
                    {
                        text: 'Engage in diplomacy',
                        action: () => {
                            // TODO: Implement diplomacy system
                            onClose();
                        }
                    },
                    {
                        text: 'Attack despite their peaceful intentions',
                        action: () => onCombat(fleet, encounteredFleet),
                        requiresCombat: true
                    }
                ]
            },
            pirate: {
                type: 'pirate',
                title: 'Pirate Fleet',
                description: `A notorious pirate fleet has appeared! They're demanding tribute or threatening combat.`,
                options: [
                    {
                        text: 'Pay tribute to avoid combat',
                        action: () => {
                            // TODO: Implement tribute system
                            onClose();
                        }
                    },
                    {
                        text: 'Fight the pirates!',
                        action: () => onCombat(fleet, encounteredFleet),
                        requiresCombat: true
                    }
                ]
            },
            military: {
                type: 'military',
                title: 'Military Patrol',
                description: `A military fleet has detected your presence. They're scanning your ships and preparing for potential combat.`,
                options: [
                    {
                        text: 'Attempt to flee',
                        action: () => {
                            // TODO: Implement escape mechanics
                            onClose();
                        }
                    },
                    {
                        text: 'Stand and fight',
                        action: () => onCombat(fleet, encounteredFleet),
                        requiresCombat: true
                    }
                ]
            }
        };

        setScenario(scenarios[randomType]);
        setIsLoading(false);
    };

    const handleCombat = () => {
        onCombat(fleet, encounteredFleet);
    };

    const handleIgnore = () => {
        onClose();
    };

    if (isLoading) {
        return (
            <div className="modal-overlay">
                <div className="encounter-modal">
                    <div className="encounter-loading">
                        Analyzing encounter...
                    </div>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="modal-overlay">
                <div className="encounter-modal">
                    <div className="encounter-error">
                        {error}
                    </div>
                    <button className="close-button" onClick={onClose}>Close</button>
                </div>
            </div>
        );
    }

    return (
        <div className="modal-overlay">
            <div className="encounter-modal">
                <div className="encounter-header">
                    <h2>{scenario?.title}</h2>
                    <button className="close-button" onClick={onClose}>×</button>
                </div>
                
                <div className="encounter-content">
                    <div className="encounter-description">
                        {scenario?.description}
                    </div>

                    <div className="fleet-info">
                        <div className="your-fleet">
                            <h3>Your Fleet</h3>
                            <p>Ships: {fleet.ships.length}</p>
                            <p>Position: ({fleet.position.x}, {fleet.position.y}, {fleet.position.z})</p>
                        </div>
                        <div className="encountered-fleet">
                            <h3>Encountered Fleet</h3>
                            <p>Ships: {encounteredFleet.ships.length}</p>
                            <p>Position: ({encounteredFleet.position.x}, {encounteredFleet.position.y}, {encounteredFleet.position.z})</p>
                        </div>
                    </div>

                    <div className="encounter-options">
                        <button
                            className="combat-button"
                            onClick={handleCombat}
                        >
                            Engage in Combat
                        </button>
                        <button
                            className="ignore-button"
                            onClick={handleIgnore}
                        >
                            Ignore Situation
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}; 