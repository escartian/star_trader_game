import React, { useEffect, useState } from 'react';
import { StarSystem, Planet } from '../types/game';
import { api } from '../services/api';
import { StarSystemModal } from './StarSystemModal';
import './GalaxyMap.css';

export const GalaxyMap: React.FC = () => {
    const [systems, setSystems] = useState<StarSystem[]>([]);
    const [selectedSystem, setSelectedSystem] = useState<StarSystem | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const loadSystems = async () => {
            try {
                setLoading(true);
                setError(null);
                const systems = await api.getGalaxyMap();
                setSystems(systems);
            } catch (err) {
                console.error('Error loading star systems:', err);
                setError(err instanceof Error ? err.message : 'Failed to load star systems');
            } finally {
                setLoading(false);
            }
        };

        loadSystems();
    }, []);

    const handleSystemClick = (system: StarSystem) => {
        setSelectedSystem(system);
    };

    const handleCloseModal = () => {
        setSelectedSystem(null);
    };

    if (loading) {
        return <div className="galaxy-map">Loading galaxy map...</div>;
    }

    if (error) {
        return <div className="galaxy-map error">{error}</div>;
    }

    return (
        <div className="galaxy-map">
            <div className="galaxy-map-container">
                <div className="galaxy-map-header">
                    <h2 className="galaxy-map-title">Galaxy Map</h2>
                    <div className="galaxy-map-controls">
                        <button className="galaxy-map-button">Filter Systems</button>
                        <button className="galaxy-map-button">Sort By</button>
                    </div>
                </div>
                <div className="galaxy-map-content">
                    <div className="systems-grid">
                        {systems.map((system) => (
                            <div
                                key={system.star.name}
                                className={`system-card ${selectedSystem?.star.name === system.star.name ? 'selected' : ''}`}
                                onClick={() => handleSystemClick(system)}
                            >
                                <h3 className="system-name">{system.star.name}</h3>
                                <div className="system-info">
                                    <div className="system-stat">
                                        <div className="stat-label">Star Type</div>
                                        <div className="stat-value">{system.star.star_type}</div>
                                    </div>
                                    <div className="system-stat">
                                        <div className="stat-label">Position</div>
                                        <div className="stat-value">
                                            ({system.position.x}, {system.position.y}, {system.position.z})
                                        </div>
                                    </div>
                                    <div className="system-stat">
                                        <div className="stat-label">Planets</div>
                                        <div className="stat-value">{system.planets.length}</div>
                                    </div>
                                </div>
                                <div className="planets-list">
                                    {system.planets.map((planet) => (
                                        <div key={`${system.star.name}-${planet.name}`} className="planet-item">
                                            <span className="planet-name">{planet.name}</span>
                                            <span className="planet-type">{planet.biome}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
            {selectedSystem && (
                <StarSystemModal
                    system={selectedSystem}
                    systemIndex={systems.findIndex(s => s.star.name === selectedSystem.star.name)}
                    onClose={handleCloseModal}
                />
            )}
        </div>
    );
};
