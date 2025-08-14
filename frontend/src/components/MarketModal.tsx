import React, { useEffect, useRef, useState } from 'react';
import { Planet, Resource, Player, Market, Fleet } from '../types/game';
import { api } from '../services/api';
import './MarketModal.css';
import { ApiResponse } from '../types/api';

interface MarketModalProps {
    isOpen: boolean;
    onClose: () => void;
    systemId: number;
    planetId: number;
    planet: Planet;
    selectedFleet?: Fleet;
    embedded?: boolean;
    onPlayerRefresh?: (player: Player) => void;
    showHeaderCredits?: boolean;
}

export const MarketModal: React.FC<MarketModalProps> = ({ isOpen, onClose, systemId, planetId, planet, selectedFleet, embedded = false, onPlayerRefresh, showHeaderCredits = true }) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [market, setMarket] = useState<Market | null>(null);
    const [player, setPlayer] = useState<Player | null>(null);
    const [fleets, setFleets] = useState<Fleet[]>([]);
    const [selectedFleetName, setSelectedFleetName] = useState<string | null>(selectedFleet?.name || null);
    const [selectedShipIndex, setSelectedShipIndex] = useState<number>(-1); // -1 => all ships in fleet
    const [selectedResource, setSelectedResource] = useState<Resource | null>(null);
    const [tradeAmount, setTradeAmount] = useState<number>(1);
    const [tradeMessage, setTradeMessage] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [distMode, setDistMode] = useState<'first' | 'even' | 'specific'>('first');
    const [resolvedIds, setResolvedIds] = useState<{ systemId: number; planetId: number } | null>(null);
    const [displayPlanet, setDisplayPlanet] = useState<Planet | null>(planet);
    const latestLoadRef = React.useRef(0);

    const resolvePlanetIndex = React.useCallback((sys: any, localPos: { x: number; y: number; z: number }) => {
        // Exact match first
        let idx = sys.planets.findIndex((p: any) => p.position.x === localPos.x && p.position.y === localPos.y && p.position.z === localPos.z);
        if (idx >= 0) return idx;
        // Fallback: nearest planet by distance
        let bestIdx = -1;
        let bestD = Number.POSITIVE_INFINITY;
        for (let i = 0; i < sys.planets.length; i++) {
            const p = sys.planets[i].position;
            const dx = p.x - localPos.x, dy = p.y - localPos.y, dz = p.z - localPos.z;
            const d = Math.sqrt(dx*dx + dy*dy + dz*dz);
            if (d < bestD) { bestD = d; bestIdx = i; }
        }
        return bestIdx;
    }, []);

    const getActiveFleet = React.useCallback(() => {
        if (selectedFleetName) {
            const f = fleets.find(fl => fl.name === selectedFleetName);
            if (f) return f;
        }
        return selectedFleet || null;
    }, [fleets, selectedFleetName, selectedFleet]);

    useEffect(() => {
        const loadData = async () => {
            const runId = ++latestLoadRef.current;
            try {
                setLoading(true);
                setError(null);
                
                // Load settings first to get player name
                const settings = await api.getGameSettings();

                // Load fleets first to ensure freshest location for selected fleet
                const fleetsResponse = await api.getPlayerFleets(settings.player_name);
                if (fleetsResponse.success && fleetsResponse.data) {
                    if (latestLoadRef.current !== runId) return;
                    setFleets(fleetsResponse.data);
                    const preferred = selectedFleet?.name || selectedFleetName || fleetsResponse.data[0]?.name || null;
                    if (preferred && preferred !== selectedFleetName) {
                        setSelectedFleetName(preferred);
                    }
                }

                // Resolve system/planet from freshest fleet data (prefer dropdown selection, then prop, then first fleet)
                let resolvedSystemId = systemId;
                let resolvedPlanetId = planetId;
                try {
                    const fleetRef = (fleetsResponse.success && fleetsResponse.data)
                        ? (fleetsResponse.data.find(f => f.name === (selectedFleetName || selectedFleet?.name)) || fleetsResponse.data[0])
                        : selectedFleet || null;
                    if (fleetRef && fleetRef.current_system_id != null) {
                        resolvedSystemId = fleetRef.current_system_id;
                        if (fleetRef.local_position) {
                            const sys = await api.getStarSystem(resolvedSystemId);
                            const idx = resolvePlanetIndex(sys, fleetRef.local_position!);
                            if (idx >= 0) {
                                resolvedPlanetId = idx;
                                if (latestLoadRef.current !== runId) return;
                                setDisplayPlanet(sys.planets[idx]);
                                // Prefer fleet that is co-located with resolved planet
                                if (fleetsResponse.success && fleetsResponse.data) {
                                    const coloc = fleetsResponse.data.find(f => f.current_system_id === resolvedSystemId && f.local_position && f.local_position.x === sys.planets[idx].position.x && f.local_position.y === sys.planets[idx].position.y && f.local_position.z === sys.planets[idx].position.z);
                                    if (coloc && coloc.name !== selectedFleetName) {
                                        setSelectedFleetName(coloc.name);
                                    }
                                }
                            }
                        }
                    }
                } catch (e) { console.warn('[MarketModal] resolve fleet->planet failed, using props', e); }

                // Load market and player using resolved IDs
                const [marketResponse, playerResponse] = await Promise.all([
                    api.getPlanetMarket(resolvedSystemId, resolvedPlanetId),
                    api.getPlayer(settings.player_name)
                ]);

                if (latestLoadRef.current !== runId) return;
                setMarket(marketResponse);
                setPlayer(playerResponse);
                console.log('[MarketModal] initial load:', {
                    systemId: resolvedSystemId,
                    planetId: resolvedPlanetId,
                    planetName: (displayPlanet?.name || planet.name),
                    playerCredits: playerResponse?.credits,
                    fleetsCount: fleetsResponse?.data?.length
                });
                setResolvedIds({ systemId: resolvedSystemId, planetId: resolvedPlanetId });
            } catch (err) {
                console.error('Error loading market data:', err);
                setError(err instanceof Error ? err.message : 'Failed to load market data');
            } finally {
                if (latestLoadRef.current === runId) setLoading(false);
            }
        };

        if (isOpen) {
            loadData();
        }

        const onFleetMoved = async (ev: Event) => {
            const ce = ev as CustomEvent<any>;
            const resp = ce?.detail?.data;
            if (!isOpen || !resp) {
                // fallback to full reload
                if (isOpen) loadData();
                return;
            }
            try {
                // Trust backend's IDs and local positions
                const newSystemId = resp.current_system_id as number | undefined;
                const localPos = resp.local_current_position;
                if (newSystemId == null || !localPos) { loadData(); return; }
                // Resolve planet by exact local coords
                        const sys = await api.getStarSystem(newSystemId);
                        const idx = resolvePlanetIndex(sys, localPos);
                if (idx >= 0) {
                    setResolvedIds({ systemId: newSystemId, planetId: idx });
                    setDisplayPlanet(sys.planets[idx]);
                    // Load market/player for resolved target
                    const settings = await api.getGameSettings();
                    const [mkt, ply] = await Promise.all([
                        api.getPlanetMarket(newSystemId, idx),
                        api.getPlayer(settings.player_name)
                    ]);
                    setMarket(mkt);
                    setPlayer(ply);
                } else {
                    // Fallback
                    loadData();
                }
            } catch {
                loadData();
            }
        };
        window.addEventListener('fleetMoved', onFleetMoved as EventListener);
        return () => window.removeEventListener('fleetMoved', onFleetMoved as EventListener);
    }, [systemId, planetId, isOpen, selectedFleetName, selectedFleet?.name, selectedFleet?.current_system_id, selectedFleet?.local_position?.x, selectedFleet?.local_position?.y, selectedFleet?.local_position?.z]);

    const refreshAfterTrade = async () => {
        try {
            const settings = await api.getGameSettings();
            const fleetsResponse = await api.getPlayerFleets(settings.player_name);
            // Re-resolve system/planet using freshest fleet data (prefer dropdown selection)
            let resolvedSystemId = systemId;
            let resolvedPlanetId = planetId;
            try {
                const fleetRef = (fleetsResponse.success && fleetsResponse.data)
                    ? (fleetsResponse.data.find(f => f.name === (selectedFleetName || selectedFleet?.name)) || fleetsResponse.data[0])
                    : selectedFleet || null;
                if (fleetRef && fleetRef.current_system_id != null) {
                    resolvedSystemId = fleetRef.current_system_id;
                    if (fleetRef.local_position) {
                        const sys = await api.getStarSystem(resolvedSystemId);
                        const idx = resolvePlanetIndex(sys, fleetRef.local_position!);
                        if (idx >= 0) {
                            resolvedPlanetId = idx;
                            setDisplayPlanet(sys.planets[idx]);
                        }
                    }
                }
            } catch (e) { console.warn('[MarketModal] resolve fleet->planet on refresh failed', e); }
            const [playerResponse, marketResponse] = await Promise.all([
                api.getPlayer(settings.player_name),
                api.getPlanetMarket(resolvedSystemId, resolvedPlanetId)
            ]);
            setPlayer(playerResponse);
            if (onPlayerRefresh) onPlayerRefresh(playerResponse);
            // Broadcast to any global header listeners as a safety net
            try { window.dispatchEvent(new CustomEvent('creditsUpdated', { detail: playerResponse.credits })); } catch (_) {}
            console.log('[MarketModal] refreshAfterTrade → credits:', playerResponse.credits);
            if (fleetsResponse.success && fleetsResponse.data) {
                setFleets(fleetsResponse.data);
                const preferred = selectedFleet?.name || selectedFleetName || fleetsResponse.data[0]?.name || null;
                if (preferred && preferred !== selectedFleetName) {
                    setSelectedFleetName(preferred);
                }
                console.log('[MarketModal] refreshAfterTrade → fleets reloaded:', fleetsResponse.data.map(f=>({
                    name: f.name,
                    current_system_id: f.current_system_id,
                    position: f.position,
                    local_position: f.local_position
                })));
            }
            setMarket(marketResponse);
            setResolvedIds({ systemId: resolvedSystemId, planetId: resolvedPlanetId });
        } catch (e) {
            // non-fatal
            console.warn('[MarketModal] refreshAfterTrade failed to refresh fully:', e);
        }
    };

    const handleBuy = async () => {
        if (!selectedResource || !market) return;
        const fleet = getActiveFleet();
        const ids = resolvedIds || { systemId, planetId };
        // Only allow trade when fleet present at same planet
        if (!fleet || fleet.current_system_id !== ids.systemId) {
            setTradeMessage('Fleet must be in this system to trade');
            return;
        }
        console.log('[MarketModal] buy check positions (strict local_position):', {
            fleet: fleet.name,
            current_system_id: fleet.current_system_id,
            local_position: fleet.local_position,
            planet_local: (displayPlanet?.position || planet.position),
            systemId: ids.systemId,
            usingResolved: resolvedIds != null,
        });
        if (!fleet.local_position || !displayPlanet ||
            fleet.local_position.x !== displayPlanet.position.x ||
            fleet.local_position.y !== displayPlanet.position.y ||
            fleet.local_position.z !== displayPlanet.position.z) {
            setTradeMessage('Fleet must be at this planet to trade');
            return;
        }
        if (distMode === 'specific' && selectedShipIndex < 0) {
            setTradeMessage('Select a ship for Specific distribution');
            return;
        }
        try {
            setTradeMessage(null);
            const distributionMode = distMode;
            const allocations = distMode === 'specific' ? [{ ship_index: selectedShipIndex, quantity: tradeAmount }] : undefined;
            console.log('[MarketModal] BUY sending request', { systemId: ids.systemId, planetId: ids.planetId, resource: selectedResource.resource_type, tradeAmount, fleet: fleet.name, distributionMode, allocations });
            const response = await api.buyResource(
                ids.systemId,
                ids.planetId,
                selectedResource.resource_type,
                tradeAmount,
                fleet.name,
                distributionMode,
                allocations
            );
            console.log('[MarketModal] BUY response:', response);
            await refreshAfterTrade();
            setTradeMessage(response);
            // Keep the selection and only reset the trade amount
            setTradeAmount(1);
        } catch (err) {
            console.error('Buy error:', err);
            setTradeMessage(err instanceof Error ? err.message : 'Failed to buy resource');
        }
    };

    const handleSell = async () => {
        if (!selectedResource || !market) return;
        const fleet = getActiveFleet();
        const ids = resolvedIds || { systemId, planetId };
        if (!fleet || fleet.current_system_id !== ids.systemId) {
            setTradeMessage('Fleet must be in this system to trade');
            return;
        }
        console.log('[MarketModal] sell check positions (strict local_position):', {
            fleet: fleet.name,
            current_system_id: fleet.current_system_id,
            local_position: fleet.local_position,
            planet_local: (displayPlanet?.position || planet.position),
            systemId: ids.systemId,
            usingResolved: resolvedIds != null,
        });
        if (!fleet.local_position || !displayPlanet ||
            fleet.local_position.x !== displayPlanet.position.x ||
            fleet.local_position.y !== displayPlanet.position.y ||
            fleet.local_position.z !== displayPlanet.position.z) {
            setTradeMessage('Fleet must be at this planet to trade');
            return;
        }
        if (distMode === 'specific' && selectedShipIndex < 0) {
            setTradeMessage('Select a ship for Specific distribution');
            return;
        }
        try {
            setTradeMessage(null);
            const distributionMode = distMode;
            const allocations = distMode === 'specific' ? [{ ship_index: selectedShipIndex, quantity: tradeAmount }] : undefined;
            console.log('[MarketModal] SELL sending request', { systemId: ids.systemId, planetId: ids.planetId, resource: selectedResource.resource_type, tradeAmount, fleet: fleet.name, distributionMode, allocations });
            const response = await api.sellResource(
                ids.systemId,
                ids.planetId,
                selectedResource.resource_type,
                tradeAmount,
                fleet.name,
                distributionMode,
                allocations
            );
            console.log('[MarketModal] SELL response:', response);
            await refreshAfterTrade();
            setTradeMessage(response);
            // Keep the selection and only reset the trade amount
            setTradeAmount(1);
        } catch (err) {
            console.error('Sell error:', err);
            setTradeMessage(err instanceof Error ? err.message : 'Failed to sell resource');
        }
    };

    // Log selector changes to trace unexpected behavior
    React.useEffect(()=>{
        console.log('[MarketModal] selection changed:', { selectedFleetName, selectedShipIndex, distMode });
    }, [selectedFleetName, selectedShipIndex, distMode]);

    const handleOverlayClick = (e: React.MouseEvent) => {
        if (e.target === modalRef.current?.parentElement) {
            onClose();
        }
    };

    const handleModalClick = (e: React.MouseEvent) => {
        e.stopPropagation();
    };

    const calculateTotalPrice = (price: number | null | undefined, amount: number) => {
        return price ? (price * amount).toFixed(2) : 'N/A';
    };

    if (!isOpen && !embedded) return null;
    if (loading) return embedded ? <div className="market-modal embedded">Loading market...</div> : <div className="modal-overlay"><div className="market-modal">Loading market...</div></div>;
    if (error) return embedded ? <div className="market-modal embedded error">{error}</div> : <div className="modal-overlay"><div className="market-modal error">{error}</div></div>;
    if (!market) return embedded ? <div className="market-modal embedded">No market available</div> : <div className="modal-overlay"><div className="market-modal">No market available</div></div>;

    const content = (
            <div className={`market-modal ${embedded ? 'embedded' : ''}`} ref={modalRef} onClick={embedded ? undefined : handleModalClick}>
                <div className="market-modal-header">
                    <h2>{(displayPlanet?.name || planet.name)} Market</h2>
                    <div className="market-info">
                        <span className="market-spec">
                            <strong>Specialization:</strong> {displayPlanet?.specialization || planet.specialization}
                        </span>
                        <span className="market-econ">
                            <strong>Economy:</strong> {displayPlanet?.economy || planet.economy}
                        </span>
                        <span className="planet-coords">
                            <strong>Local:</strong> (
                                {(displayPlanet?.position?.x ?? planet.position.x)}, {(displayPlanet?.position?.y ?? planet.position.y)}, {(displayPlanet?.position?.z ?? planet.position.z)}
                            )
                        </span>
                        {showHeaderCredits && player && (
                            <span className="player-credits">
                                <strong>Credits:</strong> {player.credits.toLocaleString()} cr
                            </span>
                        )}
                    </div>
                    {!embedded && <button className="close-button" onClick={onClose}>&times;</button>}
                </div>

                <div className="market-sections">
                    <div className="market-section">
                        <h3>Available Resources</h3>
                        {fleets.length > 0 && (
                            <div style={{ display: 'flex', gap: 12, alignItems: 'center', marginBottom: 12 }}>
                                <div>
                                    <label style={{ marginRight: 6 }}>Fleet:</label>
                                    <select value={selectedFleetName || ''} onChange={(e) => { setSelectedFleetName(e.target.value || null); setSelectedShipIndex(-1); }}>
                                        {fleets.map(f => <option key={f.name} value={f.name}>{f.name}</option>)}
                                    </select>
                                </div>
                                {selectedFleetName && (
                                    <div>
                                        <label style={{ marginRight: 6 }}>Ship:</label>
                                        <select value={selectedShipIndex} onChange={(e) => setSelectedShipIndex(Number(e.target.value))}>
                                            <option value={-1}>All ships</option>
                                            {(fleets.find(f=>f.name===selectedFleetName)?.ships || []).map((s, idx) => (
                                                <option key={idx} value={idx}>{s.name || `Ship #${idx+1}`}</option>
                                            ))}
                                        </select>
                                    </div>
                                )}
                                <div>
                                    <label style={{ marginRight: 6 }}>Distribution:</label>
                                    <select value={distMode} onChange={(e)=> setDistMode(e.target.value as any)}>
                                        <option value="first">First ship</option>
                                        <option value="even">Even across ships</option>
                                        <option value="specific">Specific ship</option>
                                    </select>
                                </div>
                                {/* Capacity banner */}
                                {selectedFleetName && (() => {
                                    const fleet = fleets.find(f=>f.name===selectedFleetName);
                                    if (!fleet) return null;
                                    const current = selectedShipIndex>=0 
                                        ? (fleet.ships[selectedShipIndex]?.cargo.reduce((a,r)=>a+(r.quantity||0),0)||0)
                                        : fleet.ships.reduce((s,ship)=> s + ship.cargo.reduce((ss,r)=> ss+(r.quantity||0),0),0);
                                    const capacity = selectedShipIndex>=0 
                                        ? (()=>{ const size=fleet.ships[selectedShipIndex]?.size; return size==='Tiny'?100:size==='Small'?250:size==='Medium'?500:size==='Large'?1000:size==='Huge'?2500:5000; })()
                                        : fleet.ships.reduce((s,ship)=> s + (ship.size==='Tiny'?100:ship.size==='Small'?250:ship.size==='Medium'?500:ship.size==='Large'?1000:ship.size==='Huge'?2500:5000),0);
                                    const remaining = Math.max(0, capacity-current);
                                    return <div style={{ background:'rgba(0,255,0,0.1)', border:'1px solid rgba(0,255,0,0.3)', padding:'4px 8px', borderRadius:4 }}>Cargo: {current} / {capacity} (remaining {remaining})</div>;
                                })()}
                            </div>
                        )}
                        <table className="resources-grid">
                            <thead>
                                <tr>
                                    <th>Resource</th>
                                    <th>Available</th>
                                    <th>Buy Price (cr)</th>
                                    <th>Sell Price (cr)</th>
                                    <th>Your Stock</th>
                                    <th>Player Total</th>
                                </tr>
                            </thead>
                            <tbody>
                                {market.resources.map((resource) => {
                                    // Backend mirrors sum across fleets into player.resources; still fallback to fleet sum if undefined
                                    const fleetSum = fleets.reduce((sum, f) => sum + f.ships.reduce((ss, ship) => ss + (ship.cargo.find(c => c.resource_type === resource.resource_type)?.quantity || 0), 0), 0);
                                    const playerQuantity = (player?.resources.find(r => r.resource_type === resource.resource_type)?.quantity ?? fleetSum) || 0;
                                    // Fleet/Ship totals
                                    const fleet = selectedFleetName ? fleets.find(f=>f.name===selectedFleetName) : null;
                                    const yourStock = fleet ? (
                                        selectedShipIndex>=0
                                            ? (fleet.ships[selectedShipIndex]?.cargo.find(c=>c.resource_type===resource.resource_type)?.quantity || 0)
                                            : fleet.ships.reduce((s,ship)=> s + (ship.cargo.find(c=>c.resource_type===resource.resource_type)?.quantity || 0), 0)
                                    ) : 0;
                                    const isSelected = selectedResource?.resource_type === resource.resource_type;
                                    const isUpdated = isSelected && tradeMessage?.includes('Successfully');
                                    
                                    return (
                                        <tr 
                                            key={resource.resource_type} 
                                            className={`${isSelected ? 'selected' : ''} ${isUpdated ? 'updated' : ''}`}
                                            onClick={() => setSelectedResource(resource)}
                                        >
                                            <td>{resource.resource_type}</td>
                                            <td className={isUpdated ? 'updated-value' : ''}>{resource.quantity?.toLocaleString() || '0'}</td>
                                            <td>{resource.buy != null ? resource.buy.toFixed(3) : 'N/A'}</td>
                                            <td>{resource.sell != null ? resource.sell.toFixed(3) : 'N/A'}</td>
                                            <td className={isUpdated ? 'updated-value' : ''}>{yourStock.toLocaleString()}</td>
                                            <td>{playerQuantity.toLocaleString()}</td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>

                    {selectedResource ? (
                        <div className="trade-actions">
                            <h3>Trade Actions</h3>
                            <div className="trade-amount">
                                <label>Amount:</label>
                                <input
                                    type="number"
                                    min="1"
                                    value={tradeAmount}
                                    onChange={(e) => setTradeAmount(Math.max(1, parseInt(e.target.value) || 1))}
                                    className="amount-input"
                                />
                            </div>
                            <div className="trade-buttons">
                                {selectedResource.buy && (
                                    <button 
                                        className="buy-button"
                                        onClick={handleBuy}
                                        disabled={(() => { const fleet = getActiveFleet(); const ids = resolvedIds || { systemId, planetId }; const p = displayPlanet?.position || planet.position; return !fleet || fleet.current_system_id !== ids.systemId || !fleet.local_position || fleet.local_position.x !== p.x || fleet.local_position.y !== p.y || fleet.local_position.z !== p.z; })()}
                                    >
                                        Buy ({calculateTotalPrice(selectedResource.buy, tradeAmount)} cr)
                                    </button>
                                )}
                                {selectedResource.sell && (
                                    <button 
                                        className="sell-button"
                                        onClick={handleSell}
                                        disabled={(() => { const fleet = getActiveFleet(); const ids = resolvedIds || { systemId, planetId }; const p = displayPlanet?.position || planet.position; return !fleet || fleet.current_system_id !== ids.systemId || !fleet.local_position || fleet.local_position.x !== p.x || fleet.local_position.y !== p.y || fleet.local_position.z !== p.z; })()}
                                    >
                                        Sell ({calculateTotalPrice(selectedResource.sell, tradeAmount)} cr)
                                    </button>
                                )}
                            </div>
                            {tradeMessage && (
                                <div className={`trade-message ${tradeMessage.includes('Successfully') ? 'success' : 'error'}`}>
                                    {tradeMessage}
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="trade-actions">
                            <h3>Trade Actions</h3>
                            <div className="no-selection-message">
                                Select a resource to trade
                            </div>
                        </div>
                    )}
                </div>
            </div>
    );
    if (embedded) return content;
    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>{content}</div>
    );
}; 