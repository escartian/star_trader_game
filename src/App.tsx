const [playerData, fleetsData] = await Promise.all([
    api.getPlayer(settings.player_name),
    api.getPlayerFleets(settings.player_name)
]);