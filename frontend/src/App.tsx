import React, { useState, useEffect } from 'react';
import { MainMenu } from './components/MainMenu';
import { GalaxyMap } from './components/GalaxyMap';
import MarketView from './components/Market';
import { FleetList } from './components/FleetList';
import { TabBar, TabType } from './components/TabBar';
import { Player, GameSettings, Fleet } from './types/game';
import { api } from './services/api';
import './App.css';

function App() {
    const [isInGame, setIsInGame] = useState(false);
    const [player, setPlayer] = useState<Player | null>(null);
    const [activeTab, setActiveTab] = useState<TabType>('galaxy');
  const [selectedFleet, setSelectedFleet] = useState<Fleet | null>(null);

    useEffect(() => {
        if (isInGame) {
            loadPlayerInfo();
        }
    }, [isInGame]);

    useEffect(() => {
        const onCreditsUpdated = async (e: Event) => {
            try {
                const settings = await api.getGameSettings();
                const refreshed = await api.getPlayer(settings.player_name);
                setPlayer(prev => prev ? { ...prev, credits: refreshed.credits, resources: refreshed.resources } : prev);
            } catch {
                // fallback to event detail if available
                const ce = e as CustomEvent<number>;
                setPlayer(prev => prev ? { ...prev, credits: typeof ce.detail === 'number' ? ce.detail : prev.credits } : prev);
            }
        };
        window.addEventListener('creditsUpdated', onCreditsUpdated as EventListener);
        return () => window.removeEventListener('creditsUpdated', onCreditsUpdated as EventListener);
    }, []);

  // Keep fleets/selectedFleet in sync after backend-confirmed moves
  useEffect(() => {
    const onFleetMoved = async (_e: Event) => {
      try {
        const settings = await api.getGameSettings();
        const fleetsResp = await api.getPlayerFleets(settings.player_name);
        if (fleetsResp.success) {
          // Update player fleets list for header/consumers
          setPlayer(prev => prev ? { ...prev, fleets: fleetsResp.data } : prev);
          // Preserve the same fleet selection by name if possible
          setSelectedFleet(prev => {
            const name = prev?.name;
            return fleetsResp.data.find(f => f.name === name) || fleetsResp.data[0] || null;
          });
        }
      } catch {
        // ignore
      }
    };
    window.addEventListener('fleetMoved', onFleetMoved as EventListener);
    return () => window.removeEventListener('fleetMoved', onFleetMoved as EventListener);
  }, []);

    const loadPlayerInfo = async () => {
        try {
            const settings = await api.getGameSettings();
            console.log('Loaded game settings:', settings);
            
            const [playerData, fleetsData] = await Promise.all([
                api.getPlayer(settings.player_name),
                api.getPlayerFleets()
            ]);
            console.log('Loaded player data:', playerData);
            console.log('Loaded fleets data:', fleetsData);
            
            setPlayer({
                name: settings.player_name,
                credits: playerData.credits,
                resources: playerData.resources,
                fleets: fleetsData.data || []
            });
        } catch (error) {
            console.error('Failed to load player info:', error);
            // Set a default player state with 0 credits if loading fails
            setPlayer({
                name: 'Unknown Player',
                credits: 0,
                resources: [],
                fleets: []
            });
        }
    };

    const handleReturnToMainMenu = async () => {
        try {
            // Only clear caches when explicitly returning to menu
            await api.clearCaches();
            setIsInGame(false);
            setPlayer(null);
        } catch (error) {
            console.error('Failed to clear caches:', error);
            // Still proceed with resetting the game state even if cache clearing fails
            setIsInGame(false);
            setPlayer(null);
        }
    };

    const renderContent = () => {
        if (!isInGame) {
            return <MainMenu onStartGame={() => setIsInGame(true)} />;
        }

        switch (activeTab) {
            case 'galaxy':
                return <GalaxyMap selectedFleet={selectedFleet} />;
            case 'fleets':
                return <FleetList onFleetSelected={setSelectedFleet} />;
            case 'market':
                return <MarketView selectedFleet={selectedFleet} />;
            case 'research':
                return <div>Research View (Coming Soon)</div>;
            default:
                return <GalaxyMap />;
        }
    };

    return (
        <div className="app">
            {isInGame && (
                <>
                    <header className="app-header">
                        <div className="header-left">
                            <button className="main-menu-button" onClick={handleReturnToMainMenu}>
                                Main Menu
                            </button>
                            <h1>Star Trader</h1>
                        </div>
                        {player && (
                            <div className="player-info">
                                <span>Player: {player.name}</span>
                                <span>Credits: {player.credits.toLocaleString()}</span>
                            </div>
                        )}
                    </header>
                    <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
                </>
            )}
            <main className="app-content">
                {renderContent()}
            </main>
        </div>
    );
}

export default App;
