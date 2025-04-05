import React, { useEffect, useRef, useState } from 'react';
import { StarSystem, Planet, Fleet } from '../types/game';
import './StarSystemModal.css';
import { MarketModal } from './MarketModal';
import { ShipMarketModal } from './ShipMarketModal';

interface StarSystemModalProps {
    system: StarSystem;
    systemIndex: number;
    selectedFleet?: Fleet | null;
    onClose: () => void;
}

export const StarSystemModal: React.FC<StarSystemModalProps> = ({ system, systemIndex, selectedFleet = null, onClose }) => {
    const [selectedPlanet, setSelectedPlanet] = useState<Planet | null>(null);
    const [showMarket, setShowMarket] = useState(false);
    const [showShipMarket, setShowShipMarket] = useState(false);
    const modalRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                if (showMarket || showShipMarket) {
                    setShowMarket(false);
                    setShowShipMarket(false);
                    setSelectedPlanet(null);
                } else {
                    onClose();
                }
            }
        };

        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose, showMarket, showShipMarket]);

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === e.currentTarget) {
            if (showMarket || showShipMarket) {
                setShowMarket(false);
                setShowShipMarket(false);
                setSelectedPlanet(null);
            } else {
                onClose();
            }
        }
    };

    const handleShowMarket = (planet: Planet) => {
        setSelectedPlanet(planet);
        setShowMarket(true);
    };

    const handleShowShipMarket = (planet: Planet) => {
        setSelectedPlanet(planet);
        setShowShipMarket(true);
    };

    const handleClose = () => {
        if (showMarket || showShipMarket) {
            setShowMarket(false);
            setShowShipMarket(false);
            setSelectedPlanet(null);
        } else {
            onClose();
        }
    };

    const isStarCentered = system.star.position.x === 0 && 
                          system.star.position.y === 0 && 
                          system.star.position.z === 0;

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="star-system-modal" ref={modalRef} onClick={handleModalClick}>
                <div className="star-system-modal-header">
                    <h2>{system.star.name}</h2>
                    <button className="close-button" onClick={handleClose}>×</button>
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
                        {system.planets.map((planet) => (
                            <div key={planet.name} className="planet-card">
                                <h3>{planet.name}</h3>
                                <div className="planet-details">
                                    <p><strong>Biome:</strong> {planet.biome}</p>
                                    <p><strong>Economy:</strong> {planet.economy}</p>
                                    <p><strong>Specialization:</strong> {planet.specialization}</p>
                                    <p><strong>Danger Level:</strong> {planet.danger}</p>
                                </div>
                                <div className="planet-actions">
                                    <button onClick={() => handleShowMarket(planet)}>
                                        Resource Market
                                    </button>
                                    <button onClick={() => handleShowShipMarket(planet)}>
                                        Ship Market
                                    </button>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
                {showMarket && selectedPlanet && (
                    <MarketModal
                        isOpen={showMarket}
                        onClose={() => {
                            setShowMarket(false);
                            setSelectedPlanet(null);
                        }}
                        systemId={systemIndex}
                        planetId={system.planets.indexOf(selectedPlanet)}
                        planet={selectedPlanet}
                    />
                )}
                {showShipMarket && selectedPlanet && (
                    <ShipMarketModal
                        isOpen={showShipMarket}
                        onClose={() => {
                            setShowShipMarket(false);
                            setSelectedPlanet(null);
                        }}
                        systemId={systemIndex}
                        planetId={system.planets.indexOf(selectedPlanet)}
                    />
                )}
            </div>
        </div>
    );
}; 