            try {
                // Load player fleets first
                const settings = await api.getGameSettings(); // Need settings for player name
                const fleetsResponse = await api.getPlayerFleets(settings.player_name); // Pass player name
                if (fleetsResponse.success) {
// ... existing code ...
                }

                // Load player fleets again after trade?
                // Assuming settings is still in scope or re-fetch if necessary
                const fleetsResponseAfterTrade = await api.getPlayerFleets(settings.player_name); // Pass player name
                if (fleetsResponseAfterTrade.success) {
// ... existing code ...
                }
            } catch (error) {
                console.error('Error loading player fleets:', error);
            } 