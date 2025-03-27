import React, { useEffect, useRef, useState } from 'react';
import { StarSystem, Planet } from '../types/game';
import './StarSystemModal.css';
import { MarketModal } from './MarketModal';

interface StarSystemModalProps {
    system: StarSystem;
    systemIndex: number;
    onClose: () => void;
}

export const StarSystemModal: React.FC<StarSystemModalProps> = ({ system, systemIndex, onClose }) => {
    const [selectedPlanet, setSelectedPlanet] = useState<Planet | null>(null);
    const modalRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                if (!selectedPlanet) {
                    onClose();
                }
            }
        };

        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose, selectedPlanet]);

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget && !selectedPlanet) {
            onClose();
        }
    };

    const handleCloseMarket = () => {
        setSelectedPlanet(null);
    };

    const isStarCentered = system.star.position.x === 0 && 
                          system.star.position.y === 0 && 
                          system.star.position.z === 0;

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="star-system-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="star-system-modal-header">
                    <h2>{system.star.name}</h2>
                    <button className="close-button" onClick={onClose} disabled={!!selectedPlanet}>×</button>
                </div>
                <div className="star-system-content">
                    <div className="system-info">
                        <div className="star-info">
                            <h3>Star Information</h3>
                            <p><strong>Type:</strong> {system.star.star_type}</p>
                            <p><strong>System Position:</strong> ({system.position.x}, {system.position.y}, {system.position.z})</p>
                            {!isStarCentered && (
                                <p><strong>Star Position:</strong> ({system.star.position.x}, {system.star.position.y}, {system.star.position.z})</p>
                            )}
                        </div>
                        <div className="planets-info">
                            <h3>Planets ({system.planets.length})</h3>
                        </div>
                    </div>
                    <div className="planets-grid">
                        {system.planets.map((planet, index) => (
                            <div key={index} className="planet-card">
                                <h3>{planet.name}</h3>
                                <div className="planet-details">
                                    <p><strong>Biome:</strong> {planet.biome}</p>
                                    <p><strong>Economy:</strong> {planet.economy}</p>
                                    <p><strong>Specialization:</strong> {planet.specialization}</p>
                                    <p><strong>Danger Level:</strong> {planet.danger}</p>
                                </div>
                                <button 
                                    className="view-market-button"
                                    onClick={() => setSelectedPlanet(planet)}
                                >
                                    View Market
                                </button>
                            </div>
                        ))}
                    </div>
                </div>
                {selectedPlanet && (
                    <MarketModal
                        planet={selectedPlanet}
                        systemId={systemIndex}
                        planetId={system.planets.indexOf(selectedPlanet)}
                        onClose={handleCloseMarket}
                    />
                )}
            </div>
        </div>
    );
}; 