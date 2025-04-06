import React, { useState } from 'react';
import './TabBar.css';

export type TabType = 'galaxy' | 'fleets' | 'market' | 'research';

interface TabBarProps {
    activeTab: TabType;
    onTabChange: (tab: TabType) => void;
    onVisibilityChange?: (isVisible: boolean) => void;
}

export const TabBar: React.FC<TabBarProps> = ({ activeTab, onTabChange, onVisibilityChange }) => {
    const [isVisible, setIsVisible] = useState(true);

    const tabs: { id: TabType; label: string; icon: string }[] = [
        { id: 'galaxy', label: 'Galaxy', icon: '🌌' },
        { id: 'fleets', label: 'Fleets', icon: '🛸' },
        { id: 'market', label: 'Market', icon: '⚖' },
        { id: 'research', label: 'Research', icon: '⚛' }
    ];

    const toggleVisibility = () => {
        const newVisibility = !isVisible;
        setIsVisible(newVisibility);
        if (onVisibilityChange) {
            onVisibilityChange(newVisibility);
        }
    };

    return (
        <div className={`tab-container ${!isVisible ? 'hidden' : ''}`}>
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
            <button 
                className="toggle-button"
                onClick={toggleVisibility}
                title={isVisible ? "Hide Menu" : "Show Menu"}
            >
                {isVisible ? '▲' : '▼'}
            </button>
        </div>
    );
}; 