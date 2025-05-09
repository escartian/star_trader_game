import React, { useState, useEffect } from 'react';
import { api } from '../services/api';
import { GameSettings, SavedGame, Faction } from '../types/game';
import CanvasBackground from './CanvasBackground';
import './MainMenu.css';

interface MainMenuProps {
    onStartGame: () => void;
}

export const MainMenu: React.FC<MainMenuProps> = ({ onStartGame }) => {
    const [savedGames, setSavedGames] = useState<SavedGame[]>([]);
    const [showNewGame, setShowNewGame] = useState(false);
    const [showAdvancedSettings, setShowAdvancedSettings] = useState(false);
    const [isMenuHidden, setIsMenuHidden] = useState(false);
    const [settings, setSettings] = useState<GameSettings>({
        game_id: Date.now().toString(),
        display_name: 'New Game',
        player_name: 'Player1',
        map_width: 100,
        map_height: 100,
        map_length: 100,
        star_count: 10,
        starting_credits: 1000,
        print_debug: true,
        max_combat_time: 500,
        factions: [
            { name: 'Federation', influence: 50 },
            { name: 'Empire', influence: 50 },
            { name: 'Republic', influence: 50 },
            { name: 'Alliance', influence: 50 }
        ],
        created_at: new Date().toISOString(),
        last_played: new Date().toISOString()
    });

    useEffect(() => {
        loadSavedGames();
        // Set initial comet count to 4 since menu is visible
        const event = new CustomEvent('updateCometCount', {
            detail: { count: 4 }
        });
        window.dispatchEvent(event);
    }, []);

    const toggleMenu = () => {
        setIsMenuHidden(!isMenuHidden);
        // Update comet count based on menu visibility
        const event = new CustomEvent('updateCometCount', {
            detail: { count: !isMenuHidden ? 2 : 4 }
        });
        window.dispatchEvent(event);
    };

    const loadSavedGames = async () => {
        try {
            const games = await api.listSavedGames();
            setSavedGames(games);
        } catch (error) {
            console.error('Failed to load saved games:', error);
        }
    };

    const handleNewGame = async () => {
        try {
            // Clear caches before creating new game
            await api.clearCaches();
            await api.createNewGame(settings);
            onStartGame();
        } catch (error) {
            console.error('Failed to create new game:', error);
        }
    };

    const handleLoadGame = async (gameId: string) => {
        try {
            await api.loadGame(gameId);
            onStartGame();
        } catch (error) {
            console.error('Failed to load game:', error);
        }
    };

    const handleDeleteGame = async (gameId: string) => {
        try {
            await api.deleteGame(gameId);
            loadSavedGames(); // Reload the list after deletion
        } catch (error) {
            console.error('Failed to delete game:', error);
        }
    };

    const addFaction = () => {
        setSettings({
            ...settings,
            factions: [...settings.factions, { name: 'New Faction', influence: 50 }]
        });
    };

    const removeFaction = (index: number) => {
        setSettings({
            ...settings,
            factions: settings.factions.filter((_, i) => i !== index)
        });
    };

    const updateFaction = (index: number, field: keyof Faction, value: string | number) => {
        const newFactions = [...settings.factions];
        newFactions[index] = { ...newFactions[index], [field]: value };
        setSettings({ ...settings, factions: newFactions });
    };

    return (
        <>
            <CanvasBackground />
            <button 
                className={`toggle-menu-button ${isMenuHidden ? 'visible' : ''}`}
                onClick={toggleMenu}
            >
                {isMenuHidden ? 'Show Menu' : 'Hide Menu'}
            </button>
            <div className={`main-menu ${isMenuHidden ? 'hidden' : ''}`}>
                <h1>Star Trader</h1>
                
                <div className="menu-buttons">
                    <button onClick={() => setShowNewGame(true)}>New Game</button>
                </div>

                {showNewGame && (
                    <div className="new-game-settings">
                        <h2>New Game Settings</h2>
                        <div className="settings-form">
                            <div className="setting-group">
                                <label>Game Name:</label>
                                <input
                                    type="text"
                                    value={settings.display_name}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        display_name: e.target.value
                                    })}
                                />
                            </div>

                            <div className="setting-group">
                                <label>Map Size:</label>
                                <div className="map-size-inputs">
                                    <input
                                        type="number"
                                        value={settings.map_width}
                                        onChange={(e) => setSettings({
                                            ...settings,
                                            map_width: parseInt(e.target.value),
                                            map_height: parseInt(e.target.value),
                                            map_length: parseInt(e.target.value)
                                        })}
                                        min="50"
                                        max="500"
                                    />
                                </div>
                            </div>

                            <div className="setting-group">
                                <label>Star Systems:</label>
                                <input
                                    type="number"
                                    value={settings.star_count}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        star_count: parseInt(e.target.value)
                                    })}
                                    min="5"
                                    max="50"
                                />
                            </div>

                            <div className="setting-group">
                                <label>Starting Credits:</label>
                                <input
                                    type="number"
                                    value={settings.starting_credits}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        starting_credits: parseInt(e.target.value)
                                    })}
                                    min="500"
                                    max="10000"
                                    step="100"
                                />
                            </div>

                            <div className="setting-group">
                                <label>Player Name:</label>
                                <input
                                    type="text"
                                    value={settings.player_name}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        player_name: e.target.value
                                    })}
                                />
                            </div>

                            <div className="setting-group">
                                <button onClick={() => setShowAdvancedSettings(!showAdvancedSettings)}>
                                    {showAdvancedSettings ? 'Hide Advanced Settings' : 'Show Advanced Settings'}
                                </button>
                            </div>

                            {showAdvancedSettings && (
                                <div className="advanced-settings">
                                    <h3>Faction Settings</h3>
                                    <p className="settings-description">Configure the major factions that will influence the galaxy. Each faction's influence affects their starting territory and fleet strength.</p>
                                    {settings.factions.map((faction, index) => (
                                        <div key={index} className="faction-settings">
                                            <div className="setting-group">
                                                <label>Faction Name:</label>
                                                <input
                                                    type="text"
                                                    value={faction.name}
                                                    onChange={(e) => updateFaction(index, 'name', e.target.value)}
                                                    placeholder="Enter faction name"
                                                />
                                            </div>
                                            <div className="setting-group">
                                                <label>Starting Influence:</label>
                                                <input
                                                    type="range"
                                                    value={faction.influence}
                                                    onChange={(e) => updateFaction(index, 'influence', parseInt(e.target.value))}
                                                    min="10"
                                                    max="90"
                                                    step="10"
                                                />
                                                <span className="influence-value">{faction.influence}%</span>
                                            </div>
                                            <button 
                                                onClick={() => removeFaction(index)}
                                                className="remove-faction-button"
                                                disabled={settings.factions.length <= 2}
                                                title={settings.factions.length <= 2 ? "At least 2 factions are required" : "Remove this faction"}
                                            >
                                                Remove Faction
                                            </button>
                                        </div>
                                    ))}
                                    <button
                                        onClick={addFaction}
                                        className="add-faction-button"
                                        disabled={settings.factions.length >= 6}
                                        title={settings.factions.length >= 6 ? "Maximum 6 factions allowed" : "Add a new faction"}
                                    >
                                        Add Faction
                                    </button>
                                </div>
                            )}

                            <button onClick={handleNewGame}>Start Game</button>
                        </div>
                    </div>
                )}

                <div className="saved-games">
                    <h2>Saved Games</h2>
                    {savedGames.map((game) => (
                        <div key={game.game_id} className="saved-game-item">
                            <div className="game-info">
                                <span>{game.settings.display_name}</span>
                                <span>Last played: {new Date(game.last_played).toLocaleDateString()}</span>
                            </div>
                            <div className="game-actions">
                                <button onClick={() => handleLoadGame(game.game_id)}>Load</button>
                                <button onClick={() => handleDeleteGame(game.game_id)}>Delete</button>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </>
    );
}; 