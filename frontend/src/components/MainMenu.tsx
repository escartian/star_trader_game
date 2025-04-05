import React, { useState, useEffect } from 'react';
import { api } from '../services/api';
import { GameSettings, SavedGame, Faction } from '../types/game';
import './MainMenu.css';

interface MainMenuProps {
    onStartGame: () => void;
}

export const MainMenu: React.FC<MainMenuProps> = ({ onStartGame }) => {
    const [savedGames, setSavedGames] = useState<SavedGame[]>([]);
    const [showNewGame, setShowNewGame] = useState(false);
    const [showAdvancedSettings, setShowAdvancedSettings] = useState(false);
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
            { name: 'Federation', influence: 50, prefix: 'The' },
            { name: 'Empire', influence: 50, prefix: 'The' },
            { name: 'Republic', influence: 50, prefix: 'The' },
            { name: 'Alliance', influence: 50, prefix: 'The' }
        ],
        created_at: new Date().toISOString(),
        last_played: new Date().toISOString()
    });

    useEffect(() => {
        loadSavedGames();
    }, []);

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
            factions: [...settings.factions, { name: 'New Faction', influence: 50, prefix: 'The' }]
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
        <div className="main-menu">
            <h1>Star Trader</h1>
            
            <div className="menu-buttons">
                <button onClick={() => setShowNewGame(true)}>New Game</button>
                {savedGames.length > 0 && (
                    <button onClick={() => setShowNewGame(false)}>Load Game</button>
                )}
            </div>

            {showNewGame ? (
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
                                {settings.factions.map((faction, index) => (
                                    <div key={index} className="faction-settings">
                                        <div className="setting-group">
                                            <label>Faction Name:</label>
                                            <input
                                                type="text"
                                                value={faction.name}
                                                onChange={(e) => updateFaction(index, 'name', e.target.value)}
                                            />
                                        </div>
                                        <div className="setting-group">
                                            <label>Influence:</label>
                                            <input
                                                type="number"
                                                value={faction.influence}
                                                onChange={(e) => updateFaction(index, 'influence', parseInt(e.target.value))}
                                                min="0"
                                                max="100"
                                            />
                                        </div>
                                        <div className="setting-group">
                                            <label>Prefix:</label>
                                            <input
                                                type="text"
                                                value={faction.prefix}
                                                onChange={(e) => updateFaction(index, 'prefix', e.target.value)}
                                            />
                                        </div>
                                        <button onClick={() => removeFaction(index)}>Remove Faction</button>
                                    </div>
                                ))}
                                <button onClick={addFaction}>Add Faction</button>
                            </div>
                        )}

                        <button onClick={handleNewGame}>Start Game</button>
                    </div>
                </div>
            ) : (
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
            )}
        </div>
    );
}; 