import React, { useState, useEffect } from 'react';
import { GalaxyMap } from './components/GalaxyMap';
import { FleetList } from './components/FleetList';
import { TabBar, TabType } from './components/TabBar';
import { HOST_PLAYER_NAME } from './constants';
import { Player } from './types/game';
import { api } from './services/api';
import './App.css';

function App() {
    const [player, setPlayer] = useState<Player | null>(null);
    const [activeTab, setActiveTab] = useState<TabType>('galaxy');

    useEffect(() => {
        const loadPlayer = async () => {
            try {
                const playerData = await api.getPlayer(HOST_PLAYER_NAME);
                setPlayer(playerData);
            } catch (err) {
                console.error('Error loading player:', err);
            }
        };
        loadPlayer();
    }, []);

    const renderContent = () => {
        switch (activeTab) {
            case 'galaxy':
                return <GalaxyMap />;
            case 'fleets':
                return <FleetList />;
            case 'market':
                return <div>Market View (Coming Soon)</div>;
            case 'research':
                return <div>Research View (Coming Soon)</div>;
            default:
                return <GalaxyMap />;
        }
    };

    return (
        <div className="app">
            <header className="app-header">
                <h1>Star Trader</h1>
                {player && (
                    <div className="player-info">
                        <span>Player: {player.name}</span>
                        <span>Credits: {player.credits.toLocaleString()}</span>
                    </div>
                )}
            </header>
            <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
            <main className="app-content">
                {renderContent()}
            </main>
        </div>
    );
}

export default App;
