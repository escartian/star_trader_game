import React, { useEffect, useRef, useState } from 'react';
import { StarSystem, Planet, Fleet } from '../types/game';
import { api } from '../services/api';
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

    const handleSendFleetToSystem = async () => {
        try {
            if (!selectedFleet) return;
            const parts = selectedFleet.name.split('_');
            const owner_id = encodeURIComponent(parts[1]);
            const fleet_number = parseInt(parts[2]);
            // Use explicit galaxy intent to ensure deep space travel to system
            await api.moveFleet(owner_id, fleet_number, {
                x: system.position.x,
                y: system.position.y,
                z: system.position.z,
                system_id: systemIndex,
            });
            onClose();
        } catch (e) {
            console.error('Failed to send fleet to system:', e);
        }
    };

    const handleSendFleetToPlanet = async (planet: Planet) => {
        try {
            if (!selectedFleet) return;
            const parts = selectedFleet.name.split('_');
            const owner_id = encodeURIComponent(parts[1]);
            const fleet_number = parseInt(parts[2]);
            // If fleet is already in this system, move directly to the planet.
            // Otherwise, initiate deep-space travel by moving "out of the system"
            // Use one-call planet intent so backend handles traversal and entry
            // For planet moves, send galaxy coordinates derived from in-system position
            // and include stable system id when available
            await api.moveFleet(owner_id, fleet_number, {
                x: planet.position.x,
                y: planet.position.y,
                z: planet.position.z,
                space: 'system',
                system_id: system.id ?? systemIndex,
                planet_id: system.planets.indexOf(planet),
            });
            onClose();
        } catch (e) {
            console.error('Failed to send fleet to planet:', e);
        }
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
                    {selectedFleet && (
                        <div className="system-actions" style={{ margin: '8px 0' }}>
                            <button onClick={handleSendFleetToSystem}>Send Selected Fleet Here</button>
                        </div>
                    )}
                    <div className="planets-grid">
                        {system.planets.map((planet) => (
                            <div key={planet.name} className="planet-card">
                                <h3>{planet.name}</h3>
                                <div className="planet-details">
                                    <p><strong>Biome:</strong> {planet.biome}</p>
                                    <p><strong>Economy:</strong> {planet.economy}</p>
                                    <p><strong>Specialization:</strong> {planet.specialization}</p>
                                    <p><strong>Danger Level:</strong> {planet.danger}</p>
                                    <p><strong>Local Position:</strong> ({planet.position.x}, {planet.position.y}, {planet.position.z})</p>
                                </div>
                                <div className="planet-actions">
                                    <button onClick={() => handleShowMarket(planet)}>
                                        Resource Market
                                    </button>
                                    <button onClick={() => handleShowShipMarket(planet)}>
                                        Ship Market
                                    </button>
                                    {selectedFleet && (
                                        <button onClick={() => handleSendFleetToPlanet(planet)}>
                                            Send Selected Fleet To Planet
                                        </button>
                                    )}
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