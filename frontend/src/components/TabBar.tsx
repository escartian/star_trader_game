import React from 'react';
import './TabBar.css';

export type TabType = 'galaxy' | 'fleets' | 'market' | 'research';

interface TabBarProps {
    activeTab: TabType;
    onTabChange: (tab: TabType) => void;
}

export const TabBar: React.FC<TabBarProps> = ({ activeTab, onTabChange }) => {
    const tabs: { id: TabType; label: string; icon: string }[] = [
        { id: 'galaxy', label: 'Galaxy', icon: '🌌' },
        { id: 'fleets', label: 'Fleets', icon: '🛸' },
        { id: 'market', label: 'Market', icon: '⚖' },
        { id: 'research', label: 'Research', icon: '⚛' }
    ];

    return (
        <nav className="tab-bar">
            {tabs.map(tab => (
                <button
                    key={tab.id}
                    className={`tab-button ${activeTab === tab.id ? 'active' : ''}`}
                    onClick={() => onTabChange(tab.id)}
                >
                    <span className="tab-icon">{tab.icon}</span>
                    <span className="tab-label">{tab.label}</span>
                </button>
            ))}
        </nav>
    );
}; 