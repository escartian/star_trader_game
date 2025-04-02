import React, { useEffect, useState } from 'react';
import { StarSystem, Planet } from '../types/game';
import { api } from '../services/api';
import { StarSystemModal } from './StarSystemModal';
import './GalaxyMap.css';

export const GalaxyMap: React.FC = () => {
    const [starSystems, setStarSystems] = useState<StarSystem[]>([]);
    const [selectedSystem, setSelectedSystem] = useState<StarSystem | null>(null);
    const [selectedSystemIndex, setSelectedSystemIndex] = useState<number | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const loadGalaxyMap = async () => {
            try {
                setLoading(true);
                const data = await api.getGalaxyMap();
                setStarSystems(data);
            } catch (err) {
                console.error('Failed to load galaxy map:', err);
                setError('Failed to load galaxy map');
            } finally {
                setLoading(false);
            }
        };

        loadGalaxyMap();
    }, []);

    const handleSystemClick = (system: StarSystem, index: number) => {
        setSelectedSystem(system);
        setSelectedSystemIndex(index);
    };

    const handleCloseSystem = () => {
        setSelectedSystem(null);
        setSelectedSystemIndex(null);
    };

    if (loading) {
        return <div className="loading">Loading galaxy map...</div>;
    }

    if (error) {
        return <div className="error">{error}</div>;
    }

    return (
        <div className="galaxy-map">
            <div className="star-systems-grid">
                {starSystems.map((system, index) => {
                    const isStarCentered = system.star.position.x === 0 && 
                                         system.star.position.y === 0 && 
                                         system.star.position.z === 0;
                    
                    return (
                        <div
                            key={index}
                            className="star-system"
                            onClick={() => handleSystemClick(system, index)}
                        >
                            <div className="star-system-content">
                                <h3>{system.star.name}</h3>
                                <p><strong>Type:</strong> {system.star.star_type}</p>
                                <p><strong>Position:</strong> ({system.position.x}, {system.position.y}, {system.position.z})</p>
                                {!isStarCentered && (
                                    <p><strong>Star Position:</strong> ({system.star.position.x}, {system.star.position.y}, {system.star.position.z})</p>
                                )}
                                <p><strong>Planets:</strong> {system.planets.length}</p>
                            </div>
                        </div>
                    );
                })}
            </div>
            {selectedSystem && selectedSystemIndex !== null && (
                <StarSystemModal
                    system={selectedSystem}
                    systemIndex={selectedSystemIndex}
                    onClose={handleCloseSystem}
                    selectedFleet={null}
                />
            )}
        </div>
    );
};
