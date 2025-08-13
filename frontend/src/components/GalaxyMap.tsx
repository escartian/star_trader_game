import React, { useEffect, useState } from 'react';
import { StarSystem, Planet, Fleet, GameSettings } from '../types/game';
import { api } from '../services/api';
import { StarSystemModal } from './StarSystemModal';
import './GalaxyMap.css';

interface GalaxyMapProps {
    selectedFleet?: Fleet | null;
}

export const GalaxyMap: React.FC<GalaxyMapProps> = ({ selectedFleet = null }) => {
    const [systems, setSystems] = useState<StarSystem[]>([]);
    const [selectedSystem, setSelectedSystem] = useState<StarSystem | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [filterText, setFilterText] = useState<string>('');
    const [sortKey, setSortKey] = useState<'name' | 'planets'>('name');
    const [galaxyBound, setGalaxyBound] = useState<number | null>(null);
    const [playerFleets, setPlayerFleets] = useState<Fleet[]>([]);
    const [selectedPlayerFleet, setSelectedPlayerFleet] = useState<Fleet | null>(selectedFleet);

    useEffect(() => {
        const loadSystems = async () => {
            try {
                setLoading(true);
                setError(null);
                const systems = await api.getGalaxyMap();
                setSystems(systems);
                // Also load settings for bounds display + player fleets
                const settings = await api.getGameSettings();
                setGalaxyBound(Math.floor(settings.map_width));
                const fleets = await api.getOwnerFleets(settings.player_name);
                setPlayerFleets(fleets);
                if (!selectedPlayerFleet && fleets.length > 0) {
                    setSelectedPlayerFleet(fleets[0]);
                }
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

    const visibleSystems = systems
        .filter(s => !filterText || s.star.name.toLowerCase().includes(filterText.toLowerCase()))
        .sort((a, b) => sortKey === 'name' ? a.star.name.localeCompare(b.star.name) : b.planets.length - a.planets.length);

    const handleClear = () => {
        setFilterText('');
        setSortKey('name');
    };

    return (
        <div className="galaxy-map">
            <div className="galaxy-map-container">
                <div className="galaxy-map-header">
                    <h2 className="galaxy-map-title">Galaxy Map</h2>
                    <div className="galaxy-map-controls">
                        {galaxyBound !== null && (
                            <span className="galaxy-bounds">{`Galaxy bounds: -${galaxyBound} .. +${galaxyBound}`}</span>
                        )}
                        <input
                            placeholder="Filter by name..."
                            value={filterText}
                            onChange={(e) => setFilterText(e.target.value)}
                            className="galaxy-map-filter"
                        />
                        <select value={sortKey} onChange={(e) => setSortKey(e.target.value as 'name' | 'planets')}>
                            <option value="name">Sort: Name</option>
                            <option value="planets">Sort: # Planets</option>
                        </select>
                        <button className="galaxy-map-button" onClick={handleClear}>Clear</button>
                    </div>
                </div>
                <div className="galaxy-map-content">
                    <div className="selected-fleet-banner" style={{ marginBottom: '8px', display: 'flex', gap: '8px', alignItems: 'center' }}>
                        <span>Selected Fleet:</span>
                        <select
                            value={selectedPlayerFleet ? selectedPlayerFleet.name : ''}
                            onChange={(e) => {
                                const f = playerFleets.find(fl => fl.name === e.target.value) || null;
                                setSelectedPlayerFleet(f);
                            }}
                        >
                            {playerFleets.map(fl => (
                                <option key={fl.name} value={fl.name}>{fl.name}</option>
                            ))}
                        </select>
                    </div>
                    <div className="systems-grid">
                        {visibleSystems.map((system) => (
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
            {selectedSystem && selectedPlayerFleet && (
                <StarSystemModal
                    system={selectedSystem}
                    systemIndex={systems.findIndex(s => s.star.name === selectedSystem.star.name)}
                selectedFleet={selectedPlayerFleet}
                    onClose={handleCloseModal}
                />
            )}
        </div>
    );
};
