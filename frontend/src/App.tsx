import React, { useState, useEffect } from 'react';
import { GalaxyMap } from './components/GalaxyMap';
import { Player } from './types/game';
import { api } from './services/api';
import { FleetList } from './components/FleetList';
import './App.css';

function App() {
  const [player, setPlayer] = useState<Player | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchPlayer = async () => {
      try {
        const data = await api.getPlayer('Igor');
        setPlayer(data);
      } catch (err) {
        if (err instanceof Error) {
          if (err.message.includes('404')) {
            setError('Oops! Looks like we need to start a new game! 🚀');
          } else {
            setError('Whoops! Something went wrong. Let\'s try again! 🌟');
          }
        } else {
          setError('Oops! Something unexpected happened! 🌠');
        }
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    fetchPlayer();
  }, []);

  if (loading) return (
    <div className="loading">
      <div className="loading-spinner">🚀</div>
      <p>Loading your galactic adventure...</p>
    </div>
  );

  if (error) return (
    <div className="error-container">
      <div className="error-content">
        <div className="error-icon">🌌</div>
        <h2>{error}</h2>
        <p>Don't worry, we'll get you back on track!</p>
        <button onClick={() => window.location.reload()}>Try Again</button>
      </div>
    </div>
  );

  return (
    <div className="app">
      <header className="app-header">
        <h1>Star Trader Game</h1>
        {player && (
          <div className="player-info">
            <h2>Welcome, {player.name}! 👋</h2>
            <p>Credits: {player.credits.toLocaleString()} 💰</p>
          </div>
        )}
      </header>

      <main className="app-main">
        <div className="content-grid">
          <div className="galaxy-section">
            <GalaxyMap />
          </div>
          <div className="fleet-section">
            <FleetList />
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
